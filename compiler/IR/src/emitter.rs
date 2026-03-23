#![allow(unused)]
#![deny(unused_must_use)]
#![allow(clippy::needless_pass_by_ref_mut)]

use ::rcc_adt::{Floating, Integral, Signedness};
use ::rcc_ast::{
  SymbolPtr, UnaryKind,
  types::{self as ast, TypeInfo},
};
use ::rcc_sema::{declaration as sd, expression as se, statement as ss};
use ::rcc_shared::{Constant, OpDiag, Operator, OperatorCategory, SourceSpan};
use ::rcc_utils::{RefEq, SmallString, Unbox, contract_violation};
use ::slotmap::Key;
use ::std::collections::HashMap;

use super::{
  Argument,
  context::{Session, SessionRef},
  emitable::Emitable,
  instruction::{self as inst},
  module::{self, BasicBlock, Module},
  value::{Value, ValueID, WithActionMut},
};

pub struct Emitter<'c> {
  pub(super) session: SessionRef<'c, OpDiag<'c>>,
  /// The basic block currently being written into
  pub(super) current_block: ValueID,
  /// Blocks finalized in the current function
  pub(super) current_function: ValueID,
  pub(super) labels: HashMap<SmallString, ValueID>,
  // pub(super) current_blocks: Vec<ValueID>,
  pub(super) locals: HashMap<SymbolPtr<'c>, ValueID>,
  /// function name → ValueID for call resolution
  pub(super) globals: HashMap<SymbolPtr<'c>, ValueID>,
  pub(super) module: Module,
}
impl<'a> ::std::ops::Deref for Emitter<'a> {
  type Target = Session<'a, OpDiag<'a>>;

  fn deref(&self) -> &Self::Target {
    self.session
  }
}
#[macro_use]
mod macros {
  macro_rules! ty {
    ($self:ident, $ast_type:expr) => {
      $self.ir().ir_type(&$ast_type)
    };
  }
  macro_rules! lookup {
    ($self:ident, $value_id:expr) => {
      $self.ir().get($value_id)
    };
  }
  macro_rules! lookup_mut {
    ($self:ident, $value_id:expr) => {
      $self.ir().get_mut($value_id)
    };
  }
}
impl<'c> Emitter<'c> {
  pub fn new(session: SessionRef<'c, OpDiag<'c>>) -> Self {
    Self {
      session,
      current_block: Default::default(),
      current_function: Default::default(),
      locals: Default::default(),
      globals: Default::default(),
      module: Default::default(),
      labels: Default::default(),
    }
  }

  #[inline(always)]
  pub(super) fn session(&self) -> SessionRef<'c, OpDiag<'c>> {
    self.session
  }

  #[inline(always)]
  pub(super) fn visit<R, F: FnOnce(&Value<'c>) -> R>(
    &self,
    id: ValueID,
    action: F,
  ) -> R {
    self.session().ir().visit(id, action)
  }

  #[inline(always)]
  pub(super) fn apply<R, F: FnOnce(&mut Value<'c>) -> R>(
    &self,
    id: ValueID,
    action: F,
  ) -> R {
    self.session().ir().apply(id, action)
  }
}
impl<'c> Emitter<'c> {
  fn contextual_convert_to_i1(&mut self, value_id: ValueID) -> ValueID {
    let (ir_type, ast_type) = {
      let value = lookup!(self, value_id);
      (value.ir_type, value.ast_type)
    };
    use inst::*;

    use super::types::Type;
    match ir_type {
      Type::Void()
      | Type::Label()
      | Type::Struct(_)
      | Type::Array(_)
      | Type::Function(_) => unreachable!(),
      Type::Pointer() => self.emit(
        ICmp::new(ICmpPredicate::Ne, value_id, self.ir().nullptr()),
        self.ast().i1_bool_type(),
      ),
      Type::Floating(format) => self.emit(
        FCmp::new(
          FCmpPredicate::Une,
          value_id,
          self.ir().floating_zero(*format),
        ),
        self.ast().i1_bool_type(),
      ),
      Type::Integer(width) => self.emit(
        ICmp::new(ICmpPredicate::Ne, value_id, self.ir().integer_zero(*width)),
        self.ast().i1_bool_type(),
      ),
    }
  }
}
impl<'c> Emitter<'c> {
  #[must_use]
  fn push_block(&mut self, block_id: ValueID) -> ValueID {
    let old_id = self.current_block;
    self.seal_current_block();
    self.current_block = block_id;
    old_id
  }

