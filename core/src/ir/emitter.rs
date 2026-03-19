#![allow(unused)]
#![deny(unused_must_use)]
#![allow(clippy::needless_pass_by_ref_mut)]

use ::rcc_utils::contract_violation;
use ::slotmap::Key;
use ::std::collections::HashMap;

use super::{
  Argument,
  emitable::Emitable,
  instruction::{self as inst},
  module::{self, BasicBlock, Module},
  value::{Value, ValueID},
};
use crate::{
  blueprints::UnaryKind,
  common::{
    FloatFormat, Floating, Integral, Operator, OperatorCategory, RefEq,
    Signedness, SourceSpan, StrRef, Symbol, SymbolPtr,
  },
  ir,
  sema::{declaration as sd, expression as se, statement as ss},
  session::{Session, SessionRef},
  types::{CastType, Constant, QualifiedType, TypeInfo},
};
pub struct Emitter<'c> {
  pub(super) session: SessionRef<'c>,
  /// The basic block currently being written into
  pub(super) current_block: ValueID,
  /// Blocks finalized in the current function
  pub(super) current_blocks: Vec<ValueID>,
  pub(super) locals: HashMap<SymbolPtr<'c>, ValueID>,
  /// function name → ValueID for call resolution
  pub(super) globals: HashMap<SymbolPtr<'c>, ValueID>,
  pub(super) module: Module,
}
impl<'a> ::std::ops::Deref for Emitter<'a> {
  type Target = Session<'a>;

  fn deref(&self) -> &Self::Target {
    self.session
  }
}
#[macro_use]
mod macros {
  macro_rules! ty {
    ($self:ident, $qualified_type:expr) => {
      $self.session.ir().ir_type(&$qualified_type)
    };
  }
  macro_rules! lookup {
    ($self:ident, $value_id:expr) => {
      $self.session().ir().get($value_id)
    };
  }
  macro_rules! lookup_mut {
    ($self:ident, $value_id:expr) => {
      $self.session().ir().get_mut($value_id)
    };
  }
}
impl<'c> Emitter<'c> {
  pub fn new(session: SessionRef<'c>) -> Self {
    Self {
      session,
      current_block: Default::default(),
      current_blocks: Default::default(),
      locals: Default::default(),
      globals: Default::default(),
      module: Default::default(),
    }
  }

  #[inline(always)]
  pub(super) fn session(&self) -> SessionRef<'c> {
    self.session
  }
}
impl<'c> Emitter<'c> {
  #[must_use]
  fn push_block(&mut self, block_id: ValueID) -> ValueID {
    let old_id = self.seal_current_block();
    self.current_block = block_id;
    old_id
  }

  #[must_use]
  fn seal_current_block(&mut self) -> ValueID {
    if !self.current_block.is_null() {
      assert!(
        !lookup!(self, self.current_block)
          .data
          .as_basicblock_unchecked()
          .terminator
          .is_null(),
        "BasicBlock must ends with a proper terminator."
      );
      let current_block_id = self.current_block;
      self.current_blocks.push(self.current_block);
      self.current_block = ValueID::null();
      current_block_id
    } else {
      // do nothing
      ValueID::null()
    }
  }

  #[must_use]
  fn new_block(basic_block: BasicBlock, session: SessionRef<'c>) -> ValueID {
    session.ir().insert(Value::new(
      session.ast().void_type().into(),
      session.ir().label_type(),
      basic_block.into(),
    ))
  }

  fn is_global(&self) -> bool {
    self.current_block.is_null()
  }
}

impl<'c> Emitter<'c> {
  pub fn build(mut self, translation_unit: sd::TranslationUnit<'c>) -> Module {
    let declarations = translation_unit.declarations;

    self.module.globals = Vec::with_capacity(declarations.len());

    declarations
      .into_iter()
      .for_each(|declaration| self.global_decl(declaration));
    self.module
  }
}

impl<'c> Emitter<'c> {
  fn global_decl(&mut self, declaration: sd::ExternalDeclaration<'c>) {
    match declaration {
      sd::ExternalDeclaration::Function(function) =>
        match function.is_definition() {
          true => self.global_funcdef(function),
          false => self.funcdecl(function),
        },
      sd::ExternalDeclaration::Variable(variable) => {
        self.global_vardef(variable);
      },
    }
  }

