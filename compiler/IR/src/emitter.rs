#![allow(unused_variables)]
#![deny(unused_must_use)]
use ::rcc_adt::{Integral, Signedness};
use ::rcc_ast::{
  SymbolPtr, UnaryKind,
  types::{self as ast, TypeInfo},
};
use ::rcc_sema::{declaration as sd, expression as se, statement as ss};
use ::rcc_shared::{Constant, OpDiag, Operator, OperatorCategory, SourceSpan};
use ::rcc_utils::{RefEq, StrRef, Unbox, contract_violation};
use ::std::collections::HashMap;

use super::{
  Argument,
  context::{Session, SessionRef},
  emitable::Emitable,
  instruction::{self as inst},
  module::{self, BasicBlock, Module},
  value::{Value, ValueID, WithActionMut},
};
#[derive(Default, PartialEq, Eq)]
pub(super) struct ControlFlowContext {
  /// loops w/ switch
  pub(super) break_target: ValueID,
  /// [`None`] for switch, [`Some`] for loops
  pub(super) continue_target: Option<ValueID>,
}

impl ControlFlowContext {
  pub(super) fn new(
    break_target: ValueID,
    continue_target: Option<ValueID>,
  ) -> Self {
    Self {
      break_target,
      continue_target,
    }
  }
}
pub struct Emitter<'c> {
  pub(super) session: SessionRef<'c, OpDiag<'c>>,
  /// The basic block currently being written into
  pub(super) current_block: ValueID,
  /// Blocks finalized in the current function
  pub(super) current_function: ValueID,
  pub(super) ctrlflow_ctx: Vec<ControlFlowContext>,
  pub(super) labels: HashMap<StrRef<'c>, ValueID>,
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
      ctrlflow_ctx: Default::default(),
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
    use inst::*;

    use super::types::Type;

    let ir_type = lookup!(self, value_id).ir_type;
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
    if !self.current_block.is_null() {
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
    };
  }

  #[must_use]
  fn new_empty_block(&mut self) -> ValueID {
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
    self.current_block = self.new_empty_block();

    let declarations = translation_unit.declarations;

    self.module.globals = Vec::with_capacity(declarations.len());

    declarations
      .into_iter()
      .for_each(|declaration| self.global_decl(declaration));

    debug_assert!(self.current_function.is_null());
    debug_assert!(self.ctrlflow_ctx.is_empty());

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
    debug_assert!(function.is_definition());

    let sd::Function {
      symbol,
      parameters,
      body,
      gotos,
      labels,
      ..
    } = function;

    let function_name = symbol.borrow().name;
    let ast_type = symbol.borrow().qualified_type.unqualified_type;

    self.current_function = if let Some(&value_id) =
      self.globals.get(&(symbol.as_ptr() as *const _))
    {
      // should be function and declaration-only
      debug_assert!(
        !lookup!(self, value_id).data.as_function().is_some_and(|f| f
          .is_definition()
          && RefEq::ref_eq(
            function_name,
            lookup!(self, value_id).data.as_function_unchecked().name
          )
          && f.is_variadic
            == ast_type.as_functionproto_unchecked().is_variadic),
        "pre-registered function should be declaration-only"
      );
      value_id
    } else {
      let function_id = self.emit(
        module::Function::new_empty(
          function_name,
          Default::default(),
          ast_type.as_functionproto_unchecked().is_variadic,
        ),
        ast_type,
      );

      self
        .globals
        .insert(symbol.as_ptr() as *const _, function_id);
      debug_assert!(
        lookup!(self, function_id)
          .data
          .as_function()
          .is_some_and(|f| !f.is_definition()),
        "pre-registered function should be declaration-only"
      );
      function_id
    };

    assert!(self.locals.is_empty());
    assert!(self.labels.is_empty());

    self.apply(self.current_block, |val| {
      val.parent = self.current_function;
      let entry = val.data.as_basicblock_mut_unchecked();
      debug_assert!(entry.is_empty());
      entry.instructions.reserve((parameters.len() + 1) * 2 + 1);
    });

    let return_type = ast_type
      .as_functionproto_unchecked()
      .return_type
      .unqualified_type;

    if !return_type.is_void() {
      // return value storage
      let return_slot_id = self.emit(inst::Alloca::new(), return_type);
      let default_value_id =
        self.emit(return_type.default_value(), return_type);

      _ = self.emit(
        inst::Memory::Store(inst::Store::new(return_slot_id, default_value_id)),
        self.ast().void_type(),
      );
    }

    // insert params into the local scope and allocate spaces
    let params = parameters
      .into_iter()
      .enumerate()
      .map(|(index, parameter)| {
        let ast_type =
          parameter.symbol.borrow().qualified_type.unqualified_type;
        let arg_id = self.emit(Argument::new(index), ast_type);
        let localed_arg_id = self.emit(inst::Alloca::new(), ast_type);
        self
          .locals
          .insert(parameter.symbol.as_ptr(), localed_arg_id);
        _ = self.emit(
          inst::Memory::Store(inst::Store::new(localed_arg_id, arg_id)),
          self.ast().void_type(),
        );
        arg_id
      })
      .collect::<Vec<_>>();

    self.apply(self.current_function, |value| {
      value.data.as_function_mut_unchecked().params = params
    });

    self.compound(body.expect("Precondition: function.is_definition()"));

    let (has_inst, has_term) = self.visit(self.current_block, |value| {
      let block = value.data.as_basicblock_unchecked();
      (!block.instructions.is_empty(), !block.terminator.is_null())
    });

    let this = ::std::ptr::from_mut(self);

    let common = || {
      let this = unsafe { &mut *this };
      let next_function_entry = this.new_empty_block();
      let _ = this.push_block(next_function_entry);
    };
    // A | (B if B ...) this syntax is not supported.
    let make_unreachable_block = || {
      let this = unsafe { &mut *this };
      let _unreachable = this.emit(
        inst::Terminator::Unreachable(inst::Unreachable::new()),
        this.ast().void_type(),
      );
      common();
    };

    match (has_inst, has_term) {
      // if the current block has a terminator, push it and insert am empty one
      (_, true) => common(),
      // 5.1.2.3.4 Program termination
      // If [...], reaching the `}` that terminates the main function returns a value of 0.
      (_, false)
        if function_name == "main"
          && !self.ir().get_use_list(self.current_block).is_empty() =>
      {
        let _implicit_return = self.emit(
          inst::Terminator::Return(inst::Return::new(Some(
            self
              .ir()
              .integer_zero(self.ast().int_type().size_bits() as u8),
          ))),
          self.ast().int_type(),
        );
        common()
      },
      (_, false)
        if ast_type.as_functionproto_unchecked().return_type.is_void() =>
      // if the return type is void it may also be an implicit return void;
      // only when it has no users does it indicate an traling empty block.
        if !self.ir().get_use_list(self.current_block).is_empty()
        // or user didnt write anything
          || self.visit(self.current_function, |value|value.data.as_function_unchecked().blocks.is_empty())
        {
          let _implicit_return = self.emit(
            inst::Terminator::Return(inst::Return::new(None)),
            self.ast().void_type(),
          );
          common()
        },
      // if the current blobk is not empty but it does not have a terminator, insert an unreachable and take it.
      (true, false) => make_unreachable_block(),
      // if current block is not empty and is used by other blocks, it probably means the block just missing a terminator,
      (false, false)
        if !self.ir().get_use_list(self.current_block).is_empty() =>
        make_unreachable_block(),
      // if the current block is empty, and is not used by any other blocks,
      // it prob means the previous one is return stmt
      // and this one is just a trailing empty redundant block, leave as-is.
      (false, false) => (),
    }

    self.locals.clear();
    self.labels.clear();

    assert!(
      !lookup!(self, self.current_function)
        .data
        .as_function_unchecked()
        .entry()
        .is_null()
    );

    self.current_function = ValueID::null();
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
      Return(return_stmt) => self.return_stmt(return_stmt),
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

  fn return_stmt(&mut self, return_stmt: ss::Return<'c>) {
    let ss::Return { expression, .. } = return_stmt;
    let ast_type = expression
      .as_ref()
      .map(|e| e.unqualified_type())
      .unwrap_or(self.ast().void_type());
    let operand: Option<ValueID> = expression.map(|e| self.expression(e));
    let _ret_inst = self.emit(
      inst::Terminator::Return(inst::Return::new(operand)),
      ast_type,
    );
    let immediate_block_id = self.new_empty_block();
    let _should_be_now = self.push_block(immediate_block_id);
  }

  fn compound(&mut self, compound: ss::Compound<'c>) {
    let ss::Compound { statements, .. } = compound;
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
      // tag, // tag is needed for break and continue, now TODO.
      ..
    } = while_stmt;

    let now_block_id = self.current_block;

    let cond_block_id = self.new_empty_block();
    let body_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    self.ctrlflow_ctx.push(ControlFlowContext::new(
      immediate_block_id,
      Some(cond_block_id),
    ));

    let _now_block_terminator = self.emit(
      inst::Terminator::Jump(inst::Jump::new(cond_block_id)),
      self.ast().void_type(),
    );

    let should_be_now = self.push_block(cond_block_id);
    assert_eq!(should_be_now, now_block_id);

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let _cond_block_terminator = self.emit(
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

    let _body_block_terminator = {
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
    let _poped = self.ctrlflow_ctx.pop();
    debug_assert!(
      _poped.is_some_and(|_ctrl| _ctrl.break_target == immediate_block_id
        && _ctrl.continue_target == Some(cond_block_id))
    );
  }

  fn do_while(&mut self, do_while: ss::DoWhile<'c>) {
    let ss::DoWhile {
      condition,
      body,
      // tag,
      ..
    } = do_while;

    let now_block_id = self.current_block;

    let body_block_id = self.new_empty_block();
    let cond_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    self.ctrlflow_ctx.push(ControlFlowContext::new(
      immediate_block_id,
      Some(cond_block_id),
    ));

    let _now_block_terminator = self.emit(
      inst::Terminator::Jump(inst::Jump::new(body_block_id)),
      self.ast().void_type(),
    );

    let _should_be_now = self.push_block(cond_block_id);
    assert_eq!(_should_be_now, now_block_id);

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let _cond_block_terminator = self.emit(
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

    let _body_block_terminator = {
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
    let _poped = self.ctrlflow_ctx.pop();
    debug_assert!(
      _poped.is_some_and(|_ctrl| _ctrl.break_target == immediate_block_id
        && _ctrl.continue_target == Some(cond_block_id))
    );
  }

  fn for_stmt(&mut self, for_stmt: ss::For<'c>) {
    let ss::For {
      initializer,
      condition,
      increment,
      body,
      ..
    } = for_stmt;

    if let Some(statement) = initializer {
      self.statement(statement)
    }

    let now_block_id = self.current_block;
    let cond_block_id = self.new_empty_block();
    let increment_block_id = self.new_empty_block();
    let body_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    self.ctrlflow_ctx.push(ControlFlowContext::new(
      immediate_block_id,
      Some(increment_block_id), // < this is different than while and do-while
    ));

    let _now_block_terminator = self.emit(
      inst::Terminator::Jump(inst::Jump::new(cond_block_id)),
      self.ast().void_type(),
    );

    let _should_be_now = self.push_block(cond_block_id);
    assert_eq!(_should_be_now, now_block_id);

    let boolean_condition = condition
      .map(|cond| self.expression(cond))
      .map(|cond| self.contextual_convert_to_i1(cond))
      .unwrap_or_else(|| self.ir().i1_true()); // if condition is omitted, it is treated as true.
    let _cond_block_terminator = self.emit(
      inst::Terminator::Branch(inst::Branch::new(
        boolean_condition,
        body_block_id,
        immediate_block_id,
      )),
      self.ast().void_type(),
    );

    let _should_be_cond = self.push_block(body_block_id);
    assert_eq!(_should_be_cond, cond_block_id);

    self.statement(body);

    let _body_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;
      terminator.unwrap_or_else(|| {
        self.emit_terminator(
          inst::Jump::new(increment_block_id),
          self.ast().void_type(),
          self.current_block,
        )
      })
    };

    let _last_block_of_body = self.push_block(increment_block_id);

    if let Some(increment) = increment {
      self.expression(increment);
    }
    let _inc_block_terminator = self.emit(
      inst::Terminator::Jump(inst::Jump::new(cond_block_id)),
      self.ast().void_type(),
    );

    let _should_be_inc = self.push_block(immediate_block_id);
    assert_eq!(_should_be_inc, increment_block_id);

    let _poped = self.ctrlflow_ctx.pop();
    debug_assert!(
      _poped.is_some_and(|_ctrl| _ctrl.break_target == immediate_block_id
        && _ctrl.continue_target == Some(increment_block_id))
    );
  }

  fn switch(&self, switch: ss::Switch<'c>) {
    todo!()
  }

  fn goto(&self, goto: ss::Goto<'c>) {
    todo!()
  }

  fn label(&mut self, label: ss::Label<'c>) {
    todo!()
  }

  fn break_stmt(&mut self, break_stmt: ss::Break<'c>) {
    let ss::Break { .. } = break_stmt;

    let target_block_id = self
      .ctrlflow_ctx
      .last()
      .map(|ctrl| ctrl.break_target)
      .unwrap_or_else(|| {
        panic!(
          "break statement not within a loop or switch. this should have been \
           caught in semantic checks."
        )
      });

    let now_block_id = self.current_block;

    let _break_inst_id = self.emit(
      inst::Terminator::Jump(inst::Jump::new(target_block_id)),
      self.ast().void_type(),
    );

    let immediate_block_id = self.new_empty_block();
    let _should_be_now = self.push_block(immediate_block_id);
    debug_assert_eq!(now_block_id, _should_be_now);
  }

  fn continue_stmt(&mut self, continue_stmt: ss::Continue<'c>) {
    let ss::Continue { .. } = continue_stmt;

    let target_block_id = self
      .ctrlflow_ctx
      .iter()
      .rev()
      .find_map(|ctrl| ctrl.continue_target)
      .unwrap_or_else(|| {
        panic!(
          "continue statement not within a loop or switch. this should have \
           been caught in semantic checks."
        )
      });

    let now_block_id = self.current_block;

    let _continue_inst_id = self.emit(
      inst::Terminator::Jump(inst::Jump::new(target_block_id)),
      self.ast().void_type(),
    );

    let immediate_block_id = self.new_empty_block();
    let _should_be_now = self.push_block(immediate_block_id);
    debug_assert_eq!(now_block_id, _should_be_now);
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
      CompoundAssign(compound_assign) =>
        self.compound_assign(compound_assign, unqualified_type),
    }
  }

  fn constant(
    &mut self,
    constant: se::Constant<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit(constant.inner, ast_type)
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
      span,
    } = ternary;
    debug_assert_eq!(then_expr.qualified_type(), else_expr.qualified_type());
    // type res; if (cond) { res = then; } else { res = else; }

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let now_block_id = self.current_block;
    let then_block_id = self.new_empty_block();
    let else_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    let _now_block_terminator = self.emit(
      inst::Terminator::Branch(inst::Branch::new(
        condition,
        then_block_id,
        else_block_id,
      )),
      self.ast().void_type(),
    );

    let _should_be_now = self.push_block(then_block_id);
    debug_assert_eq!(_should_be_now, now_block_id);

    let then_id = self.expression(then_expr);

    let _then_block_terminator = {
      assert!(
        lookup!(self, self.current_block)
          .data
          .as_basicblock_unchecked()
          .terminator
          .is_null()
      );

      self.emit(
        inst::Terminator::Jump(inst::Jump::new(immediate_block_id)),
        self.ast().void_type(),
      )
    };

    let _last_block_of_then = self.push_block(else_block_id);

    let else_id = self.expression(else_expr);

    let _else_block_terminator = {
      assert!(
        lookup!(self, self.current_block)
          .data
          .as_basicblock_unchecked()
          .terminator
          .is_null()
      );
      self.emit(
        inst::Terminator::Jump(inst::Jump::new(immediate_block_id)),
        self.ast().void_type(),
      )
    };

    let _last_block_of_else = self.push_block(immediate_block_id);

    self.emit(
      inst::Phi::new(vec![then_id, then_block_id, else_id, else_block_id]),
      ast_type,
    )
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

  fn implicit_cast(
    &mut self,
    implicit_cast: se::ImplicitCast<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let se::ImplicitCast {
      cast_type, expr, ..
    } = implicit_cast;

    let operand = self.expression(expr);
    self.do_cast(operand, cast_type, ast_type)
  }

  fn do_cast(
    &mut self,
    operand: ValueID,
    cast_type: ast::CastType,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    use ::std::cmp::Ordering::*;
    use Signedness::*;
    use ast::CastType::*;
    use inst::{Cast, Load, Memory, Sext, Trunc, Zext};

    match cast_type {
      Noop | FunctionToPointerDecay | ArrayToPointerDecay => operand,
      LValueToRValue => self.emit(Memory::from(Load::new(operand)), ast_type),
      IntegralCast => {
        assert!(
          ast_type.as_primitive().is_some_and(|p| p.is_integer())
            && ast_type.size_bits() > 0
            && ast_type.size_bits() <= 128
        );
        match Ord::cmp(
          lookup!(self, operand).ir_type.as_integer_unchecked(),
          &(ast_type.size_bits() as u8),
        ) {
          Less => match ast_type.signedness() {
            Some(Signed) => self.emit(Cast::from(Sext::new(operand)), ast_type),
            Some(Unsigned) =>
              self.emit(Cast::from(Zext::new(operand)), ast_type),
            None => unreachable!(),
          },
          Equal => operand,
          Greater => self.emit(Cast::from(Trunc::new(operand)), ast_type),
        }
      },
      _ => todo!("implicit cast: {:?}", cast_type),
    }
  }

  fn compound_assign(
    &mut self,
    compound_assign: se::CompoundAssign,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let se::CompoundAssign {
      operator,
      left,
      right,
      intermediate_result_type,
      ..
    } = compound_assign;
    todo!()
  }
}
impl<'c> Emitter<'c> {
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

  fn assignment(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    assert_eq!(operator, Operator::Assign);
    assert!(lookup!(self, left).ir_type.is_pointer());

    self.emit(
      inst::Memory::Store(inst::Store::new(left, right)),
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
    match operator {
      // A && B -> if(A) { B } else { 0 }
      Operator::And => todo!(),
      // A || B -> if(A) { 1 } else { B }
      Operator::Or => todo!(),
      _ => unreachable!(),
    }
  }

  fn arithmetic(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use inst::{Binary, BinaryOp};
    let lhs_ty = self.visit(left, |lhs| lhs.ast_type);
    let rhs_ty = self.visit(right, |rhs| rhs.ast_type);

    debug_assert_eq!(lhs_ty.signedness(), rhs_ty.signedness());
    let singedness = lhs_ty.signedness().unwrap();

    let this = ::std::ptr::from_mut(self);

    let sizeof = |pointer_type: &ast::Type<'_>| {
      let this = unsafe { &mut *this };
      this.emit(
        Constant::Integral(Integral::from_uintptr(
          pointer_type.as_pointer_unchecked().pointee.size(),
        )),
        this.ast().uintptr_type(),
      )
    };

    match (lhs_ty.is_pointer(), rhs_ty.is_pointer()) {
      (false, false) => self.emit(
        Binary::new(
          BinaryOp::from_op_and_sign(operator, singedness)
            .expect("semantic analysis should catch this."),
          left,
          right,
        ),
        ast_type,
      ),
      // SHOULD USE GEP.
      (true, true) => {
        debug_assert_eq!(operator, Operator::Minus);
        use ast::Compatibility;
        debug_assert!(Compatibility::compatible(
          &lhs_ty.as_pointer_unchecked().pointee,
          &rhs_ty.as_pointer_unchecked().pointee
        ));
        debug_assert!(RefEq::ref_eq(ast_type, self.ast().ptrdiff_type()));

        let sizeof_id = sizeof(lhs_ty);
        let offset = self.emit(
          inst::Binary::new(inst::BinaryOp::Sub, left, right),
          ast_type, // self.ast().ptrdiff_type()
        );

        self.emit(
          inst::Binary::new(inst::BinaryOp::SDiv, offset, sizeof_id),
          ast_type,
        )
      },
      (true, false) => {
        use ::std::debug_assert_matches;
        debug_assert_matches!(operator, Operator::Plus | Operator::Minus);
        debug_assert!(RefEq::ref_eq(ast_type, lhs_ty));
        todo!()
      },
      (false, true) => {
        debug_assert_eq!(operator, Operator::Plus);
        debug_assert!(RefEq::ref_eq(ast_type, rhs_ty));

        let sizeof_id = sizeof(rhs_ty);
        let multiplied = self.emit(
          Binary::new(BinaryOp::Mul, left, sizeof_id),
          self.ast().ptrdiff_type(),
        );
        todo!()
      },
    }
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
  fn relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use super::Type::*;

    let lhs_ir_type = lookup!(self, left).ir_type;
    let rhs_ir_type = lookup!(self, right).ir_type;

    match (lhs_ir_type, rhs_ir_type) {
      (Integer(left_width), Integer(right_width)) =>
        self.integral_relational(operator, left, right, ast_type, span),
      (Floating(_), Floating(_)) =>
        self.floating_relational(operator, left, right, ast_type, span),
      (Pointer(), Pointer()) =>
        self.pointer_relational(operator, left, right, ast_type, span),

      _ => unreachable!(),
    }
  }

  fn integral_relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(
      self.visit(left, |value| {
        (
          value.ast_type.as_primitive_unchecked().is_integer(),
          value.ast_type.size_bits(),
          value.ast_type.signedness().unwrap(),
        )
      }),
      self.visit(right, |value| {
        (
          value.ast_type.as_primitive_unchecked().is_integer(),
          value.ast_type.size_bits(),
          value.ast_type.signedness().unwrap(),
        )
      }),
    );
    let signedness = self
      .visit(left, |value| value.ast_type.signedness())
      .expect("integer always have signedness");

    self.do_integral_relational(
      inst::ICmpPredicate::from_op_and_sign(operator, signedness),
      left,
      right,
      ast_type,
      span,
    )
  }

  fn do_integral_relational(
    &mut self,
    operator: inst::ICmpPredicate,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    self.emit(inst::ICmp::new(operator, left, right), ast_type)
  }

  fn pointer_relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    self.do_pointer_relational(
      inst::ICmpPredicate::from_op_and_sign(operator, Signedness::Unsigned),
      left,
      right,
      ast_type,
      span,
    )
  }

  fn do_pointer_relational(
    &mut self,
    operator: inst::ICmpPredicate,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    self.emit(inst::ICmp::new(operator, left, right), ast_type)
  }

  fn floating_relational(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    self.do_floating_relational(
      inst::FCmpPredicate::from_op(operator),
      left,
      right,
      ast_type,
      span,
    )
  }

  fn do_floating_relational(
    &mut self,
    operator: inst::FCmpPredicate,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    self.emit(inst::FCmp::new(operator, left, right), ast_type)
  }
}
impl<'c> Emitter<'c> {
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