  fn seal_current_block(&mut self) {
    self.current_block.and_then(|block_id: ValueID| {
      assert!(
        !lookup!(self, self.current_block)
          .data
          .as_basicblock_unchecked()
          .terminator
          .is_null(),
        "BasicBlock must ends with a proper terminator before adding it to \
         parent function."
      );
      lookup_mut!(self, self.current_function)
        .data
        .as_function_mut_unchecked()
        .blocks
        .push(self.current_block);
      self.current_block = ValueID::null();
      block_id
    });
  }

  #[must_use]
  fn new_empty_block(&mut self) -> ValueID {
    debug_assert!(!self.current_function.is_null());
    self.ir().insert(Value::new(
      self.ast().void_type(),
      self.ir().label_type(),
      BasicBlock::default(),
      self.current_function,
    ))
  }

  fn refill_branch(
    &mut self,
    branch_id: ValueID,
    then_block_id: ValueID,
    else_block_id: ValueID,
  ) -> ValueID {
    lookup_mut!(self, branch_id).with_action_mut(|now| {
      let branch = now
        .data
        .as_instruction_mut_unchecked()
        .as_terminator_mut_unchecked()
        .as_branch_mut_unchecked();
      branch.set_else_branch(else_block_id);
      branch.set_then_branch(then_block_id);
    });
    self.ir().add_user_for(branch_id, then_block_id);
    self.ir().add_user_for(branch_id, else_block_id);
    branch_id
  }

  /// terminator return also handlede here, but no effect.
  fn refill_jump(&mut self, jump_id: ValueID, to_block_id: ValueID) -> ValueID {
    lookup_mut!(self, jump_id).with_action_mut(|jump| {
      jump
        .data
        .as_instruction_mut_unchecked()
        .as_terminator_mut_unchecked()
        .as_jump_mut()
        .map(|j| j.set_target(to_block_id))
    });
    self.ir().add_user_for(jump_id, to_block_id);
    jump_id
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
      self.globals.get(&(function.symbol.as_ptr() as *const _))
    {
      debug_assert!(
        lookup!(self, value_id).data.is_function(),
        "pre-registered value should be a function"
      );
    } else {
      let sym = function.symbol.borrow();
      let name = sym.name;
      let ast_type = sym.qualified_type.unqualified_type;
      let is_variadic = ast_type.as_functionproto_unchecked().is_variadic;
      drop(sym);

      let value_id = self.emit(
        module::Function::new_empty(name, Default::default(), is_variadic),
        ast_type,
      );

      self
        .globals
        .insert(function.symbol.as_ptr() as *const _, value_id);
    }
  }

