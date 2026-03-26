// C's built-in types
#[derive(
  Debug,
  Clone,
  Copy,
  PartialEq,
  Eq,
  Hash,
  ::strum_macros::Display,
  ::strum_macros::IntoStaticStr,
  ::strum_macros::EnumString,
)]
pub enum Primitive {
  #[strum(serialize = "bool")]
  #[strum(serialize = "_Bool")]
  Bool,
  #[strum(serialize = "char")]
  Char, // plain char
  #[strum(serialize = "signed char")]
  SChar, // signed char
  #[strum(serialize = "short")]
  Short,
  #[strum(serialize = "int")]
  Int,
  #[strum(serialize = "long")]
  Long,
  #[strum(serialize = "long long")]
  LongLong,
  #[strum(serialize = "unsigned char")]
  UChar,
  #[strum(serialize = "unsigned short")]
  UShort,
  #[strum(serialize = "unsigned int")]
  UInt,
  #[strum(serialize = "unsigned long")]
  ULong,
  #[strum(serialize = "unsigned long long")]
  ULongLong,
  #[strum(serialize = "float")]
  Float,
  #[strum(serialize = "double")]
  Double,
  #[strum(serialize = "long double")]
  LongDouble,
  /// 6.2.5.24: The void type comprises an empty set of values; it is an incomplete object type that cannot be completed.
  #[strum(serialize = "void")]
  Void,
  /// 6.5.5.4: `nullptr`. The type `nullptr_t` shall not be converted to any type other than `void`, `bool` or a pointer type.
  #[strum(serialize = "nullptr_t")]
  Nullptr,
  // ignore below for now: __STDC_NO_COMPLEX__
  #[strum(serialize = "_Complex float")]
  ComplexFloat,
  #[strum(serialize = "_Complex")]
  #[strum(serialize = "_Complex double")]
  ComplexDouble,
  #[strum(serialize = "_Complex long double")]
  ComplexLongDouble,
  /// This represent a bit -- 1/8 of byte for IR's `i1` type -- merely a workaround to fix my design.
  ///
  /// # Warning:
  /// **Any attempt to access, create or call to this primitive type using AST-type related function would immediately result in panic.**
  ///
  /// The **ONLY** valid member function is [`TypeInfo::size_bits`], which [`Self::Bool`] returns 8 and [`Self::__IRBit`] returns 1.
  #[strum(disabled)]
  __IRBit,
}
::rcc_utils::ensure_is_pod!(Primitive);

use super::{
  CastType::{self, *},
  TypeInfo,
};
impl Primitive {
  #[must_use]
  pub fn common_type(lhs: &Self, rhs: &Self) -> (Self, CastType, CastType) {
    // If both operands have the same type, then no further conversion is needed.
    // first: _Decimal types ignored
    // also, complex types ignored
    if lhs == rhs {
      return (*lhs, Noop, Noop);
    }
    if matches!(lhs, Self::Void | Self::Nullptr)
      || matches!(rhs, Self::Void | Self::Nullptr)
    {
      panic!("Invalid types for common type: {:?}, {:?}", lhs, rhs);
    }
    // otherwise, if either operand is of some floating type, the other operand is converted to it.
    // Otherwise, if any of the two types is an enumeration, it is converted to its underlying type. - handled upstream
    match (lhs.is_floating_point(), rhs.is_floating_point()) {
      (true, false) => (*lhs, Noop, IntegralToFloating),
      (false, true) => (*rhs, IntegralToFloating, Noop),
      (true, true) => Self::common_floating_rank(*lhs, *rhs),
      (false, false) => Self::common_integer_rank(*lhs, *rhs),
    }
  }

  #[must_use]
  fn common_floating_rank(lhs: Self, rhs: Self) -> (Self, CastType, CastType) {
    assert!(lhs.is_floating_point() && rhs.is_floating_point());
    if lhs.floating_rank() > rhs.floating_rank() {
      (lhs, Noop, FloatingCast)
    } else {
      (rhs, FloatingCast, Noop)
    }
  }

  #[must_use]
  fn common_integer_rank(lhs: Self, rhs: Self) -> (Self, CastType, CastType) {
    assert!(lhs.is_integer() && rhs.is_integer());

    let (lhs, _) = lhs.integer_promotion();
    let (rhs, _) = rhs.integer_promotion();
    if lhs == rhs {
      // done
      return (lhs, Noop, Noop);
    }
    if lhs.is_unsigned() == rhs.is_unsigned() {
      return if lhs.integer_rank() > rhs.integer_rank() {
        (lhs, Noop, IntegralCast)
      } else {
        (rhs, IntegralCast, Noop)
      };
    }
    fn signed_and_unsigned(
      lhs: Primitive,
      rhs: Primitive,
    ) -> (Primitive, CastType, CastType) {
      debug_assert!(!lhs.is_unsigned());
      debug_assert!(rhs.is_unsigned());
      if lhs.integer_rank() >= rhs.integer_rank() {
        (lhs, Noop, IntegralCast)
      } else if rhs.size() > lhs.size() {
        (rhs, IntegralCast, Noop)
      } else {
        // if the signed type cannot represent all values of the unsigned type, return the unsigned version of the signed type
        // the signed type is always larger than the corresponding unsigned type on my x86_64 architecture
        // so this branch is unlikely to be taken
        let promoted_rhs = rhs.into_unsigned();
        (promoted_rhs, IntegralCast, IntegralCast)
      }
    }

    if lhs.is_unsigned() {
      signed_and_unsigned(rhs, lhs)
    } else {
      signed_and_unsigned(lhs, rhs)
    }
  }
}
