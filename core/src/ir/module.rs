use super::value::ValueID;
use crate::{common::StrRef, types::Constant};
#[derive(Debug, Default)]
pub struct Module {
  /// global function and variable entry. Shall be either [`Function`] or [`Variable`].
  pub globals: Vec<ValueID>,
  // pub constants: Vec<ValueID>,
}

/// **Global** function in TAC-SSA form
#[derive(Debug)]
pub struct Function<'c> {
  pub name: StrRef<'c>,
  /// Shall be [`Argument`].
  pub params: Vec<ValueID>,
  /// Shall be [`BasicBlock`].
  pub blocks: Vec<ValueID>,
  pub is_variadic: bool,
}

impl<'c> Function<'c> {
  pub fn new(
    name: StrRef<'c>,
    params: Vec<ValueID>,
    blocks: Vec<ValueID>,
    is_variadic: bool,
  ) -> Self {
    Self {
      name,
      params,
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
  pub function: ValueID,
  pub index: usize,
}

impl Argument {
  pub fn new(function: ValueID, index: usize) -> Self {
    Self { function, index }
  }
}

/// **Static** initializer.
#[derive(Debug, Clone)]
pub enum Initializer<'c> {
  Scalar(Constant<'c>),
  Aggregate(Vec<Initializer<'c>>),
}
