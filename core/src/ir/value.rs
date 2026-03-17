use super::{
  Argument, BasicBlock, TypeRef,
  instruction::Instruction,
  module::{Function, Variable},
};
use crate::types::{Constant, QualifiedType};

::slotmap::new_key_type! {
    /// size = 8 (0x08), align = 0x8, no Drop
    pub struct ValueID;
}

impl ValueID {
  pub fn handle(&self) -> u64 {
    self.0.as_ffi()
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
