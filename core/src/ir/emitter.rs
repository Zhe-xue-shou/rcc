#![allow(unused)]
#![deny(unused_must_use)]
#![allow(clippy::needless_pass_by_ref_mut)]

use ::rcc_utils::contract_violation;
use ::std::collections::HashMap;

use super::{
  Argument,
  emitable::Emitable,
  instruction::{self as inst},
  module::{self, BasicBlock, Module},
  value::{Value, ValueID},
};
use crate::{
  common::{Integral, Operator, OperatorCategory, RefEq, SourceSpan, StrRef},
  ir,
  sema::{declaration as sd, expression as se, statement as ss},
  session::Session,
  types::{CastType, Constant, QualifiedType, TypeInfo},
};
pub struct Emitter<'source, 'context, 'session>
where
  'context: 'session,
  'source: 'context,
{
  pub(super) session: &'session Session<'source, 'context>,
  /// The basic block currently being written into
  pub(super) current_block: Option<BasicBlock>,
  /// Blocks finalized in the current function
  pub(super) current_blocks: Vec<ValueID>,
  pub(super) locals: HashMap<StrRef<'context>, ValueID>,
  /// function name → ValueID for call resolution
  pub(super) functions: HashMap<StrRef<'context>, ValueID>,
  pub(super) module: Module,
}
#[macro_use]
mod macros {
  macro_rules! ty {
    ($self:ident, $qualified_type:expr) => {
      $self.session.ir_context.ir_type(&$qualified_type)
    };
  }
  macro_rules! ctx {
    ($self:ident) => {
      $self.session().ir_context
    };
  }
  macro_rules! lookup {
    ($self:ident, $value_id:expr) => {
      ctx!($self).get($value_id)
    };
  }
  macro_rules! lookup_mut {
    ($self:ident, $value_id:expr) => {
      ctx!($self).get_mut($value_id)
    };
  }
}
impl<'source, 'context, 'session> Emitter<'source, 'context, 'session> {
  pub fn new(session: &'session Session<'source, 'context>) -> Self {
    Self {
      session,
      current_block: Default::default(),
      current_blocks: Default::default(),
      locals: Default::default(),
      functions: Default::default(),
      module: Default::default(),
    }
  }

  #[inline(always)]
  pub(super) fn session(&self) -> &'session Session<'source, 'context> {
    self.session
  }
}
impl<'context> Emitter<'_, 'context, '_> {
  fn push_block(&mut self) {
    self.seal_current_block();
    self.current_block = Some(Default::default());
  }

  fn seal_current_block(&mut self) {
    if let Some(block) = self.current_block.take() {
      let block_id = ctx!(self).insert(Value::new(
        self.session.ast_context.void_type().into(),
        ctx!(self).label_type(),
        block.into(),
      ));
      self.current_blocks.push(block_id);
    }
    // do nothing
  }

  fn is_global(&self) -> bool {
    self.current_block.is_none()
  }
}

impl<'context> Emitter<'_, 'context, '_> {
  pub fn build(
    mut self,
    translation_unit: sd::TranslationUnit<'context>,
  ) -> Module {
    let declarations = translation_unit.declarations;

    self.module.globals = Vec::with_capacity(declarations.len());

    declarations
      .into_iter()
      .for_each(|declaration| match declaration {
        sd::ExternalDeclaration::Function(function) => {
          if function.is_definition() {
            self.global_funcdef(function);
          } else {
            self.global_funcdecl(function);
          }
        },
        sd::ExternalDeclaration::Variable(variable) => {
          let qualified_type = variable.symbol.borrow().qualified_type;
          let variable = self.global_vardef(variable);
          let _value_id = self.emit(variable, qualified_type);
        },
      });
    self.module
  }
}

impl<'context> Emitter<'_, 'context, '_> {
  fn global_funcdecl(&mut self, function: sd::Function<'context>) -> ValueID {
    if let Some(&value_id) = self.functions.get(function.symbol.borrow().name) {
      debug_assert!(
        lookup!(self, value_id).data.is_function(),
        "pre-registered value should be a function"
      );
      value_id
    } else {
      let sym = function.symbol.borrow();
      let name = sym.name;
      let qualified_type = sym.qualified_type;
      let is_variadic = qualified_type.as_functionproto_unchecked().is_variadic;
      drop(sym);

      let value_id = self.emit(
        module::Function::new_empty(name, Default::default(), is_variadic),
        qualified_type,
      );

      self.functions.insert(name, value_id);
      value_id
    }
  }