  fn global_funcdef(&mut self, function: sd::Function<'c>) {
    assert!(function.is_definition());

    self.current_function = if let Some(&value_id) =
      self.globals.get(&(function.symbol.as_ptr() as *const _))
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
      let ast_type = sym.qualified_type.unqualified_type;
      let is_variadic = ast_type.as_functionproto_unchecked().is_variadic;
      drop(sym);

      let function_id = self.emit(
        module::Function::new_empty(name, Default::default(), is_variadic),
        ast_type,
      );

      self
        .globals
        .insert(function.symbol.as_ptr() as *const _, function_id);
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

    assert!(self.locals.is_empty());
    assert!(self.labels.is_empty());

    assert!(self.current_block.is_null());

    debug_assert!(!self.current_function.is_null());
    let entry_id = self.ir().insert(Value::new(
      self.ast().void_type(),
      self.ir().label_type(),
      BasicBlock::new(
        Vec::with_capacity((parameters.len() + 1) * 2 + 1),
        Default::default(),
      ),
      self.current_function,
    ));

    let _null = self.push_block(entry_id);
    assert!(_null.is_null());

    {
      let return_type = lookup!(self, self.current_function)
        .ast_type
        .as_functionproto_unchecked()
        .return_type
        .unqualified_type;

      // return value storage
      let return_slot_id = self.emit(inst::Alloca::new(), return_type);

      let default_value_id =
        self.emit(return_type.default_value(), return_type);
      _ = self.emit(
        inst::Memory::Store(inst::Store::new(return_slot_id, default_value_id)),
        self.ast().void_type(),
      );

      // insert params into the local scope and allocate spaces
      parameters
        .into_iter()
        .enumerate()
        .for_each(|(index, parameter)| {
          let ast_type =
            parameter.symbol.borrow().qualified_type.unqualified_type;
          let arg_id = self.emit(Argument::new(index), ast_type);
          self.locals.insert(parameter.symbol.as_ptr(), arg_id);
          let localed_arg_id = self.emit(inst::Alloca::new(), ast_type);
          _ = self.emit(
            inst::Memory::Store(inst::Store::new(localed_arg_id, arg_id)),
            self.ast().void_type(),
          )
        })
    };

    let body_id = self.new_empty_block();
    let unconditional_jump = self.emit(
      inst::Terminator::Jump(inst::Jump::new(body_id)),
      self.ast().void_type(),
    );

    self.apply(self.current_function, |value| {
      value.data.as_function_mut_unchecked().entry = self.current_block
    });

    self.current_block = body_id;

    self.compound(body.expect("Precondition: function.is_definition()"));

    self.seal_current_block();

    self.locals.clear();
    self.labels.clear();
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
      symbol.borrow().qualified_type.unqualified_type,
    );
    self.globals.insert(symbol.as_ptr(), value_id);
  }

  fn local_decl(&mut self, external_declaration: sd::ExternalDeclaration<'c>) {
    debug_assert!(!self.current_block.is_null());
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
    let value_id = self.emit(
      inst::Alloca::new(),
      symbol.borrow().qualified_type.unqualified_type,
    );

    match initializer {
      Some(sd::Initializer::Scalar(expr)) => {
        let init_value_id = self.expression(expr);
        _ = self.emit(
          inst::Memory::Store(inst::Store::new(value_id, init_value_id)),
          self.ast().void_type(),
        );
      },
      Some(sd::Initializer::Aggregate(_)) => todo!(),
      None => (),
    };
    self.locals.insert(symbol.as_ptr(), value_id);
  }
}

