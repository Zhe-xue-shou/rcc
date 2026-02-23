use ::slotmap::new_key_type;

use super::{
  BasicBlock, Module,
  instruction::Instruction,
  module::{Function, Variable},
};
use crate::types::{Constant, QualifiedType};

new_key_type! {
    pub struct ValueID;
    pub struct BlockID;
    pub struct FuncID;
    pub struct GlobalID;
    pub struct InstID;
}

pub trait LookUp<K> {
  type Output;
  #[must_use]
  fn lookup(&self, key: K) -> &Self::Output;
}
impl<K: Copy, C: LookUp<K>> LookUp<&K> for C {
  type Output = <C as LookUp<K>>::Output;

  #[inline(always)]
  fn lookup(&self, key: &K) -> &Self::Output {
    self.lookup(*key)
  }
}

impl<'a> LookUp<ValueID> for Module<'a> {
  type Output = ValueData<'a>;

  #[inline(always)]
  fn lookup(&self, key: ValueID) -> &Self::Output {
    &self.values[key]
  }
}
impl<'a> LookUp<InstID> for Module<'a> {
  type Output = Instruction<'a>;

  #[inline(always)]
  fn lookup(&self, key: InstID) -> &Self::Output {
    &self.instructions[key]
  }
}
impl<'a> LookUp<BlockID> for Module<'a> {
  type Output = BasicBlock;

  #[inline(always)]
  fn lookup(&self, key: BlockID) -> &Self::Output {
    &self.blocks[key]
  }
}
impl<'a> LookUp<FuncID> for Module<'a> {
  type Output = Function<'a>;

  #[inline(always)]
  fn lookup(&self, key: FuncID) -> &Self::Output {
    &self.functions[key]
  }
}
impl<'a> LookUp<GlobalID> for Module<'a> {
  type Output = Variable<'a>;

  #[inline(always)]
  fn lookup(&self, key: GlobalID) -> &Self::Output {
    &self.globals[key]
  }
}

#[derive(Debug)]
pub enum Value<'context> {
  Instruction(InstID),
  Argument(FuncID, usize),
  Constant(Constant<'context>),
  Function(FuncID),
  Global(GlobalID),
}

#[derive(Debug)]
pub struct ValueData<'context> {
  pub qualified_type: QualifiedType<'context>,
  pub value: Value<'context>,
  pub users: Vec<InstID>,
}

impl<'context> ValueData<'context> {
  pub fn new(
    qualified_type: QualifiedType<'context>,
    value: Value<'context>,
  ) -> Self {
    Self {
      qualified_type,
      value,
      users: Vec::new(),
    }
  }
}
