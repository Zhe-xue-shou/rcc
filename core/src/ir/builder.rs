use ::rcc_utils::contract_violation;
use ::slotmap::Key;
use ::std::collections::HashMap;

use super::{
  Argument, ValueData,
  instruction::{self as inst, Instruction, Terminator},
  module::{self, BasicBlock, Module},
  value::{Value, ValueID},
};
use crate::{
  common::StrRef,
  sema::{declaration as sd, expression as se, statement as ss},
  session::Session,
  types::{CastType, QualifiedType},
};
/// Overload helper. I love overloading.
pub trait Emitable<'a, ValueType> {
  fn emit(
    &mut self,
    value: ValueType,
    qualified_type: QualifiedType<'a>,
  ) -> ValueID;
}
pub struct ModuleBuilder<'source, 'context, 'session>
where
  'context: 'session,
  'source: 'context,
{
  session: &'session Session<'source, 'context>,
  /// The basic block currently being written into
  current_block: Option<BasicBlock>,
  /// Blocks finalized in the current function
  current_blocks: Vec<ValueID>,
  locals: HashMap<StrRef<'context>, ValueID>,
  /// function name → ValueID for call resolution
  func_values: HashMap<StrRef<'context>, ValueID>,
  module: Module,
}
macro_rules! ir_type {
  ($self:ident, $qualified_type:expr) => {
    $self.session.ir_context.ir_type(&$qualified_type)
  };
}
impl<'source, 'context, 'session> ModuleBuilder<'source, 'context, 'session> {
  pub fn new(session: &'session Session<'source, 'context>) -> Self {
    Self {
      session,
      current_block: Default::default(),
      current_blocks: Default::default(),
      locals: Default::default(),
      func_values: Default::default(),
      module: Default::default(),
    }
  }
}
impl<'context> Emitable<'context, Terminator>
  for ModuleBuilder<'_, 'context, '_>
{
  fn emit(
    &mut self,
    terminator: Terminator,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    if let Some(block) = &mut self.current_block {
      assert!(block.terminator.is_null(), "block already has a terminator");
      let value_id = self.session.ir_context.insert(Value::new(
        qualified_type,
        ir_type!(self, qualified_type),
        Into::<Instruction>::into(terminator).into(),
      ));
      block.terminator = value_id;
      value_id
    } else {
      panic!("no block to emit terminator into")
    }
  }
}
impl<'context, InstType: Into<Instruction>> Emitable<'context, InstType>
  for ModuleBuilder<'_, 'context, '_>
{
  default fn emit(
    &mut self,
    value: InstType,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    if let Some(block) = &mut self.current_block {
      let value_id = self.session.ir_context.insert(Value::new(
        qualified_type,
        ir_type!(self, qualified_type),
        value.into().into(),
      ));
      block.instructions.push(value_id);
      value_id
    } else {
      panic!("no block to emit into")
    }
  }
}

impl<'context> Emitable<'context, module::Function<'context>>
  for ModuleBuilder<'_, 'context, '_>
{
  fn emit(
    &mut self,
    value: module::Function<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let value_id = self.session.ir_context.insert(Value::new(
      qualified_type,
      ir_type!(self, qualified_type),
      value.into(),
    ));
    self.module.globals.push(value_id);
    value_id
  }
}
impl<'context> Emitable<'context, module::Variable<'context>>
  for ModuleBuilder<'_, 'context, '_>
{
  fn emit(
    &mut self,
    value: module::Variable<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let value_id = self.session.ir_context.insert(Value::new(
      qualified_type,
      ir_type!(self, qualified_type),
      value.into(),
    ));
    self.module.globals.push(value_id);
    value_id
  }
}
impl<'context> Emitable<'context, se::Constant<'context>>
  for ModuleBuilder<'_, 'context, '_>
{
  fn emit(
    &mut self,
    value: se::Constant<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    self.session.ir_context.insert(Value::new(
      qualified_type,
      ir_type!(self, qualified_type),
      value.into(),
    ))
  }
}
impl<'context> Emitable<'context, Argument>
  for ModuleBuilder<'_, 'context, '_>
{
  fn emit(
    &mut self,
    value: Argument,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    self.session.ir_context.insert(Value::new(
      qualified_type,
      ir_type!(self, qualified_type),
      value.into(),
    ))
  }
}
impl<'context> ModuleBuilder<'_, 'context, '_> {
  fn push_block(&mut self) {
    self.seal_current_block();
    self.current_block = Some(Default::default());
  }