impl<'c> Emitter<'c> {
  fn statement(&mut self, statement: impl Unbox<Output = ss::Statement<'c>>) {
    use ss::Statement::*;
    match statement.unbox() {
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
    let ast_type = expression
      .as_ref()
      .map(|e| e.unqualified_type())
      .unwrap_or(self.ast().void_type());
    let operand: Option<ValueID> = expression.map(|e| self.expression(e));
    _ = self.emit(
      inst::Terminator::Return(inst::Return::new(operand)),
      ast_type,
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

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let now_block_id = self.current_block;
    let now_block_terminator = self.emit(
      inst::Terminator::Branch(inst::Branch::new(
        condition,
        ValueID::null(),
        ValueID::null(),
      )),
      self.ast().void_type(),
    );

    let then_block_id = self.new_empty_block();
    let else_block_id = self.new_empty_block();

    let should_be_now = self.push_block(then_block_id);
    assert_eq!(should_be_now, now_block_id);

    self.statement(then_branch);

    let then_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;

      terminator.unwrap_or_else(|| {
        self.emit_terminator(
          inst::Jump::new(ValueID::null()),
          self.ast().void_type(),
          self.current_block,
        )
      })
    };

    // the assertion here is wrong. new controlflow may add many blocks.
    // let shuold_be_then = self.push_block(else_block_id);
    // assert_eq!(shuold_be_then, then_block_id);

    let _last_block_of_then = self.push_block(else_block_id);

    self.refill_branch(now_block_terminator, then_block_id, else_block_id);

    else_branch
      .map(|else_branch| {
        self.statement(else_branch);
        let terminator = lookup!(self, self.current_block)
          .data
          .as_basicblock_unchecked()
          .terminator;
        terminator.unwrap_or_else(|| {
          self.emit_terminator(
            inst::Jump::new(ValueID::null()),
            self.ast().void_type(),
            self.current_block,
          )
        })
      })
      .unwrap_or_default()
      .and_then(|else_block_terminator| {
        let immediate_block_id = self.new_empty_block();

        // ditto
        let _last_block_of_else = self.push_block(immediate_block_id);
        // assert_eq!(should_be_else, else_block_id);

        self.refill_jump(then_block_terminator, immediate_block_id);
        self.refill_jump(else_block_terminator, immediate_block_id)
      })
      .or_else(|| self.refill_jump(then_block_terminator, else_block_id));
  }

  fn while_stmt(&mut self, while_stmt: ss::While<'c>) {
    let ss::While {
      condition,
      body,
      tag, // tag is needed for break and continue, now TODO.
      ..
    } = while_stmt;

    let now_block_id = self.current_block;

    let cond_block_id = self.new_empty_block();
    let body_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    self.labels.insert(tag, cond_block_id);

    let now_block_terminator = self.emit(
      inst::Terminator::Jump(inst::Jump::new(cond_block_id)),
      self.ast().void_type(),
    );

    let should_be_now = self.push_block(cond_block_id);
    assert_eq!(should_be_now, now_block_id);

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let cond_block_terminator = self.emit(
      inst::Terminator::Branch(inst::Branch::new(
        condition,
        body_block_id,
        immediate_block_id,
      )),
      self.ast().void_type(),
    );

    let should_be_cond = self.push_block(body_block_id);
    assert_eq!(should_be_cond, cond_block_id);

    self.statement(body);

    let body_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;
      terminator.unwrap_or_else(|| {
        self.emit_terminator(
          inst::Jump::new(cond_block_id),
          self.ast().void_type(),
          self.current_block,
        )
      })
    };

    let _last_block_of_body = self.push_block(immediate_block_id);
  }

  fn do_while(&mut self, do_while: ss::DoWhile<'c>) {
    let ss::DoWhile {
      condition,
      body,
      tag,
      ..
    } = do_while;

    let now_block_id = self.current_block;

    let body_block_id = self.new_empty_block();
    let cond_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    let now_block_terminator = self.emit(
      inst::Terminator::Jump(inst::Jump::new(body_block_id)),
      self.ast().void_type(),
    );

    self.labels.insert(tag, cond_block_id);
    let _should_be_now = self.push_block(cond_block_id);
    assert_eq!(_should_be_now, now_block_id);

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let cond_block_terminator = self.emit(
      inst::Terminator::Branch(inst::Branch::new(
        condition,
        body_block_id,
        immediate_block_id,
      )),
      self.ast().void_type(),
    );

    let _should_be_cond = self.push_block(body_block_id);
    assert_eq!(_should_be_cond, cond_block_id);

    self.statement(body);

    let body_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;
      terminator.unwrap_or_else(|| {
        self.emit_terminator(
          inst::Jump::new(cond_block_id),
          self.ast().void_type(),
          self.current_block,
        )
      })
    };

    let _last_block_of_body = self.push_block(immediate_block_id);
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

  fn label(&mut self, label: ss::Label<'c>) {
    let ss::Label {
      name, statement, ..
    } = label;
    todo!()
  }

  fn break_stmt(&mut self, break_stmt: ss::Break<'c>) {
    let ss::Break { tag, .. } = break_stmt;
    let now_block_id = self.current_block;
    let cond_block_id = self
      .labels
      .get(&tag)
      .unwrap_or_else(|| panic!("tag {tag} not found!"));
    let term_id = lookup!(self, *cond_block_id)
      .data
      .as_basicblock_unchecked()
      .terminator
      .unwrap();
    let immediate_block_id = self.visit(term_id, |value| {
      value
        .data
        .as_instruction_unchecked()
        .as_terminator_unchecked()
        .as_branch()
        .expect("cond block should end with a branch inst, not others")
        .else_branch()
    });
    let break_inst_id = self.emit(
      inst::Terminator::Jump(inst::Jump::new(immediate_block_id)),
      self.ast().void_type(),
    );

    // unreachable block?
    let next_block_id = self.new_empty_block();
    let _should_be_now = self.push_block(next_block_id);
    assert_eq!(now_block_id, _should_be_now);
  }

