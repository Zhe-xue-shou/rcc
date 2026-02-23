use ::slotmap::SlotMap;

use super::{
  instruction::Instruction,
  value::{BlockID, FuncID, GlobalID, InstID, ValueData, ValueID},
};
use crate::{
  common::StrRef,
  types::{Constant, QualifiedType},
};

#[derive(Debug, Default)]
pub struct Module<'context> {
  pub values: SlotMap<ValueID, ValueData<'context>>,
  pub instructions: SlotMap<InstID, Instruction<'context>>,
  pub blocks: SlotMap<BlockID, BasicBlock>,
  pub functions: SlotMap<FuncID, Function<'context>>,
  pub globals: SlotMap<GlobalID, Variable<'context>>,
}

/// **Global** function in TAC-SSA form
#[derive(Debug)]
pub struct Function<'context> {
  pub name: StrRef<'context>,
  pub return_type: QualifiedType<'context>,
  pub params: Vec<ValueID>,

  pub blocks: Vec<BlockID>,
  pub is_variadic: bool,
}

/// **Global** variable.
#[derive(Debug)]
pub struct Variable<'context> {
  pub name: StrRef<'context>,
  pub qualified_type: QualifiedType<'context>,
  pub initializer: Option<Initializer<'context>>,
}

#[derive(Debug)]
pub struct BasicBlock {
  pub instructions: Vec<InstID>,
  pub terminator: InstID,
}

/// **Static** initializer.
#[derive(Debug, Clone)]
pub enum Initializer<'context> {
  Scalar(Constant<'context>),
  Aggregate(Vec<Initializer<'context>>),
}
