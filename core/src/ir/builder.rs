use ::rcc_utils::contract_violation;
use ::slotmap::{Key, SlotMap};
use ::std::collections::HashMap;

use super::{
  instruction::{self as inst, Instruction, Terminator},
  module::{self, BasicBlock, Module},
  value::{BlockID, FuncID, InstID, Value, ValueData, ValueID},
};
use crate::{
  common::StrRef,
  sema::{declaration as sd, expression as se, statement as ss},
  session::Session,
  types::{CastType, QualifiedType},
};

pub struct ModuleBuilder<'session, 'context, 'source>
where
  'context: 'session,
  'source: 'context,
{
  session: &'session Session<'context, 'source>,
  /// The basic block currently being written into
  current_block: Option<BasicBlock>,
  /// Blocks finalized in the current function
  current_blocks: Vec<BlockID>,
  locals: HashMap<StrRef<'context>, ValueID>,
  /// function name → ValueID for call resolution
  func_values: HashMap<StrRef<'context>, ValueID>,
  module: Module<'context>,
}
impl<'session, 'context, 'source> ModuleBuilder<'session, 'context, 'source> {
  pub fn new(session: &'session Session<'context, 'source>) -> Self {
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

impl<'context> ModuleBuilder<'_, 'context, '_> {
  /// Create a new Value and return its ValueID.
  fn make_value(
    &mut self,
    qualified_type: QualifiedType<'context>,
    value: Value<'context>,
  ) -> ValueID {
    self
      .module
      .values
      .insert(ValueData::new(qualified_type, value))
  }

  /// Emit an instruction into the current block. Returns its InstID.
  fn emit(&mut self, instruction: Instruction<'context>) -> InstID {
    if let Some(block) = &mut self.current_block {
      let inst_id = self.module.instructions.insert(instruction);
      block.instructions.push(inst_id);
      return inst_id;
    }
    panic!("no block to emit into")
  }

  /// Emit an instruction that produces a value. Returns the result ValueID.
  fn emit_value(
    &mut self,
    instruction: Instruction<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let inst_id = self.emit(instruction);
    self.make_value(qualified_type, Value::Instruction(inst_id))
  }

  fn emit_terminator(&mut self, terminator: Terminator) {
    if let Some(block) = &mut self.current_block {
      assert!(block.terminator.is_null(), "block already has a terminator");
      let inst_id = self.module.instructions.insert(terminator.into());
      block.terminator = inst_id;
      return;
    }
    panic!("no block to emit terminator into")
  }

  fn push_block(&mut self) {
    self.seal_current_block();
    self.current_block = Some(BasicBlock {
      instructions: Vec::new(),
      terminator: InstID::null(),
    });
  }

  fn seal_current_block(&mut self) {
    if let Some(block) = self.current_block.take() {
      let block_id = self.module.blocks.insert(block);
      self.current_blocks.push(block_id);
    }
  }
}

impl<'context> ModuleBuilder<'_, 'context, '_> {
  pub fn build(
    mut self,
    translation_unit: sd::TranslationUnit<'context>,
  ) -> Module<'context> {
    let declarations = translation_unit.declarations;

    self.module.functions =
      SlotMap::with_capacity_and_key(declarations.len() * 3 / 4 + 1);
    self.module.globals =
      SlotMap::with_capacity_and_key(declarations.len() / 4 + 1);

    // Pre-register all functions so forward references resolve.
    for decl in &declarations {
      if let sd::ExternalDeclaration::Function(f) = decl {
        let sym = f.symbol.borrow();
        let name = sym.name;
        let qualified_type = sym.qualified_type;
        let proto = qualified_type.as_functionproto_unchecked();
        let return_type = proto.return_type;
        let is_variadic = proto.is_variadic;
        drop(sym);

        let func_id = self.module.functions.insert(module::Function {
          name,
          return_type,
          params: Vec::new(),
          blocks: Vec::new(),
          is_variadic,
        });
        let vid = self.make_value(qualified_type, Value::Function(func_id));
        self.func_values.insert(name, vid);
      }
    }

    // Process all declarations (fill in function bodies, globals).
    for decl in declarations {
      match decl {
        sd::ExternalDeclaration::Function(function) => {
          let name = function.symbol.borrow().name;
          let vid = self.func_values[name];
          let func_id = match self.module.values[vid].value {
            Value::Function(fid) => fid,
            _ => unreachable!(),
          };
          let ir_func = self.function(function, func_id);
          self.module.functions[func_id] = ir_func;
        },
        sd::ExternalDeclaration::Variable(variable) => {
          let variable = self.vardef(variable);
          self.module.globals.insert(variable);
        },
      }
    }
    self.module
  }
}

impl<'context> ModuleBuilder<'_, 'context, '_> {
  fn function(
    &mut self,
    function: sd::Function<'context>,
    func_id: FuncID,
  ) -> module::Function<'context> {
    let sd::Function {
      symbol,
      parameters,
      body,
      ..
    } = function;

