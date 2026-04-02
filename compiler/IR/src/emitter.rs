use ::rcc_adt::{Integral, Signedness};
use ::rcc_ast::{
  Constant, UnaryKind,
  types::{self as ast, TypeInfo},
};
use ::rcc_sema::{declaration as sd, expression as se, statement as ss};
use ::rcc_shared::{OpDiag, Operator, OperatorCategory, SourceSpan};
use ::rcc_utils::{RefEq, StrRef, contract_violation};
use ::std::collections::HashMap;

use super::{
  Argument,
  context::{Session, SessionRef},
  emitable::Emitable,
  instruction::{self as inst},
  module::{self, BasicBlock, Module},
  value::{Value, ValueID},
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
  pub(super) locals: HashMap<sd::DeclRef<'c>, ValueID>,
  /// function name → ValueID for call resolution
  pub(super) globals: HashMap<sd::DeclRef<'c>, ValueID>,
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

    use super::types::Type::*;

    let ir_type = lookup!(self, value_id).ir_type;
    match ir_type {
      Void() | Label() | Struct(_) | Array(_) | Function(_) => unreachable!(),
      Pointer() => self.emit(
        ICmp::new(ICmpPredicate::Ne, value_id, self.ir().nullptr()),
        self.ast().i1_bool_type(),
      ),
      Floating(format) => self.emit(
        FCmp::new(
          FCmpPredicate::Une,
          value_id,
          self.ir().floating_zero(*format),
        ),
        self.ast().i1_bool_type(),
      ),
      Integer(1u8) => value_id,
      Integer(width) => self.emit(
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
      debug_assert!(
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
    self.apply(branch_id, |now| {
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
    self.apply(jump_id, |jump| {
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
      .iter()
      .for_each(|declaration| self.global_decl(declaration));

    debug_assert!(self.current_function.is_null());
    debug_assert!(self.ctrlflow_ctx.is_empty());

    self.module
  }
}

impl<'c> Emitter<'c> {
  fn global_decl(&mut self, declaration: &sd::ExternalDeclarationRef<'c>) {
    match declaration {
      sd::ExternalDeclarationRef::Function(function) =>
        match function.is_definition() {
          true => self.global_funcdef(function),
          false => self.funcdecl(function.declaration),
        },
      sd::ExternalDeclarationRef::Variable(variable) => {
        self.global_vardef(variable);
      },
    }
  }

  fn funcdecl(&mut self, declaration: sd::DeclRef<'c>) {
    let declaration = declaration.canonical_decl();
    if let Some(&value_id) = self.globals.get(&declaration) {
      debug_assert!(
        lookup!(self, value_id).data.is_function(),
        "pre-registered value should be a function"
      );
    } else {
      let decl = declaration;
      let name = decl.name();
      let ast_type = decl.qualified_type().unqualified_type;
      let is_variadic = ast_type.as_functionproto_unchecked().is_variadic;

      let value_id = self.emit(
        module::Function::new_empty(name, Default::default(), is_variadic),
        ast_type,
      );

      self.globals.insert(declaration, value_id);
    }
  }

  fn global_funcdef(&mut self, function: sd::FunctionRef<'c>) {
    debug_assert!(function.is_definition());

    let declaration = function.declaration.canonical_decl();
    let parameters = function.parameters;

    let function_name = declaration.name();
    let ast_type = declaration.qualified_type().unqualified_type;

    self.current_function =
      if let Some(&value_id) = self.globals.get(&declaration) {
        // should be function and declaration-only
        debug_assert!(
          !lookup!(self, value_id).data.as_function().is_some_and(|f| f
            .is_definition()
            && RefEq::ref_eq(
              &function_name,
              &lookup!(self, value_id).data.as_function_unchecked().name
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

        self.globals.insert(declaration, function_id);
        debug_assert!(
          lookup!(self, function_id)
            .data
            .as_function()
            .is_some_and(|f| !f.is_definition()),
          "pre-registered function should be declaration-only"
        );
        function_id
      };

    debug_assert!(self.locals.is_empty());
    debug_assert!(self.labels.is_empty());

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
        inst::Store::new(return_slot_id, default_value_id),
        self.ast().void_type(),
      );
    }

    // insert params into the local scope and allocate spaces
    let params = parameters
      .iter()
      .enumerate()
      .map(|(index, parameter)| {
        let declaration = parameter.declaration;
        let ast_type = declaration.qualified_type().unqualified_type;
        let arg_id = self.emit(Argument::new(index), ast_type);
        let localed_arg_id = self.emit(inst::Alloca::new(), ast_type);
        self.locals.insert(declaration, localed_arg_id);
        _ = self.emit(
          inst::Store::new(localed_arg_id, arg_id),
          self.ast().void_type(),
        );
        arg_id
      })
      .collect::<Vec<_>>();

    self.apply(self.current_function, |value| {
      value.data.as_function_mut_unchecked().params = params
    });

    self.compound(
      function
        .body
        .as_ref()
        .expect("Precondition: function.is_definition()"),
    );

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
      let _unreachable =
        this.emit(inst::Unreachable::new(), this.ast().void_type());
      common();
    };

    match (has_inst, has_term) {
      // if the current block has a terminator, push it and insert am empty one
      (_, true) => common(),
      // 5.1.2.3.4 Program termination
      // If [...], reaching the `}` that terminates the main function returns a value of 0.
      (_, false)
        if function_name == "main"
          // && !self.ir().get_use_list(self.current_block).is_empty()
           =>
      {
        let _implicit_return = self.emit(
          inst::Return::new(Some(
            self
              .ir()
              .integer_zero(self.ast().int_type().size_bits() as u8),
          )),
          self.ast().void_type(),
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
            inst::Return::new(None),
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

    debug_assert!(
      !lookup!(self, self.current_function)
        .data
        .as_function_unchecked()
        .entry()
        .is_null()
    );

    self.current_function = ValueID::null();
  }

  fn global_vardef(&mut self, variable: sd::VarDefRef<'c>) {
    let declaration = variable.declaration.canonical_decl();

    let initializer = match variable.initializer {
      Some(sd::Initializer::Scalar(expr)) => Some(module::Initializer::Scalar(
        expr.raw_expr().as_constant_unchecked().clone(),
      )),
      Some(sd::Initializer::Aggregate(_)) => todo!(),
      None => None,
    };
    let value_id = self.emit(
      module::Variable::new(
        declaration.name(),
        initializer, // TODO: handle initializers
      ),
      declaration.qualified_type().unqualified_type,
    );
    self.globals.insert(declaration, value_id);
  }

  fn local_decl(&mut self, declaration: &sd::ExternalDeclarationRef<'c>) {
    debug_assert!(!self.current_block.is_null());
    match declaration {
      sd::ExternalDeclarationRef::Function(function_decl) => {
        self.funcdecl(function_decl.declaration);
      },
      sd::ExternalDeclarationRef::Variable(var_def) =>
        self.local_vardef(var_def),
    }
  }

  fn local_vardef(&mut self, var_def: sd::VarDefRef<'c>) {
    let declaration = var_def.declaration;
    let value_id = self.emit(
      inst::Alloca::new(),
      declaration.qualified_type().unqualified_type,
    );

    match var_def.initializer {
      Some(sd::Initializer::Scalar(expr)) => {
        let init_value_id = self.expression(expr);
        _ = self.emit(
          inst::Store::new(value_id, init_value_id),
          self.ast().void_type(),
        );
      },
      Some(sd::Initializer::Aggregate(_)) => todo!(),
      None => (),
    };
    self.locals.insert(declaration, value_id);
  }
}

impl<'c> Emitter<'c> {
  fn statement(&mut self, statement: ss::StmtRef<'c>) {
    use ss::Statement::*;
    match statement {
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
  fn exprstmt(&mut self, expression: se::ExprRef<'c>) {
    self.expression(expression);
  }

  fn return_stmt(&mut self, return_stmt: &ss::Return<'c>) {
    let ss::Return { expression, .. } = return_stmt;
    // let ast_type = expression
    //   .as_ref()
    //   .map(|e| e.unqualified_type())
    //   .unwrap_or(self.ast().void_type());
    let operand: Option<ValueID> = expression.map(|e| self.expression(e));
    let _ret_inst =
      self.emit(inst::Return::new(operand), self.ast().void_type());
    let immediate_block_id = self.new_empty_block();
    let _should_be_now = self.push_block(immediate_block_id);
  }

  fn compound(&mut self, compound: &ss::Compound<'c>) {
    let ss::Compound { statements, .. } = compound;
    statements
      .iter()
      .for_each(|statement| self.statement(statement));
  }

  fn if_stmt(&mut self, if_stmt: &ss::If<'c>) {
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
      inst::Branch::new(condition, ValueID::null(), ValueID::null()),
      self.ast().void_type(),
    );

    let then_block_id = self.new_empty_block();
    let else_block_id = self.new_empty_block();

    let should_be_now = self.push_block(then_block_id);
    debug_assert_eq!(should_be_now, now_block_id);

    self.statement(then_branch);

    let then_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;

      terminator.unwrap_or_else(|| {
        self.emit(inst::Jump::new(ValueID::null()), self.ast().void_type())
      })
    };

    // the assertion here is wrong. new controlflow may add many blocks.
    // let shuold_be_then = self.push_block(else_block_id);
    // debug_assert_eq!(shuold_be_then, then_block_id);

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
          self.emit(inst::Jump::new(ValueID::null()), self.ast().void_type())
        })
      })
      .unwrap_or_default()
      .and_then(|else_block_terminator| {
        let immediate_block_id = self.new_empty_block();

        // ditto
        let _last_block_of_else = self.push_block(immediate_block_id);
        // debug_assert_eq!(should_be_else, else_block_id);

        self.refill_jump(then_block_terminator, immediate_block_id);
        self.refill_jump(else_block_terminator, immediate_block_id)
      })
      .or_else(|| self.refill_jump(then_block_terminator, else_block_id));
  }

  fn while_stmt(&mut self, while_stmt: &ss::While<'c>) {
    let ss::While {
      condition, body, ..
    } = while_stmt;

    let now_block_id = self.current_block;

    let cond_block_id = self.new_empty_block();
    let body_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    self.ctrlflow_ctx.push(ControlFlowContext::new(
      immediate_block_id,
      Some(cond_block_id),
    ));

    let _now_block_terminator =
      self.emit(inst::Jump::new(cond_block_id), self.ast().void_type());

    let should_be_now = self.push_block(cond_block_id);
    debug_assert_eq!(should_be_now, now_block_id);

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let _cond_block_terminator = self.emit(
      inst::Branch::new(condition, body_block_id, immediate_block_id),
      self.ast().void_type(),
    );

    let should_be_cond = self.push_block(body_block_id);
    debug_assert_eq!(should_be_cond, cond_block_id);

    self.statement(body);

    let _body_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;
      terminator.unwrap_or_else(|| {
        self.emit(inst::Jump::new(cond_block_id), self.ast().void_type())
      })
    };

    let _last_block_of_body = self.push_block(immediate_block_id);
    let _poped = self.ctrlflow_ctx.pop();
    debug_assert!(
      _poped.is_some_and(|_ctrl| _ctrl.break_target == immediate_block_id
        && _ctrl.continue_target == Some(cond_block_id))
    );
  }

  fn do_while(&mut self, do_while: &ss::DoWhile<'c>) {
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

    let _now_block_terminator =
      self.emit(inst::Jump::new(body_block_id), self.ast().void_type());

    let _should_be_now = self.push_block(cond_block_id);
    debug_assert_eq!(_should_be_now, now_block_id);

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let _cond_block_terminator = self.emit(
      inst::Branch::new(condition, body_block_id, immediate_block_id),
      self.ast().void_type(),
    );

    let _should_be_cond = self.push_block(body_block_id);
    debug_assert_eq!(_should_be_cond, cond_block_id);

    self.statement(body);

    let _body_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;
      terminator.unwrap_or_else(|| {
        self.emit(inst::Jump::new(cond_block_id), self.ast().void_type())
      })
    };

    let _last_block_of_body = self.push_block(immediate_block_id);
    let _poped = self.ctrlflow_ctx.pop();
    debug_assert!(
      _poped.is_some_and(|_ctrl| _ctrl.break_target == immediate_block_id
        && _ctrl.continue_target == Some(cond_block_id))
    );
  }

  fn for_stmt(&mut self, for_stmt: &ss::For<'c>) {
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

    let _now_block_terminator =
      self.emit(inst::Jump::new(cond_block_id), self.ast().void_type());

    let _should_be_now = self.push_block(cond_block_id);
    debug_assert_eq!(_should_be_now, now_block_id);

    let boolean_condition = condition
      .map(|cond| self.expression(cond))
      .map(|cond| self.contextual_convert_to_i1(cond))
      .unwrap_or_else(|| self.ir().i1_true()); // if condition is omitted, it is treated as true.
    let _cond_block_terminator = self.emit(
      inst::Branch::new(boolean_condition, body_block_id, immediate_block_id),
      self.ast().void_type(),
    );

    let _should_be_cond = self.push_block(body_block_id);
    debug_assert_eq!(_should_be_cond, cond_block_id);

    self.statement(body);

    let _body_block_terminator = {
      let terminator = lookup!(self, self.current_block)
        .data
        .as_basicblock_unchecked()
        .terminator;
      terminator.unwrap_or_else(|| {
        self.emit(inst::Jump::new(increment_block_id), self.ast().void_type())
      })
    };

    let _last_block_of_body = self.push_block(increment_block_id);

    if let Some(increment) = increment {
      self.expression(increment);
    }
    let _inc_block_terminator =
      self.emit(inst::Jump::new(cond_block_id), self.ast().void_type());

    let _should_be_inc = self.push_block(immediate_block_id);
    debug_assert_eq!(_should_be_inc, increment_block_id);

    let _poped = self.ctrlflow_ctx.pop();
    debug_assert!(
      _poped.is_some_and(|_ctrl| _ctrl.break_target == immediate_block_id
        && _ctrl.continue_target == Some(increment_block_id))
    );
  }

  fn switch(&self, switch: &ss::Switch<'c>) {
    todo!("{switch:#?}")
  }

  fn goto(&self, goto: &ss::Goto<'c>) {
    todo!("{goto:#?}")
  }

  fn label(&mut self, label: &ss::Label<'c>) {
    todo!("{label:#?}")
  }

  fn break_stmt(&mut self, break_stmt: &ss::Break) {
    let ss::Break { .. } = break_stmt;

    let target_block_id = self
      .ctrlflow_ctx
      .last()
      .map(|ctrl| ctrl.break_target)
      .expect(
        "break statement not within a loop or switch. this should have been \
         caught in semantic checks.",
      );

    let now_block_id = self.current_block;

    let _break_inst_id =
      self.emit(inst::Jump::new(target_block_id), self.ast().void_type());

    let immediate_block_id = self.new_empty_block();
    let _should_be_now = self.push_block(immediate_block_id);
    debug_assert_eq!(now_block_id, _should_be_now);
  }

  fn continue_stmt(&mut self, continue_stmt: &ss::Continue) {
    let ss::Continue { .. } = continue_stmt;

    let target_block_id = self
      .ctrlflow_ctx
      .iter()
      .rev()
      .find_map(|ctrl| ctrl.continue_target)
      .expect(
        "continue statement not within a loop or switch. this should have \
         been caught in semantic checks.",
      );

    let now_block_id = self.current_block;

    let _continue_inst_id =
      self.emit(inst::Jump::new(target_block_id), self.ast().void_type());

    let immediate_block_id = self.new_empty_block();
    let _should_be_now = self.push_block(immediate_block_id);
    debug_assert_eq!(now_block_id, _should_be_now);
  }
}
impl<'c> Emitter<'c> {
  fn expression(&mut self, expression: se::ExprRef<'c>) -> ValueID {
    // the fold here contains partial fold. e.g. `3 + 6 + func(4 + 5)` would be folded to `9 + func(9)`.
    let expression = expression.fold(&self.session().as_ast_session()).take();
    let unqualified_type = expression.unqualified_type();
    let span = expression.span();
    use se::RawExpr::*;
    match expression.raw_expr() {
      Empty(_) => contract_violation!(
        "empty expr is used in sema for error recovery. shouldnt reach here."
      ),
      Constant(constant) => self.constant(constant, unqualified_type, span),
      Unary(unary) => self.unary(unary, unqualified_type, span),
      Binary(binary) => self.binary(binary, unqualified_type, span),
      Call(call) => self.call(call, unqualified_type, span),
      Paren(paren) => self.paren(paren, span),
      MemberAccess(member_access) =>
        self.member_access(member_access, unqualified_type, span),
      Ternary(ternary) => self.ternary(ternary, unqualified_type, span),
      SizeOf(size_of) => self.sizeof(size_of, unqualified_type, span),
      CStyleCast(cstyle_cast) =>
        self.cstyle_cast(cstyle_cast, unqualified_type, span),
      ArraySubscript(array_subscript) =>
        self.array_subscript(array_subscript, unqualified_type, span),
      CompoundLiteral(compound_literal) =>
        self.compound_literal(compound_literal, unqualified_type, span),
      Variable(variable) => self.variable(variable, unqualified_type, span),
      ImplicitCast(implicit_cast) =>
        self.implicit_cast(implicit_cast, unqualified_type, span),
      CompoundAssign(compound_assign) =>
        self.compound_assign(compound_assign, unqualified_type, span),
    }
  }

  fn constant(
    &mut self,
    constant: &se::Constant<'c>,
    ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    self.emit(constant.clone(), ast_type)
  }

  fn member_access(
    &mut self,
    _member_access: &se::MemberAccess<'c>,
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn ternary(
    &mut self,
    ternary: &se::Ternary<'c>,
    ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    let se::Ternary {
      condition,
      then_expr,
      else_expr,
      ..
    } = ternary;
    let then_expr = (*then_expr).expect("unimplemened for ?:");
    debug_assert_eq!(then_expr.qualified_type(), else_expr.qualified_type());
    // type res; if (cond) { res = then; } else { res = else; }

    let boolean_condition = self.expression(condition);
    let condition = self.contextual_convert_to_i1(boolean_condition);

    let now_block_id = self.current_block;
    let then_block_id = self.new_empty_block();
    let else_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    let _now_block_terminator = self.emit(
      inst::Branch::new(condition, then_block_id, else_block_id),
      self.ast().void_type(),
    );

    let _should_be_now = self.push_block(then_block_id);
    debug_assert_eq!(_should_be_now, now_block_id);

    let then_id = self.expression(then_expr);

    let _then_block_terminator = {
      debug_assert!(
        lookup!(self, self.current_block)
          .data
          .as_basicblock_unchecked()
          .terminator
          .is_null()
      );

      self.emit(inst::Jump::new(immediate_block_id), self.ast().void_type())
    };

    let _last_block_of_then = self.push_block(else_block_id);

    let else_id = self.expression(else_expr);

    let _else_block_terminator = {
      debug_assert!(
        lookup!(self, self.current_block)
          .data
          .as_basicblock_unchecked()
          .terminator
          .is_null()
      );
      self.emit(inst::Jump::new(immediate_block_id), self.ast().void_type())
    };

    let _last_block_of_else = self.push_block(immediate_block_id);

    self.emit(
      inst::Phi::new(vec![then_id, then_block_id, else_id, else_block_id]),
      ast_type,
    )
  }

  fn sizeof(
    &mut self,
    size_of: &se::SizeOf<'c>,
    ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
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
    _cstyle_cast: &se::CStyleCast<'c>,
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn array_subscript(
    &mut self,
    array_subscript: &se::ArraySubscript<'c>,
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    let se::ArraySubscript { array, index, .. } = array_subscript;
    debug_assert!(
      array.qualified_type().is_pointer()
        && index.qualified_type().is_integer(),
      "precond: array subscript should have been checked to have a pointer \
       type and an integer type. if the index hand is pointer while the array \
       hand is integer, Sema has already swapped them to make it a pointer \
       subscript."
    );
    // let current_extent = array.unqualified_type().extent();
    // let target_extent = ast_type.extent();
    let pointer_ty = array.unqualified_type();

    let array_id = self.expression(array);
    let raw_index_id = self.expression(index);

    // assume sema checked that the target extent type is contained within the current extent and the type is valid..

    let index_id = self.integral_cast(raw_index_id, self.ast().ptrdiff_type());

    self.emit(
      inst::GetElementPtr::new(vec![array_id, index_id]),
      pointer_ty,
    )
  }

  fn compound_literal(
    &mut self,
    _compound_literal: &se::CompoundLiteral,
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    todo!()
  }

  fn variable(
    &self,
    variable: &se::Variable<'c>,
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    let declaration = variable.declaration.canonical_decl();
    let name = declaration.name();
    if let Some(&vid) = self.locals.get(&declaration) {
      vid
    } else if let Some(&vid) = self.globals.get(&declaration) {
      vid
    } else {
      panic!("undefined variable: {name}")
    }
  }

  fn call(
    &mut self,
    call: &se::Call<'c>,
    ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    let se::Call {
      callee, arguments, ..
    } = call;

    let mut operands = vec![self.expression(callee)];

    operands.extend(
      arguments
        .iter()
        .copied()
        .map(|arg| self.expression(arg))
        .collect::<Vec<_>>(),
    );

    self.emit(inst::Call::new(operands), ast_type)
  }

  #[inline]
  fn paren(&mut self, paren: &se::Paren<'c>, _span: SourceSpan) -> ValueID {
    self.expression(paren.expr)
  }

  fn implicit_cast(
    &mut self,
    implicit_cast: &se::ImplicitCast<'c>,
    ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    let se::ImplicitCast {
      cast_type, expr, ..
    } = implicit_cast;

    let operand = self.expression(expr);
    self.do_cast(operand, *cast_type, ast_type)
  }

  // gush. this is the most fluffing part of the whole codegen.
  fn compound_assign(
    &mut self,
    compound_assign: &se::CompoundAssign<'c>,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    let se::CompoundAssign {
      operator,
      left,
      right,
      intermediate_left_type,
      intermediate_result_type,
    } = compound_assign;
    let left_ty = left.unqualified_type();
    debug_assert!(
      RefEq::ref_eq(left_ty, ast_type),
      "precond: the lhs type shall be the whole expr type."
    );
    let evaluated_lhs = self.expression(left);
    debug_assert!(
      self.visit(evaluated_lhs, |value| value.ir_type.is_pointer()),
      "The lhs should remains as-is -- an lvalue by the sema."
    );
    let rvalued_lhs =
      self.do_cast(evaluated_lhs, ast::CastType::LValueToRValue, left_ty);
    let lhs_compute_cast =
      se::Expression::get_cast_type(left_ty, intermediate_left_type);
    let lhs_id =
      self.do_cast(rvalued_lhs, lhs_compute_cast, intermediate_left_type);

    let right_ty = right.unqualified_type();
    let evaluated_rhs = self.expression(right);
    let rhs_comute_cast =
      se::Expression::get_cast_type(right_ty, intermediate_result_type);
    let rhs_id =
      self.do_cast(evaluated_rhs, rhs_comute_cast, intermediate_result_type);

    let assoc_op = operator.associated_operator().expect(
      "precond: sema ensures the validity of the op is valid compound operator",
    );

    let bin_id =
      self.do_binary(assoc_op, lhs_id, rhs_id, intermediate_result_type, span);
    let cast_me_back =
      se::Expression::get_cast_type(intermediate_result_type, ast_type);
    let casted_back = self.do_cast(bin_id, cast_me_back, ast_type);

    let _store = self.emit(
      inst::Store::new(evaluated_lhs, casted_back),
      self.ast().void_type(),
    );

    casted_back
  }
}
impl<'c> Emitter<'c> {
  fn do_cast(
    &mut self,
    operand: ValueID,
    cast_type: ast::CastType,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    macro_rules! call {
      ($method:ident) => {
        self.$method(operand, ast_type)
      };
    }

    use ast::CastType::*;
    match cast_type {
      BitCast => call!(bitcast),
      IntegralCast => call!(integral_cast),
      FloatingCast => call!(floating_cast),
      LValueToRValue => call!(lvalue_to_rvalue_cast),
      NullptrToPointer => call!(nullptr_to_pointer_cast),
      PointerToBoolean => call!(pointer_to_boolean_cast),
      IntegralToBoolean => call!(integral_to_boolean_cast),
      FloatingToBoolean => call!(floating_to_boolean_cast),
      IntegralToPointer => call!(integral_to_pointer_cast),
      PointerToIntegral => call!(pointer_to_integral_cast),
      ArrayToPointerDecay => call!(array_to_pointer_decay),
      FloatingToIntegral => call!(floating_to_integral_cast),
      IntegralToFloating => call!(integral_to_floating_cast),
      Noop | ToVoid | FunctionToPointerDecay => operand, //< noop
    }
  }

  #[inline]
  fn nullptr_to_pointer_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    eprintln!(
      "the nullptr to ptr conversion shall be folded before hit here, whose \
       type has already been assigned to the corresponding pointer type, and \
       has a data of Constant::Nullptr()."
    );
    self.emit(inst::BitCast::new(operand), ast_type)
  }

  fn floating_to_boolean_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    debug_assert!(RefEq::ref_eq(ast_type, self.ast().i8_bool_type()));
    // compare to 0.0, and then zext to i8
    let format =
      *self.visit(operand, |value| value.ir_type.as_floating_unchecked());

    let compared = self.do_floating_relational(
      inst::FCmpPredicate::Une,
      operand,
      self.ir().floating_zero(format),
      self.ast().i1_bool_type(),
      SourceSpan::default(), // doesn't matter.
    );
    self.emit(inst::Zext::new(compared), ast_type)
  }

  fn integral_to_boolean_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    debug_assert!(RefEq::ref_eq(ast_type, self.ast().i8_bool_type()));
    // compare to 0, and then zext to i8
    let width =
      *self.visit(operand, |value| value.ir_type.as_integer_unchecked());
    let compared = self.do_integral_relational(
      inst::ICmpPredicate::Ne,
      operand,
      self.ir().integer_zero(width),
      self.ast().i1_bool_type(),
      SourceSpan::default(),
    );
    self.emit(inst::Zext::new(compared), ast_type)
  }

  fn pointer_to_boolean_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    debug_assert!(RefEq::ref_eq(ast_type, self.ast().i8_bool_type()));
    // compare to nullptr, and then zext to i8
    let compared = self.do_pointer_relational(
      inst::ICmpPredicate::Ne,
      operand,
      self.ir().nullptr(),
      self.ast().i1_bool_type(),
      SourceSpan::default(), // ditto.
    );
    self.emit(inst::Zext::new(compared), ast_type)
  }

  #[inline]
  fn lvalue_to_rvalue_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit(inst::Load::new(operand), ast_type)
  }

  fn array_to_pointer_decay(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let zero = self
      .ir()
      .integer_zero(self.ast().ptrdiff_type().size_bits() as u8);
    self.emit(
      inst::GetElementPtr::new(vec![operand, zero, zero]),
      ast_type,
    )
  }

  #[inline]
  fn bitcast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit(inst::BitCast::new(operand), ast_type)
  }

  #[inline]
  fn integral_to_pointer_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit(inst::IntToPtr::new(operand), ast_type)
  }

  #[inline]
  fn pointer_to_integral_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit(inst::PtrToInt::new(operand), ast_type)
  }

  fn floating_to_integral_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let signedness = ast_type
      .signedness()
      .expect("integer always has signedness");
    use Signedness::*;
    match signedness {
      Signed => self.emit(inst::FPToSI::new(operand), ast_type),
      Unsigned => self.emit(inst::FPToUI::new(operand), ast_type),
    }
  }

  fn integral_to_floating_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let signedness = self
      .visit(operand, |value| value.ast_type.signedness())
      .expect("integer always has signedness");
    use Signedness::*;
    match signedness {
      Signed => self.emit(inst::SIToFP::new(operand), ast_type),
      Unsigned => self.emit(inst::UIToFP::new(operand), ast_type),
    }
  }

  fn floating_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    use ::std::cmp::Ordering::*;
    use inst::{FPExt, FPTrunc};
    // floating point no need to use `either` cuz there's no pointer w/ floating point arith.
    let format =
      self.visit(operand, |value| value.ir_type.as_floating_unchecked());
    match Ord::cmp(format, &ast_type.as_primitive_unchecked().floating_format())
    {
      Less => self.emit(FPExt::new(operand), ast_type),
      Equal => operand,
      Greater => self.emit(FPTrunc::new(operand), ast_type),
    }
  }

  fn integral_cast(
    &mut self,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    use ::std::cmp::Ordering::*;
    use Signedness::*;
    use inst::{Sext, Trunc, Zext};
    enum Either {
      Left(Integral),
      Right(u8),
    }
    use Either::*;
    let either = self.visit(operand, |value| {
      if let Some(c) = value.data.as_constant() {
        Left(*c.as_integral_unchecked())
      } else {
        Right(*value.ir_type.as_integer_unchecked())
      }
    });

    match either {
      Left(i) => self.emit(
        Constant::Integral(i.cast(
          ast_type.size_bits() as u8,
          ast_type.signedness().expect("never fails"),
        )),
        ast_type,
      ),
      Right(width) => match Ord::cmp(&width, &(ast_type.size_bits() as u8)) {
        Less => match ast_type.signedness() {
          Some(Signed) => self.emit(Sext::new(operand), ast_type),
          Some(Unsigned) => self.emit(Zext::new(operand), ast_type),
          None => unreachable!(),
        },
        Equal => operand,
        Greater => self.emit(Trunc::new(operand), ast_type),
      },
    }
  }
}