  fn continue_stmt(&mut self, continue_stmt: ss::Continue<'c>) {
    let ss::Continue { tag, .. } = continue_stmt;
    let now_block_id = self.current_block;
    let cond_block_id = self
      .labels
      .get(&tag)
      .unwrap_or_else(|| panic!("tag {tag} not found!"));
    let continue_inst_id = self.emit(
      inst::Terminator::Jump(inst::Jump::new(*cond_block_id)),
      self.ast().void_type(),
    );
    let next_block_id = self.new_empty_block();
    let _should_be_now = self.push_block(next_block_id);
    assert_eq!(now_block_id, _should_be_now);
  }
}
impl<'c> Emitter<'c> {
  fn expression(
    &mut self,
    expression: impl Unbox<Output = se::Expression<'c>>,
  ) -> ValueID {
    // the fold here contains partial fold. e.g. `3 + 6 + func(4 + 5)` would be folded to `9 + func(9)`.
    let (
      raw_expr,
      ast::QualifiedType {
        unqualified_type, ..
      },
      ..,
    ) = expression.unbox().fold(self.diag()).take().destructure();
    use se::RawExpr::*;
    match raw_expr {
      Empty(_) => contract_violation!(
        "empty expr is used in sema for error recovery. shouldnt reach here."
      ),
      Constant(constant) => self.constant(constant, unqualified_type),
      Unary(unary) => self.unary(unary, unqualified_type),
      Binary(binary) => self.binary(binary, unqualified_type),
      Call(call) => self.call(call, unqualified_type),
      Paren(paren) => self.paren(paren),
      MemberAccess(member_access) =>
        self.member_access(member_access, unqualified_type),
      Ternary(ternary) => self.ternary(ternary, unqualified_type),
      SizeOf(size_of) => self.sizeof(size_of, unqualified_type),
      CStyleCast(cstyle_cast) =>
        self.cstyle_cast(cstyle_cast, unqualified_type),
      ArraySubscript(array_subscript) =>
        self.array_subscript(array_subscript, unqualified_type),
      CompoundLiteral(compound_literal) =>
        self.compound_literal(compound_literal, unqualified_type),
      Variable(variable) => self.variable(variable, unqualified_type),
      ImplicitCast(implicit_cast) =>
        self.implicit_cast(implicit_cast, unqualified_type),
    }
  }

  fn constant(
    &mut self,
    constant: se::Constant<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit(constant.inner, ast_type)
  }

  fn unary(
    &mut self,
    unary: se::Unary<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let se::Unary {
      kind,
      operand,
      operator,
      span,
    } = unary;
    let operand = self.expression(operand);
    match operator {
      Operator::Ampersand => self.addressof(operator, operand, ast_type, span),
      Operator::Star => self.indirect(operator, operand, ast_type, span),
      Operator::Not => self.logical_not(operator, operand, ast_type, span),
      Operator::Tilde => self.tilde(operator, operand, ast_type, span),
      Operator::Plus | Operator::Minus =>
        self.unary_arithmetic(operator, operand, ast_type, span),
      Operator::PlusPlus | Operator::MinusMinus =>
        self.ppmm(operator, operand, kind, ast_type, span),
      _ => unreachable!("operator is not unary: {:#?}", operator),
    }
  }

  fn binary(
    &mut self,
    binary: se::Binary<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let se::Binary {
      left,
      operator,
      right,
      span,
    } = binary;

    let left = self.expression(left);
    let right = self.expression(right);
    self.do_binary(operator, left, right, ast_type, span)
  }