  fn funcdecl(&mut self, function: sd::Function<'c>) {
    if let Some(&value_id) =
      self.globals.get(&(function.symbol.as_ptr() as SymbolPtr))
    {
      debug_assert!(
        lookup!(self, value_id).data.is_function(),
        "pre-registered value should be a function"
      );
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

      self
        .globals
        .insert(function.symbol.as_ptr() as SymbolPtr, value_id);
    }
  }

  fn global_funcdef(&mut self, function: sd::Function<'c>) {
    assert!(function.is_definition());

    let function_id = if let Some(&value_id) = self
      .globals
      .get(&(function.symbol.as_ptr() as *const Symbol<'c>))
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

      self
        .globals
        .insert(function.symbol.as_ptr() as SymbolPtr, function_id);
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
    assert!(self.current_block.is_null());

    let block_id = Self::new_block(Default::default(), self.session());

    _ = self.push_block(block_id);
    let params = {
      let return_type = lookup!(self, function_id)
        .qualified_type
        .as_functionproto_unchecked()
        .return_type;

      // return value storage
      let _return_slot_id = self.emit(inst::Alloca::new(), return_type);
      // _ = self.emit(
      //   inst::Memory::Store(inst::Store::new(
      // self.ast().void_type().into()
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
          self.locals.insert(parameter.symbol.as_ptr(), arg_id);
          let localed_arg_id = self.emit(inst::Alloca::new(), qualified_type);
          _ = self.emit(
            inst::Memory::Store(inst::Store::new(localed_arg_id, arg_id)),
            self.ast().void_type().into(),
          );
          arg_id
        })
        .collect::<Vec<_>>()
    };
    self.compound(body.expect("Precondition: function.is_definition()"));
    _ = self.seal_current_block();

    let mut refmut = lookup_mut!(self, function_id);
    let mutref = refmut.data.as_function_mut_unchecked();
    mutref.params = params;
    mutref.blocks = ::std::mem::take(&mut self.current_blocks);

    self.locals.clear();
  }

  fn global_vardef(&mut self, variable: sd::VarDef<'c>) {
    let sd::VarDef {
      symbol,
      initializer,
      ..
    } = variable;
    let initializer = match initializer {
      Some(sd::Initializer::Scalar(expr)) => Some(module::Initializer::Scalar(
        expr.destructure().0.into_constant_unchecked().inner,
      )),
      Some(sd::Initializer::Aggregate(_)) => todo!(),
      None => None,
    };
    let value_id = self.emit(
      module::Variable::new(
        symbol.borrow().name,
        initializer, // TODO: handle initializers
      ),
      symbol.borrow().qualified_type,
    );
    self.globals.insert(symbol.as_ptr(), value_id);
  }

  fn local_decl(&mut self, external_declaration: sd::ExternalDeclaration<'c>) {
    debug_assert!(!self.is_global());
    match external_declaration {
      sd::ExternalDeclaration::Function(function) => {
        debug_assert!(function.is_declaration());
        self.funcdecl(function);
      },
      sd::ExternalDeclaration::Variable(var_def) => self.local_vardef(var_def),
    }
  }

  fn local_vardef(&mut self, var_def: sd::VarDef<'c>) {
    let sd::VarDef {
      symbol,
      initializer,
      ..
    } = var_def;
    let value_id =
      self.emit(inst::Alloca::new(), symbol.borrow().qualified_type);

    match initializer {
      Some(sd::Initializer::Scalar(expr)) => {
        let init_value_id = self.expression(expr);
        _ = self.emit(
          inst::Memory::Store(inst::Store::new(value_id, init_value_id)),
          self.ast().void_type().into(),
        );
      },
      Some(sd::Initializer::Aggregate(_)) => todo!(),
      None => (),
    };
    self.locals.insert(symbol.as_ptr(), value_id);
  }
}

impl<'c> Emitter<'c> {
  fn statement(&mut self, statement: ss::Statement<'c>) {
    use ss::Statement::*;
    match statement {
      Empty(_) => (),
      Return(return_stmt) => self.returnstmt(return_stmt),
      Expression(expression) => self.exprstmt(expression),
      Declaration(declaration) => self.local_decl(declaration),
      Compound(compound) => self.compound(compound),
      If(if_stmt) => self.if_stmt(if_stmt),
      While(while_stmt) => self.while_stmt(while_stmt),
      DoWhile(do_while) => self.do_while(do_while),
      For(for_stmt) => self.for_stmt(for_stmt),
      Switch(switch) => self.switch(switch),
      Goto(goto) => self.goto(goto),
      Label(label) => self.label(label),
      Break(break_stmt) => self.break_stmt(break_stmt),
      Continue(continue_stmt) => self.continue_stmt(continue_stmt),
    }
  }