  fn global_funcdef(&mut self, function: sd::Function<'context>) {
    assert!(function.is_definition());

    let function_id = if let Some(&value_id) =
      self.functions.get(function.symbol.borrow().name)
    {
      // should be function and declaration-only
      debug_assert!(
        !lookup!(self, value_id).data.as_function().is_some_and(|f| f
          .is_definition()
          && RefEq::ref_eq(
            function.symbol.borrow().name,
            lookup!(self, value_id).data.as_function_unchecked().name
          )
          && f.is_variadic
            == function
              .symbol
              .borrow()
              .qualified_type
              .as_functionproto_unchecked()
              .is_variadic),
        "pre-registered function should be declaration-only"
      );
      value_id
    } else {
      let sym = function.symbol.borrow();
      let name = sym.name;
      let qualified_type = sym.qualified_type;
      let is_variadic = qualified_type.as_functionproto_unchecked().is_variadic;
      drop(sym);

      let function_id = self.emit(
        module::Function::new_empty(name, Default::default(), is_variadic),
        qualified_type,
      );

      self.functions.insert(name, function_id);
      debug_assert!(
        lookup!(self, function_id)
          .data
          .as_function()
          .is_some_and(|f| !f.is_definition()),
        "pre-registered function should be declaration-only"
      );
      function_id
    };

    let sd::Function {
      parameters, body, ..
    } = function;
    self.locals.clear();
    self.current_blocks = Default::default();

    self.push_block();
    let params = {
      let return_type = lookup!(self, function_id)
        .qualified_type
        .as_functionproto_unchecked()
        .return_type;

      // return value storage
      let _return_slot_id = self.emit(inst::Alloca::new(), return_type);
      // _ = self.emit(
      //   inst::Memory::Store(inst::Store::new(
      //     _return_slot_id,
      //     ???
      //   )),
      //   return_type,
      // );

      // insert params into the local scope and allocate spaces
      parameters
        .into_iter()
        .enumerate()
        .map(|(index, parameter)| {
          let qualified_type = parameter.symbol.borrow().qualified_type;
          let arg_id =
            self.emit(Argument::new(function_id, index), qualified_type);
          self.locals.insert(parameter.symbol.borrow().name, arg_id);
          let localed_arg_id = self.emit(inst::Alloca::new(), qualified_type);
          _ = self.emit(
            inst::Memory::Store(inst::Store::new(localed_arg_id, arg_id)),
            qualified_type,
          );
          arg_id
        })
        .collect::<Vec<_>>()
    };

    self.raw_compound(body.expect("Precondition: function.is_definition()"));
    self.seal_current_block();

    let mut refmut = lookup_mut!(self, function_id);
    let mutref = refmut.data.as_function_mut_unchecked();
    mutref.params = params;
    mutref.blocks = ::std::mem::take(&mut self.current_blocks);

    self.locals.clear();
  }

  fn global_vardef(
    &self,
    variable: sd::VarDef<'context>,
  ) -> module::Variable<'context> {
    let sym = variable.symbol.borrow();
    module::Variable::new(
      sym.name, None, // TODO: handle initializers
    )
  }

  fn local_funcdecl(
    &mut self,
    external_declaration: sd::ExternalDeclaration<'context>,
  ) {
    debug_assert!(!self.is_global());
    match external_declaration {
      sd::ExternalDeclaration::Function(function) => {
        debug_assert!(function.is_declaration());
        self.global_funcdecl(function);
      },
      sd::ExternalDeclaration::Variable(var_def) => self.local_vardef(var_def),
    }
  }

  fn local_vardef(&mut self, var_def: sd::VarDef<'context>) {
    use inst::{Alloca, Memory, Store};
    let sym = var_def.symbol.borrow();
    let var_type = ty!(self, sym.qualified_type);
    let alloca = (Alloca::new());
    let value_id = self.emit(alloca, sym.qualified_type);
    self.locals.insert(sym.name, value_id);
  }
}

impl<'context> Emitter<'_, 'context, '_> {
  fn statement(&mut self, statement: ss::Statement<'context>) {
    use ss::Statement::*;
    match statement {
      Empty(_) => (),
      Return(return_stmt) => self.returnstmt(return_stmt),
      Expression(expression) => self.exprstmt(expression),
      Declaration(external_declaration) =>
        self.local_funcdecl(external_declaration),
      Compound(compound) => self.compound(compound),
      If(if_stmt) => todo!(),
      While(while_stmt) => todo!(),
      DoWhile(do_while) => todo!(),
      For(for_stmt) => todo!(),
      Switch(switch) => todo!(),
      Goto(goto) => todo!(),
      Label(label) => todo!(),
      Break(break_stmt) => todo!(),
      Continue(continue_stmt) => todo!(),
    }
  }

  #[inline]
  fn exprstmt(&mut self, expression: se::Expression<'context>) {
    self.expression(expression);
  }