    assert!(
      self.locals.is_empty() && self.current_block.is_none(),
      "not implemented for local func decl!"
    );

    self.locals.clear();
    self.current_blocks = Vec::new();

    // Bind parameters as Argument values.
    let params: Vec<ValueID> = parameters
      .into_iter()
      .enumerate()
      .map(|(i, p)| {
        let sym = p.symbol.borrow();
        let value_id =
          self.make_value(sym.qualified_type, Value::Argument(func_id, i));
        self.locals.insert(sym.name, value_id);
        value_id
      })
      .collect();

    if let Some(body) = body {
      self.compound(body);
      self.seal_current_block();
    }

    self.locals.clear();

    let sym = symbol.borrow();
    let proto = sym.qualified_type.as_functionproto_unchecked();
    module::Function {
      name: sym.name,
      params,
      blocks: std::mem::take(&mut self.current_blocks),
      return_type: proto.return_type,
      is_variadic: proto.is_variadic,
    }
  }

  fn vardef(
    &self,
    variable: sd::VarDef<'context>,
  ) -> module::Variable<'context> {
    let sym = variable.symbol.borrow();
    module::Variable {
      name: sym.name,
      qualified_type: sym.qualified_type,
      initializer: None, // TODO: handle initializers
    }
  }
}

impl<'context> ModuleBuilder<'_, 'context, '_> {
  fn statement(&mut self, statement: ss::Statement<'context>) {
    use ss::Statement::*;
    match statement {
      Empty(_) => (),
      Return(return_stmt) => self.returnstmt(return_stmt),
      Expression(expression) => self.exprstmt(expression),
      Declaration(external_declaration) => todo!(),
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
    let operand = expression.and_then(|e| self.expression(e));
    self.emit_terminator(inst::Terminator::Return(inst::Return {
      result: operand,
    }));
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
    /// the fold here is redundant, yet seems valid,
    /// e.g., `2 + 3 + getint(4 + 5)` adn be firstly folded to `5 + getint(4 + 5)`
    /// and when the callexpr evaluates its args, it'll fold to `5 + getint(9)` further.
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
    Some(self.make_value(qualified_type, Value::Constant(constant.value)))
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
    _qualified_type: QualifiedType<'context>,
  ) -> Option<ValueID> {
    match cast.cast_type {
      // These casts don't change the value representation at IR level.
      CastType::LValueToRValue
      | CastType::FunctionToPointerDecay
      | CastType::ArrayToPointerDecay
      | CastType::Noop => self.expression(*cast.expr),
      // int <-> int: transparent for now (same-width or promotion)
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

    if qualified_type.is_void() {
      self.emit(call_inst.into());
      None
    } else {
      Some(self.emit_value(call_inst.into(), qualified_type))
    }
  }

  #[inline]
  fn paren(&mut self, paren: se::Paren<'context>) -> Option<ValueID> {
    self.expression(*paren.expr)
  }
}