  #[inline]
  fn exprstmt(&mut self, expression: se::Expression<'c>) {
    self.expression(expression);
  }

  fn returnstmt(&mut self, return_stmt: ss::Return<'c>) {
    let ss::Return { expression, .. } = return_stmt;
    let qualified_type = expression
      .as_ref()
      .map(|e| *e.qualified_type())
      .unwrap_or(self.ast().void_type().into());
    let operand: Option<ValueID> = expression.map(|e| self.expression(e));
    _ = self.emit(
      inst::Terminator::Return(inst::Return::new(operand)),
      qualified_type,
    );
  }

  fn compound(&mut self, body: ss::Compound<'c>) {
    let ss::Compound { statements, .. } = body;
    statements
      .into_iter()
      .for_each(|statement| self.statement(statement));
  }

  fn if_stmt(&mut self, if_stmt: ss::If<'c>) {
    let ss::If {
      condition,
      then_branch,
      else_branch,
      ..
    } = if_stmt;
    let condition = self.expression(condition);
    debug_assert!(
      lookup!(self, condition)
        .qualified_type
        .as_primitive()
        .is_some_and(|p| p.is_contextual_bool())
    );

    let now_block_id = self.current_block;

    let now_block_terminator = self.emit(
      inst::Terminator::Branch(inst::Branch::new(
        condition,
        ValueID::null(),
        ValueID::null(),
      )),
      self.ast().i1_bool_type().into(),
    );
    let then_block_id = Self::new_block(Default::default(), self.session());
    let should_be_now = self.push_block(then_block_id);
    assert!(should_be_now == now_block_id);

    self.statement(*then_branch);

    let then_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;

      if terminator.is_null() {
        self.emit_terminator(
          inst::Jump::new(ValueID::null()),
          self.ast().void_type().into(),
          self.current_block,
        )
      } else {
        terminator
      }
    };
    let else_block_id = Self::new_block(Default::default(), self.session());
    let shuold_be_then = self.push_block(else_block_id);
    assert!(shuold_be_then == then_block_id);

    {
      let mut refmut = lookup_mut!(self, now_block_terminator);
      let mutref = refmut
        .data
        .as_instruction_mut_unchecked()
        .as_terminator_mut_unchecked()
        .as_branch_mut_unchecked();
      mutref.else_branch = else_block_id;
      mutref.then_branch = then_block_id;
    }

    let else_block_terminator = if let Some(stmt) = else_branch {
      self.statement(*stmt);
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;
      if terminator.is_null() {
        self.emit_terminator(
          inst::Jump::new(ValueID::null()),
          self.ast().void_type().into(),
          self.current_block,
        )
      } else {
        terminator
      }
    } else {
      ValueID::null()
    };

    if else_block_terminator.is_null() {
      {
        let mut refmut = lookup_mut!(self, then_block_terminator);
        let mutref = refmut
          .data
          .as_instruction_mut_unchecked()
          .as_terminator_mut_unchecked();
        match mutref {
          inst::Terminator::Jump(jump) => jump.to = else_block_id,
          inst::Terminator::Branch(branch) =>
            branch.then_branch = else_block_id,
          inst::Terminator::Return(_) => (),
        }
      }
    } else {
      let immediate_block_id =
        Self::new_block(Default::default(), self.session());
      let should_be_else = self.push_block(immediate_block_id);
      assert!(should_be_else == else_block_id);
      {
        let mut refmut = lookup_mut!(self, else_block_terminator);
        let mutref = refmut
          .data
          .as_instruction_mut_unchecked()
          .as_terminator_mut_unchecked();
        match mutref {
          inst::Terminator::Jump(jump) => jump.to = immediate_block_id,
          inst::Terminator::Branch(branch) =>
            branch.then_branch = immediate_block_id,
          inst::Terminator::Return(_) => (),
        }
      }
      {
        let mut refmut = lookup_mut!(self, then_block_terminator);
        let mutref = refmut
          .data
          .as_instruction_mut_unchecked()
          .as_terminator_mut_unchecked();
        match mutref {
          inst::Terminator::Jump(jump) => jump.to = immediate_block_id,
          inst::Terminator::Branch(branch) =>
            branch.then_branch = immediate_block_id,
          inst::Terminator::Return(_) => (),
        }
      }
    }
  }

  fn while_stmt(&self, while_stmt: ss::While<'c>) {
    todo!()
  }

  fn do_while(&self, do_while: ss::DoWhile<'c>) {
    todo!()
  }

  fn for_stmt(&self, for_stmt: ss::For<'c>) {
    todo!()
  }

  fn switch(&self, switch: ss::Switch<'c>) {
    todo!()
  }

  fn goto(&self, goto: ss::Goto<'c>) {
    todo!()
  }

  fn label(&self, label: ss::Label<'c>) {
    todo!()
  }

  fn break_stmt(&self, break_stmt: ss::Break<'c>) {
    todo!()
  }

  fn continue_stmt(&self, continue_stmt: ss::Continue<'c>) {
    todo!()
  }
}
impl<'c> Emitter<'c> {
  fn expression(&mut self, expression: se::Expression<'c>) -> ValueID {
    // the fold here contains partial fold. e.g. `3 + 6 + func(4 + 5)` would be folded to `9 + func(9)`.
    let (raw_expr, qualified_type, ..) =
      expression.fold(self.diag()).take().destructure();
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
    constant: se::Constant<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    self.emit(constant.inner, qualified_type)
  }