  fn returnstmt(&mut self, return_stmt: ss::Return<'context>) {
    let ss::Return { expression, .. } = return_stmt;
    let qualified_type = expression
      .as_ref()
      .map(|e| *e.qualified_type())
      .unwrap_or(self.session.ast_context.void_type().into());
    let operand = expression.map(|e| self.expression(e));
    _ = self.emit(
      inst::Terminator::Return(inst::Return::new(operand)),
      qualified_type,
    );
  }

  fn compound(&mut self, body: ss::Compound<'context>) {
    self.push_block();
    self.raw_compound(body);
    // self.seal_current_block();???
  }

  fn raw_compound(&mut self, body: ss::Compound<'context>) {
    let ss::Compound { statements, .. } = body;
    statements
      .into_iter()
      .for_each(|statement| self.statement(statement));
  }
}
#[allow(clippy::needless_pass_by_ref_mut)]
impl<'context> Emitter<'_, 'context, '_> {
  fn expression(&mut self, expression: se::Expression<'context>) -> ValueID {
    // the fold here contains partial fold. e.g. `3 + 6 + func(4 + 5)` would be folded to `9 + func(9)`.
    let (raw_expr, qualified_type, ..) = expression
      .fold(&self.session.diagnosis)
      .take()
      .destructure();
    use se::RawExpr::*;
    match raw_expr {
      Empty(_) => contract_violation!(
        "empty expr is used in sema for error recovery. shouldnt reach here."
      ),
      Constant(constant) => self.constant(constant, qualified_type),
      Unary(unary) => self.unary(unary, qualified_type),
      Binary(binary) => self.binary(binary, qualified_type),
      Call(call) => self.call(call, qualified_type),
      Paren(paren) => self.paren(paren),
      MemberAccess(member_access) =>
        self.member_access(member_access, qualified_type),
      Ternary(ternary) => self.ternary(ternary, qualified_type),
      SizeOf(size_of) => self.sizeof(size_of, qualified_type),
      CStyleCast(cstyle_cast) => self.cstyle_cast(cstyle_cast, qualified_type),
      ArraySubscript(array_subscript) =>
        self.array_subscript(array_subscript, qualified_type),
      CompoundLiteral(compound_literal) =>
        self.compound_literal(compound_literal, qualified_type),
      Variable(variable) => self.variable(variable, qualified_type),
      ImplicitCast(implicit_cast) =>
        self.implicit_cast(implicit_cast, qualified_type),
    }
  }

  fn constant(
    &mut self,
    constant: se::Constant<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    self.emit(constant.value, qualified_type)
  }

  fn unary(
    &mut self,
    unary: se::Unary<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let se::Unary {
      kind,
      operand,
      operator,
      span,
    } = unary;
    let operand = self.expression(*operand);
    match operator {
      Operator::Ampersand =>
        self.addressof(operator, operand, qualified_type, span),
      Operator::Star => self.indirect(operator, operand, qualified_type, span),
      Operator::Not =>
        self.logical_not(operator, operand, qualified_type, span),
      Operator::Tilde => self.tilde(operator, operand, qualified_type, span),
      Operator::Plus | Operator::Minus =>
        self.unary_arithmetic(operator, operand, qualified_type, span),
      Operator::PlusPlus | Operator::MinusMinus =>
        self.ppmm(operator, operand, kind, qualified_type, span),
      _ => unreachable!("operator is not unary: {:#?}", operator),
    }
  }

  fn binary(
    &mut self,
    binary: se::Binary<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let se::Binary {
      left,
      operator,
      right,
      span,
    } = binary;

    let left = self.expression(*left);
    let right = self.expression(*right);

    use OperatorCategory::*;
    match operator.category() {
      Assignment =>
        self.assignment(operator, left, right, qualified_type, span),
      Logical => self.logical(operator, left, right, qualified_type, span),
      Relational =>
        self.relational(operator, left, right, qualified_type, span),
      Arithmetic =>
        self.arithmetic(operator, left, right, qualified_type, span),
      Bitwise => self.bitwise(operator, left, right, qualified_type, span),
      BitShift => self.bitshift(operator, left, right, qualified_type, span),
      Special => self.comma(operator, left, right, qualified_type, span),
      Uncategorized => unreachable!("operator is not binary: {:#?}", operator),
    }
  }

  fn member_access(
    &mut self,
    member_access: se::MemberAccess<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    todo!()
  }

  fn ternary(
    &mut self,
    ternary: se::Ternary<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let se::Ternary {
      condition,
      then_expr,
      else_expr,
      ..
    } = ternary;
    todo!()
  }

  fn sizeof(
    &mut self,
    size_of: se::SizeOf<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    todo!()
  }

  fn cstyle_cast(
    &mut self,
    cstyle_cast: se::CStyleCast<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    todo!()
  }

