use ::rcc_shared::Constant;
use ::rcc_utils::StrRef;

use super::value::ValueID;
#[derive(Debug, Default)]
pub struct Module {
  /// global function and variable entry. Shall be either [`Function`] or [`Variable`], or [`Constant`].
  pub globals: Vec<ValueID>,
}

/// **Global** function in TAC-SSA form
#[derive(Debug)]
pub struct Function<'c> {
  pub name: StrRef<'c>,
  /// Shall be [`Argument`].
  pub params: Vec<ValueID>,
  /// Shall be [`BasicBlock`].
  pub entry: ValueID,
  /// Shall be [`BasicBlock`].
  pub blocks: Vec<ValueID>,
  pub is_variadic: bool,
}

impl<'c> Function<'c> {
  pub fn new(
    name: StrRef<'c>,
    params: Vec<ValueID>,
    entry: ValueID,
    blocks: Vec<ValueID>,
    is_variadic: bool,
  ) -> Self {
    Self {
      name,
      params,
      entry,
      blocks,
      is_variadic,
    }
  }

  pub fn new_empty(
    name: StrRef<'c>,
    params: Vec<ValueID>,
    is_variadic: bool,
  ) -> Self {
    Self {
      name,
      is_variadic,
      params,
      entry: Default::default(),
      blocks: Default::default(),
    }
  }

  #[inline(always)]
  pub fn is_definition(&self) -> bool {
    !self.blocks.is_empty()
  }
}

/// **Global** variable.
#[derive(Debug)]
pub struct Variable<'c> {
  pub name: StrRef<'c>,
  pub initializer: Option<Initializer<'c>>,
}

impl<'c> Variable<'c> {
  pub fn new(name: StrRef<'c>, initializer: Option<Initializer<'c>>) -> Self {
    Self { name, initializer }
  }
}

/// type should always be [`super::Type::Label`].
#[derive(Debug, Default)]
pub struct BasicBlock {
  /// Shall be [`super::instruction::Instruction`].
  pub instructions: Vec<ValueID>,
  /// Shall be [`super::instruction::Terminator`].
  pub terminator: ValueID,
}

impl BasicBlock {
  pub fn new(instructions: Vec<ValueID>, terminator: ValueID) -> Self {
    Self {
      instructions,
      terminator,
    }
  }
}

#[derive(Debug)]
pub struct Argument {
  /// Shall be [`Function`].
  pub index: usize,
}

impl Argument {
  pub fn new(index: usize) -> Self {
    Self { index }
  }
}

/// **Static** initializer.
#[derive(Debug, Clone)]
pub enum Initializer<'c> {
  Scalar(Constant<'c>),
  Aggregate(Vec<Initializer<'c>>),
}
