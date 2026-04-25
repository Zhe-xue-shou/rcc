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
  Aggregate(Aggregate<'c>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Aggregate<'c> {
  inner: &'c [Constant<'c>],
}
impl<'c> Aggregate<'c> {
  pub fn is_zero(&self) -> bool {
    self.inner.iter().all(|c| c.is_zero())
  }
}

use ::std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address<'c> {
  /// [`DeclNode`] is sufficient, but use [`DeclRef`] to improve clearity at tehe expense of tow indirect.
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
impl RefEq for Constant<'_> {}
impl<'c> Constant<'c> {
  pub const fn is_char_array(&self) -> bool {
    matches!(self, Self::String(_))
  }

  pub fn is_zero(&self) -> bool {
    use Constant::*;
    match self {
      Integral(integral) => integral.is_zero(),
      Floating(floating) => floating.is_zero(),
      String(s) => s.is_empty(),
      Nullptr() => true,
      Address(_) => false,
      Aggregate(aggregate) => aggregate.is_zero(),
    }
  }

  #[inline(always)]
  pub fn is_not_zero(&self) -> bool {
    !self.is_zero()
  }

  pub fn to_boolean(self) -> Self {
    match self {
      Self::Integral(integral) =>
        Integral::from_bool(!integral.is_zero()).into(),
      Self::Floating(floating) =>
        Integral::from_bool(!floating.is_zero()).into(),
      Self::String(s) => Integral::from_bool(s.is_empty()).into(),
      Self::Nullptr() => Integral::from_bool(false).into(),
      Self::Address(_) => Integral::from_bool(true).into(),
      Self::Aggregate(aggregate) =>
        Integral::from_bool(aggregate.is_zero()).into(),
    }
  }

  pub fn to_integral(self, width: u8, signedness: Signedness) -> Self {
    use Constant::*;
    match self {
      Integral(integral) => integral.cast(width, signedness).into(),
      Floating(floating) => floating.to_integral(width, signedness).into(),
      _ => unreachable!("handled elsewhere"),
    }
  }

  pub fn to_floating(self, format: FloatFormat) -> Self {
    use Constant::*;
    match self {
      Integral(integral) => integral.to_floating(format).into(),
      Floating(floating) => floating.into(),
      _ => unreachable!("handled elsewhere"),
    }
  }
}
::rcc_utils::interconvert!(Integral, Constant<'c>);
::rcc_utils::interconvert!(Floating, Constant<'c>);
::rcc_utils::interconvert!(Address, Constant,'c);
::rcc_utils::interconvert!(StrRef, Constant,'c, String);

::rcc_utils::make_trio_for!(Integral, Constant<'c>);
::rcc_utils::make_trio_for!(Floating, Constant<'c>);
::rcc_utils::make_trio_for!(Address, Constant, 'c);
::rcc_utils::make_trio_for!(StrRef, Constant,'c, String);
::rcc_utils::make_trio_for_unit_tuple!(Nullptr, Constant<'c>);
mod fmt {
  use ::std::fmt::{self, Display};

  use super::*;
  impl Display for Constant<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      use Constant::*;
      match self {
        Nullptr() => write!(f, "nullptr"),
        Integral(i) => i.fmt(f),
        Floating(d) => d.fmt(f),
        String(s) => write!(f, "\"{}\"", s),
        Address(addr) => addr.fmt(f),
        Aggregate(aggr) => aggr.fmt(f),
      }
    }
  }
  impl Display for Aggregate<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{{")?;
      self.inner.iter().try_for_each(|c| write!(f, "{}, ", c))?;
      write!(f, "}}")
    }
  }
  impl Display for Address<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      self.inner.fmt(f)
    }
  }
}