  fn do_binary(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use OperatorCategory::*;

    macro_rules! call {
      ($method:ident) => {
        self.$method(operator, left, right, ast_type, span)
      };
    }

    match operator.category() {
      Assignment => call!(assignment),
      Logical => call!(logical),
      Relational => call!(relational),
      Arithmetic => call!(arithmetic),
      Bitwise => call!(bitwise),
      BitShift => call!(bitshift),
      Special => call!(comma),
      Uncategorized => unreachable!("operator is not binary: {:#?}", operator),
    }
  }

  fn member_access(
    &mut self,
    member_access: se::MemberAccess<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    todo!("GEP")
  }

  fn ternary(
    &mut self,
    ternary: se::Ternary<'c>,
    ast_type: ast::TypeRef<'c>,
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
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let se::SizeOf { sizeof, .. } = size_of;
    match sizeof {
      se::SizeOfKind::Type(qualified_type) => self.emit(
        Constant::Integral(Integral::from_unsigned(
          qualified_type.size(),
          self.ast().uintptr_type().size() as u8,
        )),
        ast_type,
      ),
      se::SizeOfKind::Expression(expr) => self.expression(expr),
    }
  }

  fn cstyle_cast(
    &mut self,
    cstyle_cast: se::CStyleCast<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    todo!()
  }

  fn array_subscript(
    &mut self,
    array_subscript: se::ArraySubscript<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    todo!()
  }

  fn compound_literal(
    &mut self,
    compound_literal: se::CompoundLiteral,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    todo!()
  }

  fn variable(
    &self,
    variable: se::Variable<'c>,
    _ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let name = variable.name.borrow().name;
    if let Some(&vid) = self.locals.get(&(variable.name.as_ptr() as *const _)) {
      vid
    } else if let Some(&vid) =
      self.globals.get(&(variable.name.as_ptr() as *const _))
    {
      vid
    } else {
      panic!("undefined variable: {name}")
    }
  }

  fn implicit_cast(
    &mut self,
    implicit_cast: se::ImplicitCast<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let se::ImplicitCast {
      cast_type, expr, ..
    } = implicit_cast;

    use ::std::cmp::Ordering::*;
    use Signedness::*;
    use ast::CastType::*;
    use inst::{Load, Memory, Sext, Trunc, Zext};

    let value_id = self.expression(expr);
    match cast_type {
      Noop | FunctionToPointerDecay | ArrayToPointerDecay => value_id,
      LValueToRValue => self.emit(Memory::from(Load::new(value_id)), ast_type),
      IntegralCast => {
        assert!(
          ast_type.as_primitive().is_some_and(|p| p.is_integer())
            && ast_type.size_bits() > 0
            && ast_type.size_bits() <= 128
        );
        match Ord::cmp(
          lookup!(self, value_id).ir_type.as_integer_unchecked(),
          &(ast_type.size_bits() as u8),
        ) {
          Less => match ast_type.signedness() {
            Some(Signed) =>
              self.emit(inst::Cast::from(Sext::new(value_id)), ast_type),
            Some(Unsigned) =>
              self.emit(inst::Cast::from(Zext::new(value_id)), ast_type),
            None => unreachable!(),
          },
          Equal => value_id,
          Greater =>
            self.emit(inst::Cast::from(Trunc::new(value_id)), ast_type),
        }
      },
      _ => todo!("implicit cast: {:?}", implicit_cast.cast_type),
    }
  }

  fn call(
    &mut self,
    call: se::Call<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let se::Call {
      callee, arguments, ..
    } = call;

    let mut operands = vec![self.expression(callee)];

    operands.extend(
      arguments
        .into_iter()
        .map(|arg| self.expression(arg))
        .collect::<Vec<_>>(),
    );

    self.emit(inst::Call::new(operands), ast_type)
  }

  #[inline]
  fn paren(&mut self, paren: se::Paren<'c>) -> ValueID {
    self.expression(paren.expr)
  }
}
impl<'c> Emitter<'c> {
  fn assignment(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator.category(), OperatorCategory::Assignment);
    assert!(lookup!(self, left).ir_type.is_pointer());