  fn seal_current_block(&mut self) {
    if let Some(block) = self.current_block.take() {
      let block_id = self.session.ir_context.insert(Value::new(
        self.session.ast_context.void_type().into(),
        self.session.ir_context.label_type(),
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

impl<'context> ModuleBuilder<'_, 'context, '_> {
  pub fn build(
    mut self,
    translation_unit: sd::TranslationUnit<'context>,
  ) -> Module {
    let declarations = translation_unit.declarations;

    self.module.globals = Vec::with_capacity(declarations.len() / 4 + 1);

    // Pre-register all functions so forward references resolve.
    declarations.iter().for_each(|decl| {
      if let sd::ExternalDeclaration::Function(function) = decl {
        self.declare_global_function(function);
      }
    });

    // Process all declarations (fill in function bodies, globals).
    for decl in declarations {
      match decl {
        sd::ExternalDeclaration::Function(function) => {
          let name = function.symbol.borrow().name;
          let value_id = self.func_values[name];
          self.function(function, value_id);
        },
        sd::ExternalDeclaration::Variable(variable) => {
          let qualified_type = variable.symbol.borrow().qualified_type;
          let variable = self.global_vardef(variable);
          self.emit(variable, qualified_type);
        },
      }
    }
    self.module
  }

  fn declare_global_function(
    &mut self,
    function: &sd::Function<'context>,
  ) -> ValueID {
    if let Some(&value_id) = self.func_values.get(function.symbol.borrow().name)
    {
      debug_assert!(
        value_id != ValueID::null(),
        "function should have been pre-registered"
      );
      debug_assert!(
        self.session.ir_context.get(value_id).data.is_function(),
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
        module::Function::new_empty(name, is_variadic),
        qualified_type,
      );
      let params = function
        .parameters
        .iter()
        .enumerate()
        .map(|(index, p)| {
          let sym = p.symbol.borrow();
          let param_value_id =
            self.emit(Argument::new(value_id, index), sym.qualified_type);
          self.locals.insert(sym.name, param_value_id);
          param_value_id
        })
        .collect::<Vec<_>>();

      self
        .session
        .ir_context
        .get_mut(value_id)
        .data
        .as_function_mut_unchecked()
        .params = params;

      self.func_values.insert(name, value_id);
      value_id
    }
  }
}

impl<'context> ModuleBuilder<'_, 'context, '_> {
  fn function(
    &mut self,
    function: sd::Function<'context>,
    function_id: ValueID,
  ) {
    let sd::Function {
      symbol,
      parameters,
      body,
      ..
    } = function;

    self.locals.clear();
    self.current_blocks = Vec::new();

    if let Some(body) = body {
      self.compound(body);
      self.seal_current_block();
    }

    self
      .session
      .ir_context
      .get_mut(function_id)
      .data
      .as_function_mut_unchecked()
      .blocks = ::std::mem::take(&mut self.current_blocks);

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

  fn local_decl(
    &mut self,
    external_declaration: sd::ExternalDeclaration<'context>,
  ) {
    debug_assert!(!self.is_global());
    match external_declaration {
      sd::ExternalDeclaration::Function(function) => {
        debug_assert!(function.is_declaration());
        self.declare_global_function(&function);
      },
      sd::ExternalDeclaration::Variable(var_def) => self.local_vardef(var_def),
    }
  }

  fn local_vardef(&mut self, var_def: sd::VarDef<'context>) {
    use inst::{Alloca, Memory, Store};
    let sym = var_def.symbol.borrow();
    let var_type = ir_type!(self, sym.qualified_type);
    let alloca = Into::<Memory>::into(Alloca::new());
    let value_id = self.emit(alloca, sym.qualified_type);
    self.locals.insert(sym.name, value_id);
  }
}

impl<'context> ModuleBuilder<'_, 'context, '_> {
  fn statement(&mut self, statement: ss::Statement<'context>) {
    use ss::Statement::*;
    match statement {
      Empty(_) => (),
      Return(return_stmt) => self.returnstmt(return_stmt),
      Expression(expression) => self.exprstmt(expression),
      Declaration(external_declaration) =>
        self.local_decl(external_declaration),
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
    let operand = expression.and_then(|e| self.expression(e));
    self.emit(
      inst::Terminator::Return(inst::Return::new(operand)),
      qualified_type,
    );
  }

  fn compound(&mut self, body: ss::Compound<'context>) {
    let ss::Compound { statements, .. } = body;
    self.push_block();
    statements
      .into_iter()
      .for_each(|statement| self.statement(statement));
  }
}

impl<'context> ModuleBuilder<'_, 'context, '_> {
  fn expression(
    &mut self,
    expression: se::Expression<'context>,
  ) -> Option<ValueID> {
    // the fold here is redundant, yet seems valid,
    // e.g., `2 + 3 + getint(4 + 5)` adn be firstly folded to `5 + getint(4 + 5)`
    // and when the callexpr evaluates its args, it'll fold to `5 + getint(9)` further.
    let (raw_expr, qualified_type, ..) = expression
      .fold(&self.session.diagnosis)
      .unwrap()
      .destructure();
    use se::RawExpr::*;
    match raw_expr {
      Empty(_) => contract_violation!(
        "empty expr is used in sema for error recovery. shouldnt reach here."
      ),
      Constant(constant) => self.constant(constant, qualified_type),
      Unary(unary) => todo!(),
      Binary(binary) => todo!(),
      Call(call) => self.call(call, qualified_type),
      Paren(paren) => self.paren(paren),
      MemberAccess(member_access) => todo!(),
      Ternary(ternary) => todo!(),
      SizeOf(size_of) => todo!(),
      CStyleCast(cstyle_cast) => todo!(),
      ArraySubscript(array_subscript) => todo!(),
      CompoundLiteral(compound_literal) => todo!(),
      Variable(variable) => self.variable(variable, qualified_type),
      ImplicitCast(implicit_cast) =>
        self.implicit_cast(implicit_cast, qualified_type),
      Assignment(assignment) => todo!(),
    }
  }

  fn constant(
    &mut self,
    constant: se::Constant<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> Option<ValueID> {
    Some(self.emit(constant, qualified_type))
  }

  fn variable(
    &self,
    variable: se::Variable<'context>,
    _qualified_type: QualifiedType<'context>,
  ) -> Option<ValueID> {
    let name = variable.name.borrow().name;
    if let Some(&vid) = self.locals.get(name) {
      return Some(vid);
    }
    if let Some(&vid) = self.func_values.get(name) {
      return Some(vid);
    }
    panic!("undefined variable: {name}")
  }

  fn implicit_cast(
    &mut self,
    cast: se::ImplicitCast<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> Option<ValueID> {
    match cast.cast_type {
      // These casts don't change the value representation at IR level.
      CastType::LValueToRValue => {
        let value_id = self.expression(*cast.expr);
        Some(self.emit(
          Into::<inst::Memory>::into(inst::Load::new(value_id.expect(
            "The value_id is an address -- Global Function or Variable, \
             should never fails",
          ))),
          qualified_type,
        ))
      },
      CastType::FunctionToPointerDecay
      | CastType::ArrayToPointerDecay
      | CastType::Noop => self.expression(*cast.expr),
      // todo: int <-> int: transparent for now (same-width or promotion)
      CastType::IntegralCast => self.expression(*cast.expr),
      _ => todo!("implicit cast: {:?}", cast.cast_type),
    }
  }

  fn call(
    &mut self,
    call: se::Call<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> Option<ValueID> {
    let se::Call {
      callee, arguments, ..
    } = call;

    let callee = self
      .expression(*callee)
      .expect("callee must produce a value");

    let args: Vec<ValueID> = arguments
      .into_iter()
      .map(|arg| self.expression(arg).expect("argument must produce a value"))
      .collect();

    let call_inst = inst::Call { callee, args };

    Some(self.emit(call_inst, qualified_type))
  }

  #[inline]
  fn paren(&mut self, paren: se::Paren<'context>) -> Option<ValueID> {
    self.expression(*paren.expr)
  }
}