impl<'c> Emitter<'c> {
  fn binary(
    &mut self,
    binary: &se::Binary<'c>,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    let se::Binary {
      left,
      operator,
      right,
    } = binary;

    match operator {
      Operator::LogicalAnd =>
        self.logical_and(*operator, left, right, ast_type, span),
      Operator::LogicalOr =>
        self.logical_or(*operator, left, right, ast_type, span),
      _ => {
        let left = self.expression(left);
        let right = self.expression(right);
        self.do_binary(*operator, left, right, ast_type, span)
      },
    }
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
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(operator, Operator::Assign);
    debug_assert!(lookup!(self, left).ir_type.is_pointer());

    self.emit(inst::Store::new(left, right), self.ast().void_type())
  }

  #[allow(unused)]
  #[inline(always)]
  #[cold]
  fn logical(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    unreachable!("short-circuit && and || handled upstream.");
  }

  // A && B -> if(A) { B } else { 0 }
  fn logical_and(
    &mut self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(operator, Operator::LogicalAnd);
    let left_side_id = self.expression(left);
    let lhs = self.contextual_convert_to_i1(left_side_id);

    let now_block_id = self.current_block;
    let rhs_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    let _now_block_terminator = self.emit(
      inst::Branch::new(lhs, rhs_block_id, immediate_block_id),
      self.ast().void_type(),
    );

    let _should_be_now = self.push_block(rhs_block_id);
    debug_assert_eq!(now_block_id, _should_be_now);

    let right_side_id = self.expression(right);
    let rhs = self.contextual_convert_to_i1(right_side_id);
    let _rhs_block_terminator =
      self.emit(inst::Jump::new(immediate_block_id), self.ast().void_type());

    let _last_block_of_rhs_block = self.push_block(immediate_block_id);

    let i1_res = self.emit(
      inst::Phi::new(vec![
        self.ir().i1_false(),
        now_block_id,
        rhs,
        rhs_block_id,
      ]),
      self.ast().i1_bool_type(),
    );

    debug_assert!(RefEq::ref_eq(ast_type, self.ast().converted_bool()));

    self.emit(inst::Zext::new(i1_res), ast_type)
  }

  // A || B -> if(A) { 1 } else { B }
  fn logical_or(
    &mut self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(operator, Operator::LogicalOr);

    let left_side_id = self.expression(left);
    let lhs = self.contextual_convert_to_i1(left_side_id);

    let now_block_id = self.current_block;
    let rhs_block_id = self.new_empty_block();
    let immediate_block_id = self.new_empty_block();

    let _now_block_terminator = self.emit(
      inst::Branch::new(lhs, immediate_block_id, rhs_block_id),
      self.ast().void_type(),
    );

    let _should_be_now = self.push_block(rhs_block_id);
    debug_assert_eq!(now_block_id, _should_be_now);

    let right_side_id = self.expression(right);
    let rhs = self.contextual_convert_to_i1(right_side_id);
    let _rhs_block_terminator =
      self.emit(inst::Jump::new(immediate_block_id), self.ast().void_type());

    let _last_block_of_rhs_block = self.push_block(immediate_block_id);
    let i1_res = self.emit(
      inst::Phi::new(vec![
        self.ir().i1_true(),
        now_block_id,
        rhs,
        rhs_block_id,
      ]),
      self.ast().i1_bool_type(),
    );

    debug_assert!(RefEq::ref_eq(ast_type, self.ast().converted_bool()));
    self.emit(inst::Zext::new(i1_res), ast_type)
  }

  fn arithmetic(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    let lhs_ty = self.visit(left, |lhs| lhs.ast_type);
    let rhs_ty = self.visit(right, |rhs| rhs.ast_type);

    match (lhs_ty.is_pointer(), rhs_ty.is_pointer()) {
      (false, false) => self.do_arithmetic_operands(
        operator,
        left,
        right,
        ast_type,
        lhs_ty
          .signedness()
          .expect("arithmetic type always has signedness."),
        span,
      ),
      (true, true) =>
        self.do_pointer_arithmetic(left, right, ast_type, lhs_ty, rhs_ty),
      (true, false) => self
        .do_pointer_integer_arithmetic(operator, left, right, ast_type, span),
      (false, true) => unreachable!(
        "Semantic checker swapped the lhs and rhs if the left is not pointer \
         but the right is -- so this case should never happen."
      ),
    }
  }

  /// caller ensure the rhs is an integral.
  fn do_pointer_integer_arithmetic(
    &mut self,
    operator: Operator,
    pointer_operand: ValueID,
    integer_operand: ValueID,
    pointer_type: &'c ast::Type<'c>,
    span: SourceSpan,
  ) -> ValueID {
    use ::std::debug_assert_matches;
    debug_assert_matches!(operator, Operator::Plus | Operator::Minus);

    let operands = vec![
      pointer_operand,
      self.unary_arithmetic(
        operator,
        integer_operand,
        self.ast().ptrdiff_type(),
        span,
      ),
    ];

    self.emit(inst::GetElementPtr::new(operands), pointer_type)
  }

  /// caller ensures precond.
  #[inline]
  fn do_arithmetic_operands(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    signedness: Signedness,
    _span: SourceSpan,
  ) -> ValueID {
    use inst::{Binary, BinaryOp};
    self.emit(
      Binary::new(
        BinaryOp::from_op_and_sign(
          operator,
          signedness,
          ast_type.is_floating_point(),
        ),
        left,
        right,
      ),
      ast_type,
    )
  }

  fn do_pointer_arithmetic(
    &mut self,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    lhs_ty: ast::TypeRef<'c>,
    rhs_ty: ast::TypeRef<'c>,
  ) -> ValueID {
    use inst::{Binary, BinaryOp};
    // ptrtoint cast -> sub -> sdiv.
    let left = self.emit(inst::PtrToInt::new(left), self.ast().uintptr_type());
    let right =
      self.emit(inst::PtrToInt::new(right), self.ast().uintptr_type());
    let sub = self.emit(
      Binary::new(BinaryOp::Sub, left, right),
      self.ast().ptrdiff_type(),
    );
    debug_assert!(RefEq::ref_eq(
      lhs_ty.as_pointer_unchecked().pointee.unqualified_type,
      rhs_ty.as_pointer_unchecked().pointee.unqualified_type
    ));
    debug_assert!(RefEq::ref_eq(ast_type, self.ast().ptrdiff_type()));
    let size = self.emit(
      Constant::Integral(Integral::from_uintptr(
        lhs_ty
          .as_pointer_unchecked()
          .pointee
          .unqualified_type
          .size(),
      )),
      self.ast().uintptr_type(),
    );
    self.emit(Binary::new(BinaryOp::SDiv, sub, size), ast_type)
  }

  fn bitwise(
    &mut self,
    operator: Operator,
    left: ValueID,
    right: ValueID,
    ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
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
    _span: SourceSpan,
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
    _left: ValueID,
    right: ValueID,
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(operator, Operator::Comma);
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
      (Integer(_), Integer(_)) =>
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
    _span: SourceSpan,
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
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    self.emit(
      inst::ICmp::new(operator, left, right),
      self.ast().i1_bool_type(),
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
    _span: SourceSpan,
  ) -> ValueID {
    self.emit(inst::FCmp::new(operator, left, right), ast_type)
  }
}
impl<'c> Emitter<'c> {
  fn unary(
    &mut self,
    unary: &se::Unary<'c>,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    let se::Unary {
      kind,
      operand,
      operator,
    } = unary;
    let operand = self.expression(operand);