    self.do_unary(operator, operand, kind, ast_type, span)
  }

  fn do_unary(
    &mut self,
    operator: Operator,
    operand: ValueID,
    kind: UnaryKind,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    macro_rules! call {
      ($method:ident) => {
        self.$method(operator, operand, ast_type, span)
      };
    }

    use Operator::*;
    match operator {
      Ampersand => call!(addressof),
      Star => call!(indirect),
      Not => call!(logical_not),
      Tilde => call!(tilde),
      Plus | Minus => call!(unary_arithmetic),
      PlusPlus | MinusMinus => match kind {
        UnaryKind::Prefix => call!(ppmm_pre),
        UnaryKind::Postfix => call!(ppmm_post),
      },
      _ => unreachable!("operator is not unary: {:#?}", operator),
    }
  }

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

    let ir_type = self.visit(operand, |value| value.ir_type);

    use super::Type::*;
    match ir_type {
      Pointer() => self.do_pointer_relational(
        inst::ICmpPredicate::Ne,
        operand,
        self.ir().nullptr(),
        ast_type,
        span,
      ),
      Integer(width) => self.do_integral_relational(
        inst::ICmpPredicate::Ne,
        operand,
        self.ir().integer_zero(*width),
        ast_type,
        span,
      ),
      Floating(format) => self.do_floating_relational(
        inst::FCmpPredicate::Une,
        operand,
        self.ir().floating_zero(*format),
        ast_type,
        span,
      ),
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
    let is_floating = self.visit(operand, |value| value.ir_type.is_floating());
    match (operator, is_floating) {
      (Operator::Plus, _) => operand,
      (Operator::Minus, false) => self.emit(
        inst::Binary::new(
          inst::BinaryOp::Sub,
          self
            .ir()
            .integer_zero(lookup!(self, operand).ast_type.size_bits() as u8),
          operand,
        ),
        ast_type,
      ),
      (Operator::Minus, true) =>
        self.emit(inst::Unary::new(inst::UnaryOp::FNeg, operand), ast_type),
      _ => unreachable!(),
    }
  }

  fn ppmm_pre(
    &mut self,
    operator: Operator,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn ppmm_post(
    &mut self,
    operator: Operator,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    todo!()
  }
}