  fn unary(
    &mut self,
    unary: se::Unary<'c>,
    qualified_type: QualifiedType<'c>,
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
    binary: se::Binary<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let se::Binary {
      left,
      operator,
      right,
      span,
    } = binary;

    let left = self.expression(*left);
    let right = self.expression(*right);
    self.do_binary(operator, left, right, qualified_type, span)
  }

  fn do_binary(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
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
    member_access: se::MemberAccess<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    todo!("GEP")
  }

  fn ternary(
    &mut self,
    ternary: se::Ternary<'c>,
    qualified_type: QualifiedType<'c>,
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
    size_of: se::SizeOf<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let se::SizeOf { sizeof, .. } = size_of;
    match sizeof {
      se::SizeOfKind::Type(qualified_type) => self.emit(
        Constant::Integral(Integral::from_unsigned(
          qualified_type.size(),
          self.ast().uintptr_type().size() as u8,
        )),
        *qualified_type,
      ),
      se::SizeOfKind::Expression(expr) => self.expression(*expr),
    }
  }

  fn cstyle_cast(
    &mut self,
    cstyle_cast: se::CStyleCast<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    todo!()
  }

  fn array_subscript(
    &mut self,
    array_subscript: se::ArraySubscript<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    todo!()
  }

  fn compound_literal(
    &mut self,
    compound_literal: se::CompoundLiteral,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    todo!()
  }

  fn variable(
    &self,
    variable: se::Variable<'c>,
    _qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let name = variable.name.borrow().name;
    if let Some(&vid) = self.locals.get(&(variable.name.as_ptr() as SymbolPtr))
    {
      vid
    } else if let Some(&vid) =
      self.globals.get(&(variable.name.as_ptr() as SymbolPtr))
    {
      vid
    } else {
      panic!("undefined variable: {name}")
    }
  }

  fn implicit_cast(
    &mut self,
    implicit_cast: se::ImplicitCast<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let se::ImplicitCast {
      cast_type, expr, ..
    } = implicit_cast;

    use ::std::cmp::Ordering::*;
    use CastType::*;
    use Signedness::*;
    use inst::{Cast, Load, Memory, Sext, Trunc, Zext};

    let value_id = self.expression(*expr);
    match cast_type {
      Noop | FunctionToPointerDecay | ArrayToPointerDecay => value_id,
      LValueToRValue =>
        self.emit(Memory::from(Load::new(value_id)), qualified_type),
      IntegralCast => {
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
          Less => match qualified_type.signedness() {
            Some(Signed) =>
              self.emit(Cast::from(Sext::new(value_id)), qualified_type),
            Some(Unsigned) =>
              self.emit(Cast::from(Zext::new(value_id)), qualified_type),
            None => unreachable!(),
          },
          Equal => value_id,
          Greater =>
            self.emit(Cast::from(Trunc::new(value_id)), qualified_type),
        }
      },
      _ => todo!("implicit cast: {:?}", implicit_cast.cast_type),
    }
  }

  fn call(
    &mut self,
    call: se::Call<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let se::Call {
      callee, arguments, ..
    } = call;

    let callee = self.expression(*callee);

    let args: Vec<ValueID> = arguments
      .into_iter()
      .map(|arg| self.expression(arg))
      .collect();

    self.emit(inst::Call::new(callee, args), qualified_type)
  }

  #[inline]
  fn paren(&mut self, paren: se::Paren<'c>) -> ValueID {
    self.expression(*paren.expr)
  }
}
impl<'c> Emitter<'c> {
  fn assignment(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator.category(), OperatorCategory::Assignment);
    assert!(lookup!(self, left).ir_type.is_pointer());

