use ::rcc_ast::types as ast;
use ::slotmap::{Key, new_key_type};

use super::{
  BasicBlock, ConstantData, GlobalValue, TypeRef,
  constant::Constant,
  global::{Function, Variable},
  instruction::{Instruction, User},
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
pub struct Arguments {
  /// Shall be [`Function`].
  pub index: usize,
}

impl Arguments {
  pub fn new(index: usize) -> Self {
    Self { index }
  }
}

#[derive(Debug)]
pub enum Data<'c> {
  Instruction(Instruction),
  Constant(Constant<'c>),
  BasicBlock(BasicBlock),
  Arguments(Arguments),
}
impl User for Data<'_> {
  fn use_list(&self) -> &[ValueID] {
    ::rcc_utils::static_dispatch!(
      self, |variant| variant.use_list() =>
      Instruction Constant BasicBlock Arguments
    )
  }
}

impl User for Arguments {
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
    ::rcc_utils::static_dispatch!(
      self, |variant| variant.use_list() =>
      Data Global
    )
  }
}
impl User for ConstantData<'_> {
  fn use_list(&self) -> &[ValueID] {
    // TODO
    &[]
  }
}
impl User for GlobalValue<'_> {
  fn use_list(&self) -> &[ValueID] {
    ::rcc_utils::static_dispatch!(
      self, |variant| variant.use_list() =>
      Function Variable
    )
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

mod cvt {
  use ::rcc_utils::{interconvert, make_trio_for};

  use super::*;

  interconvert!(Instruction, Data<'c>);
  interconvert!(Constant, Data, 'c);
  interconvert!(BasicBlock, Data<'c>);
  interconvert!(Arguments, Data<'c>);

  make_trio_for!(Instruction, Data<'c>);
  make_trio_for!(Constant, Data, 'c);
  make_trio_for!(BasicBlock, Data<'c>);
  make_trio_for!(Arguments, Data<'c>);

  impl<'c> From<ConstantData<'c>> for Data<'c> {
    fn from(value: ConstantData<'c>) -> Self {
      Constant::from(value).into()
    }
  }
}