  fn array_subscript(
    &mut self,
    array_subscript: se::ArraySubscript<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    todo!()
  }

  fn compound_literal(
    &mut self,
    compound_literal: se::CompoundLiteral,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    todo!()
  }

  fn variable(
    &self,
    variable: se::Variable<'context>,
    _qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let name = variable.name.borrow().name;
    if let Some(&vid) = self.locals.get(name) {
      vid
    } else if let Some(&vid) = self.functions.get(name) {
      vid
    } else {
      panic!("undefined variable: {name}")
    }
  }

  fn implicit_cast(
    &mut self,
    implicit_cast: se::ImplicitCast<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let se::ImplicitCast {
      cast_type, expr, ..
    } = implicit_cast;

    let value_id = self.expression(*expr);
    match cast_type {
      CastType::FunctionToPointerDecay
      | CastType::ArrayToPointerDecay
      | CastType::Noop => value_id,
      CastType::LValueToRValue => self.emit(
        inst::Memory::from(inst::Load::new(value_id)),
        qualified_type,
      ),
      // todo: int <-> int: transparent for now (same-width or promotion)
      CastType::IntegralCast => {
        assert!(
          qualified_type
            .as_primitive()
            .is_some_and(|p| p.is_integer())
            && qualified_type.size_bits() > 0
            && qualified_type.size_bits() <= 128
        );
        match Ord::cmp(
          lookup!(self, value_id).ir_type.as_integer_unchecked(),
          &(qualified_type.size_bits() as u8),
        ) {
          ::std::cmp::Ordering::Less => match qualified_type.is_signed() {
            true => self.emit(
              inst::Cast::from(inst::Sext::new(value_id)),
              qualified_type,
            ),
            false => self.emit(
              inst::Cast::from(inst::Zext::new(value_id)),
              qualified_type,
            ),
          },
          ::std::cmp::Ordering::Equal => value_id,
          ::std::cmp::Ordering::Greater => self
            .emit(inst::Cast::from(inst::Trunc::new(value_id)), qualified_type),
        }
      },
      _ => todo!("implicit cast: {:?}", implicit_cast.cast_type),
    }
  }

  fn call(
    &mut self,
    call: se::Call<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let se::Call {
      callee, arguments, ..
    } = call;

    let callee = self.expression(*callee);

    let args: Vec<ValueID> = arguments
      .into_iter()
      .map(|arg| self.expression(arg))
      .collect();

    let call_inst = inst::Call::new(callee, args);

    self.emit(call_inst, qualified_type)
  }

  #[inline]
  fn paren(&mut self, paren: se::Paren<'context>) -> ValueID {
    self.expression(*paren.expr)
  }
}
impl<'context> Emitter<'_, 'context, '_> {
  fn assignment(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    assert!(
      operator == Operator::Assign,
      "unimplemented for other assignment operator!"
    );
    assert!(lookup!(self, left).ir_type.is_pointer());
    self.emit(
      inst::Memory::Store(inst::Store::new(left, right)),
      qualified_type,
    )
  }

  fn logical(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn arithmetic(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn bitwise(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn bitshift(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn comma(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }
}
impl<'context> Emitter<'_, 'context, '_> {
  fn addressof(
    &self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Ampersand);
    todo!()
  }

  fn indirect(
    &self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn logical_not(
    &mut self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    assert!(operator == Operator::Not);
    let typ = lookup!(self, operand).ir_type;

    match typ {
      ir::Type::Pointer() => todo!(),
      ir::Type::Float() => todo!(),
      ir::Type::Double() => todo!(),
      ir::Type::Integer(width) => {
        let i1_false: ValueID = self.emit(
          Constant::Integral(Integral::new(
            0,
            1,
            crate::common::Signedness::Unsigned,
          )),
          self.session.ast_context.bool_type().into(),
        );
        let cmp = self.emit(
          inst::ICmp::new(inst::ICmpPredicate::Ne, operand, i1_false),
          qualified_type,
        );
        let i1_true = self.emit(
          Constant::Integral(Integral::new(
            1,
            1,
            crate::common::Signedness::Unsigned,
          )),
          self.session.ast_context.bool_type().into(),
        );
        let xor = self.emit(
          inst::Binary::new(inst::BinaryOp::Xor, cmp, i1_true),
          self.session.ast_context.bool_type().into(),
        );
        self.emit(inst::Cast::Zext(inst::Zext::new(xor)), qualified_type)
      },
      _ => unreachable!(),
    }
  }

  fn tilde(
    &self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn unary_arithmetic(
    &self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn ppmm(
    &self,
    operator: Operator,
    operand: ValueID,
    kind: crate::blueprints::RawUnaryKind,
    qualified_type: QualifiedType<'context>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }
}
