use ::rcc_utils::SmallString;
use ::std::collections::HashMap;

use super::{
  ilist_type,
  instruction::{self as inst, Instruction, Operand},
  module::{self, BasicBlock, Module},
};
use crate::{
  common::{SourceSpan, StrRef},
  sema::{
    declaration as ad,
    expression::{self as ae, ValueCategory},
    statement as astmt,
  },
  session::Session,
  types::{Context, QualifiedType},
};

pub struct ModuleBuilder<'session, 'context, 'source>
where
  'context: 'session,
  'source: 'context,
{
  session: &'session Session<'context, 'source>,
  /// counter for generating unique temporary names
  temp_counter: usize,
  /// counter for generating unique label names
  label_counter: usize,
  /// The basic block currently being written into
  current_block: Option<BasicBlock<'context>>,
  /// Blocks finalized in the current function
  current_blocks: ilist_type<BasicBlock<'context>>,
  locals: HashMap<StrRef<'context>, Operand<'context>>,
  module: Module<'context>,
}
impl<'session, 'context, 'source> ModuleBuilder<'session, 'context, 'source> {
  pub fn new(session: &'session Session<'context, 'source>) -> Self {
    Self {
      session,
      label_counter: 0,
      temp_counter: 0,
      current_block: None,
      current_blocks: Default::default(),
      locals: Default::default(),
      module: Default::default(),
    }
  }
}
impl<'session, 'context, 'source> ModuleBuilder<'session, 'context, 'source> {
  fn emit(&mut self, instruction: Instruction<'context>) {
    if let Some(block) = &mut self.current_block {
      block.instructions.push(instruction);
      return;
    }
    panic!("no block to push.")
  }

  fn push_block(&mut self, label: &str) {
    self.seal_current_block();
    self.current_block = Some(BasicBlock {
      label: label.into(),
      instructions: ilist_type::new(),
    });
  }

  fn seal_current_block(&mut self) {
    if let Some(block) = self.current_block.take() {
      self.current_blocks.push(block);
    }
  }

  fn reg(&mut self) -> Operand<'context> {
    self.temp_counter += 1;
    Operand::Reg(self.temp_counter)
  }
}

