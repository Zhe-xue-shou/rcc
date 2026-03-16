use super::{
  Argument, BasicBlock, Constant, TypeRef,
  instruction::Instruction,
  module::{Function, Variable},
};
use crate::types::QualifiedType;

::slotmap::new_key_type! {
    pub struct ValueID;
}

impl ::std::fmt::Display for ValueID {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.0.as_ffi().fmt(f)
  }
}

pub(super) trait Lookup<KeyType, ValueType> {
  fn lookup(&self, key: KeyType) -> &ValueType;
}

impl<'context> ValueID {
  pub(super) fn lookup(
    &self,
    arena: &'context impl Lookup<ValueID, Value<'context>>,
  ) -> &Value<'context> {
    arena.lookup(*self)
  }
}

#[derive(Debug)]
pub enum Data<'context> {
  Instruction(Instruction),
  Constant(Constant<'context>),
  Function(Function<'context>),
  Variable(Variable<'context>),
  BasicBlock(BasicBlock),
  Argument(Argument),
}

#[derive(Debug)]
pub struct Value<'context> {
  /// AST Type.
  pub qualified_type: QualifiedType<'context>,
  pub ir_type: TypeRef<'context>,
  pub data: Data<'context>,
  pub users: Vec<ValueID>,
}

impl<'context> Value<'context> {
  pub fn new(
    qualified_type: QualifiedType<'context>,
    ir_type: TypeRef<'context>,
    value: Data<'context>,
  ) -> Self {
    Self {
      qualified_type,
      ir_type,
      data: value,
      users: Default::default(),
    }
  }
}

use ::rcc_utils::{interconvert, make_trio_for};
interconvert!(Instruction, Data<'context>);
interconvert!(Function, Data, 'context);
interconvert!(Constant, Data, 'context);
interconvert!(Variable, Data, 'context);
interconvert!(BasicBlock, Data<'context>);
interconvert!(Argument, Data<'context>);

make_trio_for!(Instruction, Data<'context>);
make_trio_for!(Function, Data, 'context);
make_trio_for!(Constant, Data, 'context);
make_trio_for!(Variable, Data, 'context);
make_trio_for!(BasicBlock, Data<'context>);
make_trio_for!(Argument, Data<'context>);