    let calculated_right = match operator.associated_operator() {
      Some(operator) => {
        let loaded_left =
          self.emit(inst::Memory::from(inst::Load::new(left)), qualified_type);
        self.do_binary(operator, loaded_left, right, qualified_type, span)
      },
      None => right,
    };
    self.emit(
      inst::Memory::Store(inst::Store::new(left, calculated_right)),
      self.ast().void_type().into(),
    )
  }

  fn logical(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    use inst::ICmpPredicate::*;
    use ir::Type::*;

    debug_assert!(
      qualified_type
        .as_primitive()
        .is_some_and(|p| p.is_contextual_bool())
    );

    let boolean = self.ast().i1_bool_type().into();

    let lhs_ir_type = lookup!(self, left).ir_type;
    let rhs_ir_type = lookup!(self, right).ir_type;
    let i1 = match (lhs_ir_type, rhs_ir_type) {
      (Integer(left_width), Integer(right_width)) => {
        debug_assert!(
          left_width == right_width
            && (lookup!(self, left).qualified_type.signedness()
              == lookup!(self, right).qualified_type.signedness())
        );
        let signedness = lookup!(self, left)
          .qualified_type
          .signedness()
          .expect("impossible to fail");
        self
          .integral_relational(operator, left, right, boolean, signedness, span)
      },
      (Floating(_), Floating(_)) =>
        self.floating_relational(operator, left, right, boolean, span),
      (Pointer(), Integer(integer)) | (Integer(integer), Pointer()) =>
        panic!("this should be rejected or emit a warning"),
      (Pointer(), Pointer()) => self.emit(
        inst::ICmp::new(
          inst::ICmpPredicate::from_op_and_sign(operator, Signedness::Unsigned),
          left,
          right,
        ),
        boolean,
      ),
      _ => unreachable!(),
    };
    self.emit(inst::Cast::from(inst::Zext::new(i1)), qualified_type)
  }

  fn integral_relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    signedness: Signedness,
    span: SourceSpan,
  ) -> ValueID {
    self.emit(
      inst::ICmp::new(
        inst::ICmpPredicate::from_op_and_sign(operator, signedness),
        left,
        right,
      ),
      qualified_type,
    )
  }

  fn floating_relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    use inst::FCmpPredicate::*;
    let predicate = match operator {
      Less => Olt,
      LessEqual => Ole,
      Greater => Ogt,
      GreaterEqual => Oge,
      EqualEqual => Oeq,
      // `NaN` always not equal than other, even both are `NaN`.
      NotEqual => Une,
      _ => unreachable!(),
    };
    self.emit(inst::FCmp::new(predicate, left, right), qualified_type)
  }

  fn arithmetic(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn bitwise(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    let bitwise = match operator {
      Ampersand => inst::BinaryOp::And,
      Pipe => inst::BinaryOp::Or,
      Caret => inst::BinaryOp::Xor,
      _ => unreachable!(),
    };
    self.emit(inst::Binary::new(bitwise, left, right), qualified_type)
  }

  fn bitshift(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    use Signedness::*;
    use inst::BinaryOp::*;

    debug_assert!(
      lookup!(self, right)
        .qualified_type
        .as_primitive()
        .is_some_and(|p| p.is_integer())
    );

    let bitshift =
      match (operator, lookup!(self, left).qualified_type.signedness()) {
        (LeftShift, Some(_)) => Shl,
        (RightShift, Some(Signed)) => AShr,
        (RightShift, Some(Unsigned)) => LShr,
        _ => unreachable!(),
      };
    self.emit(inst::Binary::new(bitshift, left, right), qualified_type)
  }

  #[inline]
  fn comma(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert!(operator == Operator::Comma);
    right
  }
}
impl<'c> Emitter<'c> {
  #[inline]
  fn addressof(
    &mut self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Ampersand);
    operand
  }

  fn indirect(
    &mut self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Star);
    assert!(lookup!(self, operand).ir_type.is_pointer());
    self.emit(inst::Memory::from(inst::Load::new(operand)), qualified_type)
  }

  fn logical_not(
    &mut self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Not);

    /// FIXME: avoid this trick but also reuse the code below. borrowck wont let self being borrowed
    /// SAFETY: safe today, maybe not tomorrow.
    let this = ::std::ptr::from_mut(self);

    let common = move |cmp| {
      let this = unsafe { &mut *this };
      let i1_true = this.emit(
        Constant::Integral(Integral::i1_true()),
        this.ast().i1_bool_type().into(),
      );
      let xor = this.emit(
        inst::Binary::new(inst::BinaryOp::Xor, cmp, i1_true),
        this.ast().i1_bool_type().into(),
      );
      this.emit(inst::Cast::from(inst::Zext::new(xor)), qualified_type)
    };
    let integral = move |i1_false_or_nullptr| {
      let this = unsafe { &mut *this };
      let cmp = this.emit(
        inst::ICmp::new(inst::ICmpPredicate::Ne, operand, i1_false_or_nullptr),
        this.ast().i1_bool_type().into(),
      );
      common(cmp)
    };

    let cannot_inline_me_otherwise_refcell_panic =
      lookup!(self, operand).ir_type;

    match cannot_inline_me_otherwise_refcell_panic {
      ir::Type::Pointer() => {
        let nullptr = self.emit(
          Constant::Nullptr(().into()),
          self.ast().nullptr_type().into(),
        );
        integral(nullptr)
      },
      ir::Type::Integer(width) => {
        let i1_false = self.emit(
          Constant::Integral(Integral::i1_false()),
          self.ast().i1_bool_type().into(),
        );
        integral(i1_false)
      },
      ir::Type::Floating(format) => {
        let float_zero = self
          .emit(Constant::Floating(Floating::zero(*format)), qualified_type);

        let cmp = self.emit(
          inst::FCmp::new(inst::FCmpPredicate::Une, operand, float_zero),
          self.ast().i1_bool_type().into(),
        );
        common(cmp)
      },
      _ => unreachable!(),
    }
  }

  fn tilde(
    &mut self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert!(operator == Operator::Tilde);
    assert!(
      qualified_type
        .as_primitive()
        .is_some_and(|p| p.is_integer())
    );
    let bitmask = self.emit(
      Constant::Integral(Integral::bitmask(qualified_type.size_bits() as u8)),
      qualified_type,
    );
    self.emit(
      inst::Binary::new(inst::BinaryOp::Xor, operand, bitmask),
      qualified_type,
    )
  }

  fn unary_arithmetic(
    &mut self,
    operator: Operator,
    operand: ValueID,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    match operator {
      Operator::Plus => operand,
      Operator::Minus => {
        let zero = self.emit(
          Constant::Integral(Integral::from_unsigned(
            0,
            lookup!(self, operand).qualified_type.size_bits() as u8,
          )),
          qualified_type,
        );
        self.emit(
          inst::Binary::new(inst::BinaryOp::Sub, zero, operand),
          qualified_type,
        )
      },
      _ => unreachable!(),
    }
  }

  fn ppmm(
    &mut self,
    operator: Operator,
    operand: ValueID,
    kind: UnaryKind,
    qualified_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    use UnaryKind::*;
    use inst::BinaryOp::*;

    let value = lookup!(self, operand);
    debug_assert!(matches!(
      value.ir_type,
      ir::Type::Pointer() | ir::Type::Integer(_) | ir::Type::Floating(_)
    ));
    debug_assert!(matches!(operator, PlusPlus | MinusMinus));

    let size = value.qualified_type.size_bits() as u8;

    let binaryop = match operator {
      PlusPlus => Add,
      MinusMinus => Sub,
      _ => unreachable!(),
    };
    match kind {
      Prefix => {
        let one = self.emit(
          Constant::Integral(Integral::from_unsigned(1, size)),
          qualified_type,
        );
        let calculated =
          self.emit(inst::Binary::new(binaryop, operand, one), qualified_type);
        _ = self.emit(
          inst::Memory::Store(inst::Store::new(operand, calculated)),
          self.ast().void_type().into(),
        );
        calculated
      },
      Postfix => {
        let one = self.emit(
          Constant::Integral(Integral::from_unsigned(1, size)),
          qualified_type,
        );
        let calculated =
          self.emit(inst::Binary::new(binaryop, operand, one), qualified_type);
        _ = self.emit(
          inst::Memory::Store(inst::Store::new(operand, calculated)),
          self.ast().void_type().into(),
        );
        operand
      },
    }
  }
}
