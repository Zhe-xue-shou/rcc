use ::rcc_adt::{FloatFormat, Floating, Integral, Signedness};
use ::rcc_utils::{Opaque, RefEq, StrRef, ensure_is_pod};

/// discrepancy: string literals are not constant values in C `char[N]`
/// (but in C++, it is, though.)
///
/// TODO: named constants `constexpr` and constant aggregate
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constant<'c> {
  Nullptr(),
  Integral(Integral),
  Floating(Floating),
  String(StrRef<'c>),
  Address(Address<'c>),
}

use ::std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address<'c> {
  inner: Opaque,
  /// this shall not be [`Address`] again, except is two base is the same: &a[5] - &a[2].
  offset: Option<ConstantRef<'c>>,
  nothing: PhantomData<StrRef<'c>>,
}

impl<'c> Address<'c> {
  pub fn new(inner: Opaque) -> Self {
    Self {
      inner,
      offset: None,
      nothing: PhantomData,
    }
  }

  pub fn with_offset(inner: Opaque, offset: ConstantRef<'c>) -> Self {
    Self {
      inner,
      offset: Some(offset),
      nothing: PhantomData,
    }
  }

  pub fn has_offset(&self) -> bool {
    self.offset.is_some()
  }

  pub fn offset(&self) -> Option<ConstantRef<'c>> {
    self.offset
  }
}

ensure_is_pod!(Constant);
pub type ConstantRef<'c> = &'c Constant<'c>;
pub type ConstantRefMut<'c> = &'c mut Constant<'c>;
impl RefEq for ConstantRef<'_> {}
impl<'c> Constant<'c> {
  pub const fn is_char_array(&self) -> bool {
    matches!(self, Self::String(_))
  }

  pub fn is_zero(&self) -> bool {
    match self {
      Self::Integral(integral) => integral.is_zero(),
      Self::Floating(floating) => floating.is_zero(),
      Self::String(s) => s.is_empty(),
      Self::Nullptr() => true,
      Self::Address(_) => false,
    }
  }

  #[inline(always)]
  pub fn is_not_zero(&self) -> bool {
    !self.is_zero()
  }

  pub fn to_boolean(self) -> Self {
    match self {
      Self::Integral(integral) =>
        Constant::Integral(Integral::from_bool(!integral.is_zero())),
      Self::Floating(floating) =>
        Constant::Integral(Integral::from_bool(!floating.is_zero())),
      Self::String(s) => Constant::Integral(Integral::from_bool(s.is_empty())),
      Self::Nullptr() => Constant::Integral(Integral::from_bool(false)),
      Self::Address(_) => Constant::Integral(Integral::from_bool(true)),
    }
  }

  pub fn to_integral(self, width: u8, signedness: Signedness) -> Self {
    match self {
      Self::Integral(integral) =>
        Constant::Integral(integral.cast(width, signedness)),
      Self::Floating(floating) =>
        Constant::Integral(floating.to_integral(width, signedness)),
      _ => unreachable!("handled elsewhere"),
    }
  }

  pub fn to_floating(self, format: FloatFormat) -> Self {
    match self {
      Self::Integral(integral) => Self::Floating(integral.to_floating(format)),
      Self::Floating(floating) => Self::Floating(floating),
      _ => unreachable!("handled elsewhere"),
    }
  }
}
::rcc_utils::interconvert!(Integral, Constant<'c>);
::rcc_utils::interconvert!(Floating, Constant<'c>);
// ::rcc_utils::interconvert!(???, Constant, String);
::rcc_utils::interconvert!(Address, Constant,'c);

::rcc_utils::make_trio_for!(Integral, Constant<'c>);
::rcc_utils::make_trio_for!(Floating, Constant<'c>);
::rcc_utils::make_trio_for!(Address, Constant, 'c);
::rcc_utils::make_trio_for_unit_tuple!(Nullptr, Constant<'c>);

// ::rcc_utils::make_trio_for!(???, Constant, String);
// ::rcc_utils::make_trio_for!(???, Constant, Address);

impl ::std::fmt::Display for Constant<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use Constant::*;
    match self {
      Integral(i) => write!(f, "{i}"),
      Floating(d) => write!(f, "{d}"),
      String(s) => write!(f, "\"{}\"", s),
      Address(addr) => write!(f, "{}", addr.inner),
      Nullptr() => write!(f, "nullptr"),
    }
  }
}