    self.do_unary(*operator, operand, *kind, ast_type, span)
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
      ($method:ident $(, $uglykind:ident)?) => {
        self.$method(operator, operand, ast_type, span $(, $uglykind)?)
      };
    }

    use Operator::*;
    match operator {
      Ampersand => call!(addressof),
      Star => call!(indirect),
      Not => call!(logical_not),
      Tilde => call!(tilde),
      Plus | Minus => call!(unary_arithmetic),
      PlusPlus | MinusMinus => call!(ppmm, kind),
      _ => unreachable!("operator is not unary: {:#?}", operator),
    }
  }

  /// Addressof is a no-op in IR level(for valid operands), since the operand should have already been loaded to a pointer if it's an lvalue, and if it's not an lvalue, it's already an rvalue and the address-of operator is invalid and should have been rejected by sema.
  #[inline(always)]
  fn addressof(
    &mut self,
    operator: Operator,
    operand: ValueID,
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(operator, Operator::Ampersand);
    operand
  }

  /// Dereference is also a no-op, since it produce an lvalue in AST level.
  ///
  /// And that when it needs to convert to rvalue, the [`Self::lvalue_to_rvalue_cast`] will emit the load inst.
  #[inline(always)]
  fn indirect(
    &mut self,
    operator: Operator,
    operand: ValueID,
    _ast_type: ast::TypeRef<'c>,
    _span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(operator, Operator::Star);
    debug_assert!(lookup!(self, operand).ir_type.is_pointer());
    operand
  }

  fn logical_not(
    &mut self,
    operator: Operator,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(operator, Operator::Not);

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
    _span: SourceSpan,
  ) -> ValueID {
    debug_assert_eq!(operator, Operator::Tilde);
    debug_assert!(ast_type.as_primitive().is_some_and(|p| p.is_integer()));
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
    _span: SourceSpan,
  ) -> ValueID {
    let is_floating = self.visit(operand, |value| value.ir_type.is_floating());
    match (operator, is_floating) {
      (Operator::Plus, false) => self.integral_cast(operand, ast_type),
      (Operator::Minus, false) => {
        let casted = self.integral_cast(operand, ast_type);
        let (width, is_constant) = self.visit(casted, |value| {
          (
            value.ast_type.size_bits() as u8,
            value.data.as_constant().map(|c| *c.as_integral_unchecked()),
          )
        });
        match is_constant {
          Some(integral) => self.emit(Constant::Integral(-integral), ast_type),
          None => {
            let zero = self.ir().integer_zero(width);
            self.emit(
              inst::Binary::new(inst::BinaryOp::Sub, zero, casted),
              ast_type,
            )
          },
        }
      },
      // no need to anything, the plus one forr integral maybe has a pointer operand(may ext or trunc to ast_type),
      // float does not have such exception(which means ast_type is same as operand's.)
      (Operator::Plus, true) => operand,
      (Operator::Minus, true) =>
        self.emit(inst::Unary::new(inst::UnaryOp::FNeg, operand), ast_type),
      _ => unreachable!(),
    }
  }

  fn ppmm(
    &mut self,
    operator: Operator,
    operand: ValueID,
    ast_type: ast::TypeRef<'c>,
    span: SourceSpan,
    kind: UnaryKind,
  ) -> ValueID {
    let pm = operator.ppmm2pm().expect("precond: op is pp or mm");

    let loaded = self.emit(inst::Load::new(operand), ast_type);
    // let ast_type = self.visit(loaded, |value| value.ast_type);
    // if ast_type is plain arithemetic type, just add or sub the corresponding integer or floating constant, and store back.
    // if it's a pointer type, do gep with 1 or -1 using the pointer arithmetic function, and store back.
    use ast::Type::*;
    let calculated = match ast_type {
      Primitive(p) => {
        let one = match p {
          i if i.is_integer() => self.ir().integer_one(i.size_bits() as u8),
          &f if f.is_floating_point() => self.ir().floating_one(f.into()),
          _ => unreachable!("not a proper arithematic type"),
        };
        self.do_arithmetic_operands(
          pm,
          loaded,
          one,
          ast_type,
          ast_type
            .signedness()
            .expect("arithematic type always have signedness"),
          span,
        )
      },
      Pointer(_) => {
        let one = self
          .ir()
          .integer_one(self.ast().ptrdiff_type().size_bits() as u8);
        self.do_pointer_integer_arithmetic(pm, loaded, one, ast_type, span)
      },
      _ => unreachable!("precond: ast type shall be a scalar."),
    };
    _ = self.emit(
      inst::Store::new(operand, calculated),
      self.ast().void_type(),
    );
    use UnaryKind::*;
    match kind {
      Prefix => calculated,
      Postfix => loaded,
    }
  }
}
