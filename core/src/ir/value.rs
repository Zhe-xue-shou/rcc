use ::slotmap::{Key, new_key_type};
use ::std::cell::{Ref, RefMut};

use super::{
  Argument, BasicBlock, TypeRef,
  instruction::Instruction,
  module::{Function, Variable},
};
use crate::types::{Constant, QualifiedType};
new_key_type! {
    /// size = 8 (0x08), align = 0x8, no Drop
    pub struct ValueID;
}

impl ValueID {
  #[inline]
  pub fn is_none(&self) -> bool {
    self.is_null()
  }

  #[inline]
  pub fn is_some_and<F: FnOnce(Self) -> bool>(self, f: F) -> bool {
    !self.is_null() && f(self)
  }

  #[inline]
  pub fn and_then<F: FnOnce(Self) -> Self>(self, f: F) -> Self {
    if self.is_null() {
      Self::null()
    } else {
      f(self)
    }
  }

  #[inline]
  pub fn unwrap_or_else<F: FnOnce() -> Self>(self, f: F) -> Self {
    if self.is_null() { f() } else { self }
  }

  #[inline]
  pub fn or_else<F: FnOnce() -> Self>(self, f: F) -> Self {
    if self.is_null() { f() } else { self }
  }

  #[inline]
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
  pub use_list: Vec<ValueID>,
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
      use_list: Default::default(),
    }
  }

  pub fn with_users(
    qualified_type: QualifiedType<'c>,
    ir_type: TypeRef<'c>,
    value: Data<'c>,
    use_list: Vec<ValueID>,
  ) -> Self {
    Self {
      qualified_type,
      ir_type,
      data: value,
      use_list,
    }
  }
}
pub(super) trait WithActionMut<T> {
  fn with_action_mut<R, F: FnOnce(&mut T) -> R>(&mut self, f: F) -> R;
}
impl<'c, T> WithActionMut<T> for RefMut<'c, T> {
  fn with_action_mut<R, F: FnOnce(&mut T) -> R>(&mut self, f: F) -> R {
    f(self)
  }
}
pub(super) trait WithAction<T> {
  fn with_action<R, F: FnOnce(&T) -> R>(&self, f: F) -> R;
}
impl<'c, T> WithAction<T> for RefMut<'c, T> {
  fn with_action<R, F: FnOnce(&T) -> R>(&self, f: F) -> R {
    f(self)
  }
}
impl<'c, T> WithAction<T> for Ref<'c, T> {
  fn with_action<R, F: FnOnce(&T) -> R>(&self, f: F) -> R {
    f(self)
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