impl<'session, 'context, 'source> ModuleBuilder<'session, 'context, 'source> {
  pub fn build(
    mut self,
    translation_unit: ad::TranslationUnit<'context>,
  ) -> Module<'context> {
    self.module.functions =
      ilist_type::with_capacity(translation_unit.declarations.len() * 2 / 3);
    self.module.globals =
      Vec::with_capacity(translation_unit.declarations.len() / 3);
    translation_unit
      .declarations
      .into_iter()
      .for_each(|decl| match decl {
        ad::ExternalDeclaration::Function(function) => {
          let function = self.function(function);
          self.module.functions.push(function)
        },
        ad::ExternalDeclaration::Variable(variable) => {
          let variable = self.vardef(variable);
          self.module.globals.push(variable)
        },
      });
    self.module
  }

  pub fn function(
    &mut self,
    function: ad::Function<'context>,
  ) -> module::Function<'context> {
    let ad::Function {
      symbol,
      parameters,
      specifier,
      body,
      labels,
      gotos,
      span,
    } = function;

    assert!(
      self.locals.is_empty() && self.current_block.is_none(),
      "not implemented for local func decl!"
    );

    self.locals.clear();
    self.current_blocks = ilist_type::new();

    // bind parameters as operands
    let params: Vec<Operand> = parameters
      .into_iter()
      .map(|p| {
        let sym = p.symbol.borrow();
        let op = self.reg();
        self.locals.insert(sym.name, op.clone());
        op
      })
      .collect();

    if let Some(body) = body {
      self.compound(body);
      self.seal_current_block();
    }
    // locals clear, mamtake would take care of cur blocks
    self.locals.clear();

    module::Function {
      name: symbol.borrow().name,
      params,
      blocks: std::mem::take(&mut self.current_blocks),
      return_type: symbol
        .borrow()
        .qualified_type
        .as_functionproto_unchecked()
        .return_type,
      is_variadic: false,
    }
  }

  pub fn vardef(
    &mut self,
    variable: ad::VarDef<'context>,
  ) -> module::Variable<'context> {
    todo!()
  }

  fn compound(&mut self, body: astmt::Compound<'context>) {
    let astmt::Compound { statements, span } = body;
    if statements.is_empty() {
      self.push_block("noop");
    } else {
      self.push_block("entry");
    }
    for statement in statements {
      self.statement(statement);
    }
  }

  fn statement(&mut self, statement: astmt::Statement<'context>) {
    #[allow(clippy::upper_case_acronyms)]
    type STMT<'a> = astmt::Statement<'a>;
    match statement {
      STMT::Empty(_) => (),
      STMT::Return(return_stmt) => todo!(),
      STMT::Expression(expression) => {
        self.expression(expression);
      },
      STMT::Declaration(external_declaration) => todo!(),
      STMT::Compound(compound) => todo!(),
      STMT::If(if_stmt) => todo!(),
      STMT::While(while_stmt) => todo!(),
      STMT::DoWhile(do_while) => todo!(),
      STMT::For(for_stmt) => todo!(),
      STMT::Switch(switch) => todo!(),
      STMT::Goto(goto) => todo!(),
      STMT::Label(label) => todo!(),
      STMT::Break(break_stmt) => todo!(),
      STMT::Continue(continue_stmt) => todo!(),
    }
  }
}
impl<'session, 'context, 'source> ModuleBuilder<'session, 'context, 'source> {
  fn expression(
    &mut self,
    expression: ae::Expression<'context>,
  ) -> Option<Operand<'context>> {
    let (raw_expr, qualified_type, value_category) = expression.destructure();
    type RE<'a> = ae::RawExpr<'a>;
    match raw_expr {
      RE::Empty(_) => None,
      RE::Constant(constant) =>
        self.constant(constant, qualified_type, value_category),
      RE::Unary(unary) => todo!(),
      RE::Binary(binary) => todo!(),
      RE::Call(call) => self.call(call, qualified_type, value_category),
      RE::Paren(paren) => todo!(),
      RE::MemberAccess(member_access) => todo!(),
      RE::Ternary(ternary) => todo!(),
      RE::SizeOf(size_of) => todo!(),
      RE::CStyleCast(cstyle_cast) => todo!(),
      RE::ArraySubscript(array_subscript) => todo!(),
      RE::CompoundLiteral(compound_literal) => todo!(),
      RE::Variable(variable) => todo!(),
      RE::ImplicitCast(implicit_cast) => todo!(),
      RE::Assignment(assignment) => todo!(),
    }
  }

  fn call(
    &mut self,
    call: ae::Call<'context>,
    qualified_type: QualifiedType<'context>,
    value_category: ValueCategory,
  ) -> Option<Operand<'context>> {
    let ae::Call {
      callee,
      arguments,
      span,
    } = call;
    // let callee_operand = self.expression(*callee);
    let oprand_args = arguments
      .into_iter()
      .map(|actual_parameter| {
        self.expression(actual_parameter).expect(
          "should have oprand (- RValue shall be handled in analyzer), or \
           probably unhandled situation",
        )
      })
      .collect();
    let retreg = if qualified_type.is_void() {
      None
    } else {
      Some(self.reg())
    };

    let callee_name = callee
      .raw_expr()
      .as_variable()
      .expect("not implemented for complex callee!")
      .name
      .borrow()
      .name;

    let func_sig = self
      .module
      .functions
      .iter()
      .find(|f| ::std::ptr::eq(f.name, callee_name))
      .expect(
        "callee not yet emitted — maybe it's local funcdecl -- currently \
         havent handled it!",
      );

    self.emit(
      inst::Call::new(
        retreg.clone(),
        Operand::Label(func_sig.name),
        oprand_args,
      )
      .into(),
    );
    retreg
  }

  fn constant(
    &self,
    constant: ae::Constant<'context>,
    qualified_type: QualifiedType<'context>,
    value_category: ValueCategory, // should be RValue
  ) -> Option<Operand<'context>> {
    debug_assert!(value_category == ValueCategory::RValue);
    let ae::Constant { value, span } = constant;
    Some(Operand::Imm(value))
  }
}
