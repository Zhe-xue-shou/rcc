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
pub enum Data<'c> {
  Instruction(Instruction),
  Constant(Constant<'c>),
  Function(Function<'c>),
  Variable(Variable<'c>),
  BasicBlock(BasicBlock),
  Argument(Argument),
}

#[derive(Debug)]
pub struct Value<'c> {
  /// AST Type.
  pub qualified_type: QualifiedType<'c>,
  pub ir_type: TypeRef<'c>,
  pub data: Data<'c>,
  pub users: Vec<ValueID>,
}

impl<'c> Value<'c> {
  pub fn new(
    qualified_type: QualifiedType<'c>,
    ir_type: TypeRef<'c>,
    value: Data<'c>,
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
interconvert!(Instruction, Data<'c>);
interconvert!(Function, Data, 'c);
interconvert!(Constant, Data, 'c);
interconvert!(Variable, Data, 'c);
interconvert!(BasicBlock, Data<'c>);
interconvert!(Argument, Data<'c>);

make_trio_for!(Instruction, Data<'c>);
make_trio_for!(Function, Data, 'c);
make_trio_for!(Constant, Data, 'c);
make_trio_for!(Variable, Data, 'c);
make_trio_for!(BasicBlock, Data<'c>);
make_trio_for!(Argument, Data<'c>);
