use ::rcc_ast::types as ast;
use ::rcc_shared::Constant;
use ::slotmap::{Key, new_key_type};
use ::std::cell::{Ref, RefMut};

use super::{
  Argument, BasicBlock, TypeRef,
  instruction::{Instruction, User},
  module::{Function, Variable},
};

new_key_type! {
    /// size = 8 (0x08), align = 0x8, no Drop
    pub struct ValueID;
}

impl ValueID {
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
  pub fn unwrap(self) -> Self {
    if self.is_null() {
      panic!("id is null")
    } else {
      self
    }
  }

  #[inline]
  pub fn unwrap_or_else<F: FnOnce() -> Self>(self, f: F) -> Self {
    if self.is_null() { f() } else { self }
  }

  #[inline]
  pub fn unwrap_or(self, alt: Self) -> Self {
    if self.is_null() { alt } else { self }
  }

  #[inline]
  pub fn or_else<F: FnOnce() -> Self>(self, f: F) -> Self {
    if self.is_null() { f() } else { self }
  }

  #[inline]
  pub fn handle(&self) -> u64 {
    self.0.as_ffi()
  }

  #[inline]
  pub fn to_option(self) -> Option<Self> {
    if self.is_null() { None } else { Some(self) }
  }

  #[inline]
  pub fn null() -> Self {
    <Self as Key>::null()
  }

  #[inline]
  pub fn is_null(&self) -> bool {
    <Self as Key>::is_null(self)
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
impl User for Data<'_> {
  fn use_list(&self) -> &[ValueID] {
    ::rcc_utils::static_dispatch!(
      self, |variant| variant.use_list() =>
      Instruction Constant Function Variable BasicBlock Argument
    )
  }
}

impl User for Argument {
  fn use_list(&self) -> &[ValueID] {
    &[]
  }
}
impl User for BasicBlock {
  fn use_list(&self) -> &[ValueID] {
    &[]
  }
}

impl User for Constant<'_> {
  fn use_list(&self) -> &[ValueID] {
    // TODO
    &[]
  }
}
impl User for Function<'_> {
  fn use_list(&self) -> &[ValueID] {
    // TODO
    &[]
  }
}

impl User for Variable<'_> {
  fn use_list(&self) -> &[ValueID] {
    // TODO
    &[]
  }
}

#[derive(Debug)]
pub struct Value<'c> {
  /// AST Type.
  pub ast_type: ast::TypeRef<'c>,
  pub ir_type: TypeRef<'c>,
  pub data: Data<'c>,
  pub parent: ValueID,
}

impl User for Value<'_> {
  fn use_list(&self) -> &[ValueID] {
    self.data.use_list()
  }
}

impl<'c> Value<'c> {
  pub fn new(
    ast_type: ast::TypeRef<'c>,
    ir_type: TypeRef<'c>,
    value: impl Into<Data<'c>>,
    parent: ValueID,
  ) -> Self {
    Self {
      ast_type,
      ir_type,
      data: value.into(),
      parent,
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