    let calculated_right = match operator.associated_operator() {
      Some(operator) => {
        let loaded_left =
          self.emit(inst::Memory::from(inst::Load::new(left)), ast_type);
        self.do_binary(operator, loaded_left, right, ast_type, span)
      },
      None => right,
    };
    self.emit(
      inst::Memory::Store(inst::Store::new(left, calculated_right)),
      self.ast().void_type(),
    )
  }

  fn logical(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use super::Type::*;

    // debug_assert!(
    //   RefEq::ref_eq(*ast_type, self.ast().converted_bool())
    //     || RefEq::ref_eq(*ast_type, self.ast().i1_bool_type()),
    // );

    let lhs_ir_type = lookup!(self, left).ir_type;
    let rhs_ir_type = lookup!(self, right).ir_type;
    match (lhs_ir_type, rhs_ir_type) {
      (Integer(left_width), Integer(right_width)) => {
        debug_assert!(
          left_width == right_width
            && (lookup!(self, left).ast_type.signedness()
              == lookup!(self, right).ast_type.signedness())
        );
        let signedness = lookup!(self, left)
          .ast_type
          .signedness()
          .expect("impossible to fail");
        self.integral_relational(
          operator, left, right, ast_type, signedness, span,
        )
      },
      (Floating(_), Floating(_)) =>
        self.floating_relational(operator, left, right, ast_type, span),
      (Pointer(), Integer(integer)) | (Integer(integer), Pointer()) =>
        panic!("this should be rejected or emit a warning"),
      (Pointer(), Pointer()) => self.emit(
        inst::ICmp::new(
          inst::ICmpPredicate::from_op_and_sign(operator, Signedness::Unsigned),
          left,
          right,
        ),
        ast_type,
      ),
      _ => unreachable!(),
    }
  }

  fn integral_relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    signedness: Signedness,
    span: SourceSpan,
  ) -> ValueID {
    self.emit(
      inst::ICmp::new(
        inst::ICmpPredicate::from_op_and_sign(operator, signedness),
        left,
        right,
      ),
      ast_type,
    )
  }

  fn floating_relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
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
    self.emit(inst::FCmp::new(predicate, left, right), ast_type)
  }

  fn arithmetic(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    use inst::BinaryOp;
    // currently only do integer arithmetic here
    let op = match operator {
      Plus => BinaryOp::Add,
      Minus => BinaryOp::Sub,
      Star => BinaryOp::Mul,
      // Slash => BinaryOp::SDiv,
      // Percent => BinaryOp::SRem,
      _ => todo!(),
    };
    self.emit(inst::Binary::new(op, left, right), ast_type)
  }

  fn bitwise(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    use inst::BinaryOp;
    let bitwise = match operator {
      Ampersand => BinaryOp::And,
      Pipe => BinaryOp::Or,
      Caret => BinaryOp::Xor,
      _ => unreachable!(),
    };
    self.emit(inst::Binary::new(bitwise, left, right), ast_type)
  }

  fn bitshift(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    use Signedness::*;
    use inst::BinaryOp::*;

    debug_assert!(
      lookup!(self, right)
        .ast_type
        .as_primitive()
        .is_some_and(|p| p.is_integer())
    );

    let bitshift = match (operator, lookup!(self, left).ast_type.signedness()) {
      (LeftShift, Some(_)) => Shl,
      (RightShift, Some(Signed)) => AShr,
      (RightShift, Some(Unsigned)) => LShr,
      _ => unreachable!(),
    };
    self.emit(inst::Binary::new(bitshift, left, right), ast_type)
  }

  #[inline]
  fn comma(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Comma);
    right
  }
}
impl<'c> Emitter<'c> {
  #[inline]
  fn addressof(
    &mut self,
    operator: Operator,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Ampersand);
    operand
  }

  fn indirect(
    &mut self,
    operator: Operator,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Star);
    assert!(lookup!(self, operand).ir_type.is_pointer());
    self.emit(inst::Memory::from(inst::Load::new(operand)), ast_type)
  }

  fn logical_not(
    &mut self,
    operator: Operator,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Not);

    // debug_assert!(
    //   RefEq::ref_eq(
    //     lookup!(self, operand).qualified_type.unqualified_type,
    //     self.ast().converted_bool()
    //   ) || RefEq::ref_eq(
    //     lookup!(self, operand).qualified_type.unqualified_type,
    //     self.ast().i1_bool_type()
    //   ),
    // );

    // FIXME: avoid this trick but also reuse the code below. borrowck wont let self being borrowed
    // SAFETY: safe today, maybe not tomorrow.
    let this = ::std::ptr::from_mut(self);

    let common = move |cmp| {
      let this = unsafe { &mut *this };
      let xor = this.emit(
        inst::Binary::new(inst::BinaryOp::Xor, cmp, this.ir().i1_true()),
        this.ast().i1_bool_type(),
      );
      this.emit(inst::Cast::from(inst::Zext::new(xor)), ast_type)
    };
    let integral = move |zero_or_nullptr| {
      let this = unsafe { &mut *this };
      let cmp = this.emit(
        inst::ICmp::new(inst::ICmpPredicate::Ne, operand, zero_or_nullptr),
        this.ast().i1_bool_type(),
      );
      common(cmp)
    };

    let cannot_inline_me_otherwise_refcell_panic =
      lookup!(self, operand).ir_type;

    match cannot_inline_me_otherwise_refcell_panic {
      super::Type::Pointer() => integral(self.ir().nullptr()),
      super::Type::Integer(width) => integral(self.ir().integer_zero(*width)),
      super::Type::Floating(format) => {
        let cmp = self.emit(
          inst::FCmp::new(
            inst::FCmpPredicate::Une,
            operand,
            self.ir().floating_zero(*format),
          ),
          self.ast().i1_bool_type(),
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
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Tilde);
    assert!(ast_type.as_primitive().is_some_and(|p| p.is_integer()));
    let bitmask = self.emit(
      Constant::Integral(Integral::bitmask(ast_type.size_bits() as u8)),
      ast_type,
    );
    self.emit(
      inst::Binary::new(inst::BinaryOp::Xor, operand, bitmask),
      ast_type,
    )
  }

  fn unary_arithmetic(
    &mut self,
    operator: Operator,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    match operator {
      Operator::Plus => operand,
      Operator::Minus => self.emit(
        inst::Binary::new(
          inst::BinaryOp::Sub,
          self
            .ir()
            .integer_zero(lookup!(self, operand).ast_type.size_bits() as u8),
          operand,
        ),
        ast_type,
      ),
      _ => unreachable!(),
    }
  }

  fn ppmm(
    &mut self,
    operator: Operator,
    operand: ValueID,
    kind: UnaryKind,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use Operator::*;
    use UnaryKind::*;
    use inst::BinaryOp::*;

    let value = lookup!(self, operand);
    // TODO: pointer is unimplemented
    debug_assert!(matches!(
      value.ir_type,
      super::Type::Pointer()
        | super::Type::Integer(_)
        | super::Type::Floating(_)
    ));
    debug_assert!(matches!(operator, PlusPlus | MinusMinus));

    let (width, _signedness) = (
      value.ast_type.size_bits() as u8,
      value.ast_type.signedness(),
    );

    let binaryop = match operator {
      PlusPlus => Add,
      MinusMinus => Sub,
      _ => unreachable!(),
    };
    match kind {
      Prefix => {
        let calculated = self.emit(
          inst::Binary::new(binaryop, operand, self.ir().integer_one(width)),
          ast_type,
        );
        _ = self.emit(
          inst::Memory::Store(inst::Store::new(operand, calculated)),
          self.ast().void_type(),
        );
        calculated
      },
      Postfix => {
        let calculated = self.emit(
          inst::Binary::new(binaryop, operand, self.ir().integer_one(width)),
          ast_type,
        );
        _ = self.emit(
          inst::Memory::Store(inst::Store::new(operand, calculated)),
          self.ast().void_type(),
        );
        operand
      },
    }
  }
}
