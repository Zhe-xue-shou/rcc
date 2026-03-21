use ::rcc_utils::{
  BuiltinFloat, BuiltinIntegerOrBoolean, NumFrom, NumTo, ToU128, ensure_is_pod,
  signed_type_of,
};

type Underlying = u128;
type SignedUnderlying = signed_type_of!(u128);

/// Signedness of an integer type.
#[derive(Debug, Copy, Hash)]
#[derive_const(Clone, PartialEq, Eq)]
pub enum Signedness {
  Unsigned = 0,
  Signed = 1,
}
use Signedness::*;

impl Signedness {
  #[inline(always)]
  pub const fn is_signed(self) -> bool {
    matches!(self, Signed)
  }

  #[inline(always)]
  pub const fn is_unsigned(self) -> bool {
    matches!(self, Unsigned)
  }
}

impl From<bool> for Signedness {
  fn from(signed: bool) -> Self {
    if signed { Signed } else { Unsigned }
  }
}

/// A width-aware integer that can represent any C integer type, inspired by
/// [LLVM/Clang's APInt](https://github.com/llvm/llvm-project/blob/main/llvm/include/llvm/ADT/APInt.h?=#L78),
/// this provides a unified representation for all integer constants.
///
/// The value is always stored in the lower [`Integral::width`] bits of [`Integral::bits`].
/// For signed interpretation, use [`Integral::as_signed`].
///
/// This class is designed to by *trivially copyable* and *const-evaluable*,
/// all methods are taking either `self` or `&self` and return a new `Integral`, no internal mutation.
///
/// Also, methods w.r.t. 2 or more [`Integral`]s (e.g. [`Integral::overflowing_add`]) needs to ensure
/// that the operands have the same width and signedness, otherwise panic.
///
/// It does not support 0-width integers, and the current maximum width is 128 bits.
#[derive(Copy, Hash)]
#[derive_const(Clone, PartialEq, Eq)]
pub struct Integral {
  /// The raw bits, stored in the lower `width` bits.
  ///
  /// The underlying storage type for all integer values.
  /// Using u128 allows us to represent all C integer types (up to 64-bit)
  /// with room for future extensions.
  bits: Underlying,
  /// The bit width of this integer (1-128).
  width: u8,
  /// Whether this integer should be interpreted as signed.
  signedness: Signedness,
}

ensure_is_pod!(Integral);

const fn consteval(a: bool, b: bool) {
  assert!(a);
  assert!(b);
}
fn notconsteval(a: bool, b: bool) {
  debug_assert!(a);
  debug_assert!(b);
}

impl Integral {
  pub const WIDTH_BOOL: u8 = 1;
  pub const WIDTH_CHAR: u8 = 8;
  pub const WIDTH_INT: u8 = 32;
  // long and long long are both 64-bit in my implementation here(ref linux x86-64 version.)
  pub const WIDTH_LONG: u8 = 64;
  pub const WIDTH_LONG_LONG: u8 = 64;
  pub const WIDTH_SHORT: u8 = 16;
  pub const WIDTH_UINTPTR: u8 = 64;
}
impl Integral {
  pub const BIT_FALSE: Self = Self::new(0, 1, Unsigned);
  pub const BIT_TRUE: Self = Self::new(1, 1, Unsigned);
  pub const FALSE: Self = Self::new(0, Self::WIDTH_BOOL, Unsigned);
  pub const TRUE: Self = Self::new(1, Self::WIDTH_BOOL, Unsigned);
}
impl Integral {
  /// Create a new integral value, automatically masking to the specified width.
  #[inline]
  pub const fn new<T: [const] ToU128 + BuiltinIntegerOrBoolean>(
    value: T,
    width: u8,
    signedness: Signedness,
  ) -> Self {
    debug_assert!(width > 0 && width <= 128, "width must be 1-128");
    Self {
      bits: Self::mask(value.to_u128(), width),
      width,
      signedness,
    }
  }

  #[inline]
  pub const fn from_signed<T: [const] ToU128 + BuiltinIntegerOrBoolean>(
    value: T,
    width: u8,
  ) -> Self {
    Self::new(value, width, Signed)
  }

  #[inline]
  pub const fn from_unsigned<T: [const] ToU128 + BuiltinIntegerOrBoolean>(
    value: T,
    width: u8,
  ) -> Self {
    Self::new(value, width, Unsigned)
  }

  // Convenience constructors for C types
  #[inline]
  pub const fn from_bool(value: bool) -> Self {
    Self::new(value, Self::WIDTH_BOOL, Unsigned)
  }

  #[inline]
  pub const fn from_uintptr(value: usize) -> Self {
    Self::new(value, Self::WIDTH_UINTPTR, Unsigned)
  }

  #[inline]
  pub const fn bitmask(width: u8) -> Self {
    debug_assert!(width > 0 && width <= 128, "width must be 1-128");
    Self::new(Self::mask(u128::MAX, width), width, Unsigned)
  }

  #[inline]
  pub const fn i1_true() -> Self {
    Self::new(1, 1, Unsigned)
  }

  #[inline]
  pub const fn i1_false() -> Self {
    Self::new(0, 1, Unsigned)
  }
}
impl Integral {
  #[inline(always)]
  pub const fn bits(&self) -> Underlying {
    self.bits
  }

  #[inline(always)]
  pub const fn width(&self) -> u8 {
    self.width
  }

  #[inline(always)]
  pub const fn signedness(&self) -> Signedness {
    self.signedness
  }

  #[inline(always)]
  pub const fn is_signed(&self) -> bool {
    self.signedness.is_signed()
  }

  #[inline(always)]
  pub const fn is_unsigned(&self) -> bool {
    self.signedness.is_unsigned()
  }

  /// sign-extended to i128.
  #[inline]
  pub const fn as_signed(&self) -> i128 {
    if self.width == 128 {
      self.bits as i128
    } else {
      let shift = 128 - self.width;
      ((self.bits as i128) << shift) >> shift
    }
  }

  /// get the bits as unsigned, same as [`Integral::bits()`].
  #[inline]
  pub const fn as_unsigned(&self) -> u128 {
    self.bits
  }

  // /// Get the value respecting signedness.
  // /// Returns (value as i128, whether it's actually negative).
  // #[inline]
  // pub const fn value(&self) -> i128 {
  //   if self.is_signed() {
  //     self.as_signed()
  //   } else {
  //     self.bits as i128
  //   }
  // }

  #[inline]
  pub const fn is_zero(&self) -> bool {
    self.bits == 0
  }

  #[inline]
  pub const fn is_one(&self) -> bool {
    self.bits == 1
  }

  /// Check if the sign bit is set.
  #[inline]
  pub const fn sign_bit(&self) -> bool {
    if self.width == 128 {
      (self.bits as i128) < 0
    } else {
      (self.bits >> (self.width - 1)) & 1 != 0
    }
  }

  /// Check if value is negative **(only meaningful for signed)**.
  #[inline]
  pub const fn is_negative(&self) -> bool {
    self.is_signed() && self.sign_bit()
  }

  /// Check if value is positive (> 0).
  #[inline]
  pub const fn is_positive(&self) -> bool {
    !self.is_zero() && !self.is_negative()
  }

  /// Get the minimum value for this width and signedness.
  pub const fn min_value(width: u8, signedness: Signedness) -> Self {
    match signedness {
      Unsigned => Self::new(0, width, signedness),
      Signed => {
        let min = 1u128 << (width - 1);
        Self::new(min, width, signedness)
      },
    }
  }

  /// Get the maximum value for this width and signedness.
  pub const fn max_value(width: u8, signedness: Signedness) -> Self {
    match signedness {
      Unsigned => Self::new(Self::max_unsigned(width), width, signedness),
      Signed => Self::new((1u128 << (width - 1)) - 1, width, signedness),
    }
  }

  /// Cast to a different width and/or signedness.
  /// This performs truncation or extension as appropriate.
  pub const fn cast(self, new_width: u8, new_signedness: Signedness) -> Self {
    let new_bits = if new_width >= self.width {
      // Extension
      if self.is_signed() && self.sign_bit() {
        // Sign extension
        let extension_mask =
          Self::max_unsigned(new_width) ^ Self::max_unsigned(self.width);
        self.bits | extension_mask
      } else {
        // zero extension
        self.bits
      }
    } else {
      // truncation
      self.bits
    };

    Self::new(new_bits, new_width, new_signedness)
  }

  /// Change signedness without changing the bits.
  #[inline]
  pub const fn reinterpret(self, signedness: Signedness) -> Self {
    Self { signedness, ..self }
  }

  /// Zero-extend to a wider type.
  #[inline]
  pub const fn zext(self, new_width: u8) -> Self {
    debug_assert!(new_width >= self.width);
    Self::new(self.bits, new_width, Unsigned)
  }

  /// Sign-extend to a wider type.
  #[inline]
  pub const fn sext(self, new_width: u8) -> Self {
    debug_assert!(new_width >= self.width);
    self.cast(new_width, Signed)
  }

  /// Truncate to a narrower type.
  #[inline]
  pub const fn trunc(self, new_width: u8, signedness: Signedness) -> Self {
    debug_assert!(new_width <= self.width);
    Self::new(self.bits, new_width, signedness)
  }

  #[inline]
  pub const fn to_builtin<
    T: [const] BuiltinIntegerOrBoolean
      + [const] NumFrom<Underlying>
      + [const] NumFrom<SignedUnderlying>,
  >(
    &self,
  ) -> T {
    if self.is_signed() {
      self.as_signed().to()
    } else {
      self.bits.to()
    }
  }

  #[inline]
  pub const fn to_builtin_float<
    T: [const] BuiltinFloat
      + [const] NumFrom<Underlying>
      + [const] NumFrom<SignedUnderlying>,
  >(
    &self,
  ) -> T {
    if self.is_signed() {
      self.as_signed().to()
    } else {
      self.bits.to()
    }
  }
}
impl Integral {
  /// Add with overflow detection.
  pub const fn overflowing_add(self, rhs: Self) -> (Self, bool) {
    const_eval_select(
      (self.signedness == rhs.signedness, self.width == rhs.width),
      consteval,
      notconsteval,
    );

    let sum = self.bits.wrapping_add(rhs.bits);
    let result = Self::new(sum, self.width, self.signedness);

    let overflow = if self.is_signed() {
      // signed overflow: signs of operands are same, but result sign differs
      let a_neg = self.sign_bit();
      let b_neg = rhs.sign_bit();
      let r_neg = result.sign_bit();
      (a_neg == b_neg) && (a_neg != r_neg)
    } else {
      // unsigned overflow: result is smaller than either operand
      result.bits < self.bits
    };

    (result, overflow)
  }

  /// Subtract with overflow detection.
  pub const fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
    const_eval_select(
      (self.signedness == rhs.signedness, self.width == rhs.width),
      consteval,
      notconsteval,
    );

    let diff = self.bits.wrapping_sub(rhs.bits);
    let result = Self::new(diff, self.width, self.signedness);

    let overflow = if self.is_signed() {
      // signed overflow.
      // a - b overflows if a and b have different signs and the result has the same sign as b
      let a_neg = self.sign_bit();
      let b_neg = rhs.sign_bit();
      let r_neg = result.sign_bit();
      (a_neg != b_neg) && (b_neg == r_neg)
    } else {
      // unsigned underflow: rhs > self
      rhs.bits > self.bits
    };

    (result, overflow)
  }

  /// Multiply with overflow detection.
  pub const fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
    const_eval_select(
      (self.signedness == rhs.signedness, self.width == rhs.width),
      consteval,
      notconsteval,
    );

    let (product, overflow) = if self.is_signed() {
      let a = self.as_signed();
      let b = rhs.as_signed();
      let (p, o) = a.overflowing_mul(b);
      (p as u128, o)
    } else {
      self.bits.overflowing_mul(rhs.bits)
    };

    let result = Self::new(product, self.width, self.signedness);

    // check if truncation lost bits
    let truncation_overflow = if self.is_signed() {
      let extended = result.as_signed();
      extended != (product as i128)
    } else {
      result.bits != product
    };

    (result, overflow || truncation_overflow)
  }

  /// Divide, returns None on division by zero.
  pub const fn checked_div(self, rhs: Self) -> Option<Self> {
    const_eval_select(
      (self.signedness == rhs.signedness, self.width == rhs.width),
      consteval,
      notconsteval,
    );

    if rhs.is_zero() {
      None?
    }

    let quotient = if self.is_signed() {
      (self.as_signed() / rhs.as_signed()) as u128
    } else {
      self.bits / rhs.bits
    };

    Some(Self::new(quotient, self.width, self.signedness))
  }

  /// Remainder, returns [`None`] on division by zero.
  pub const fn checked_rem(self, rhs: Self) -> Option<Self> {
    const_eval_select(
      (self.signedness == rhs.signedness, self.width == rhs.width),
      consteval,
      notconsteval,
    );

    if rhs.is_zero() {
      None?
    }

    let remainder = if self.is_signed() {
      (self.as_signed() % rhs.as_signed()) as u128
    } else {
      self.bits % rhs.bits
    };

    Some(Self::new(remainder, self.width, self.signedness))
  }

  /// Logical right shift, always zero-fill.
  pub const fn lshr(self, amount: u32) -> Self {
    let amount = amount.min(self.width.to());
    Self::new(self.bits >> amount, self.width, self.signedness)
  }

  /// Arithmetic right shift, always sign-fill.
  pub const fn ashr(self, amount: u32) -> Self {
    Self::new(
      (self.as_signed() >> amount.min(self.width.to())) as u128,
      self.width,
      self.signedness,
    )
  }

  #[inline]
  const fn mask(value: Underlying, width: u8) -> Underlying {
    if width >= 128 {
      value
    } else {
      value & ((1u128 << width) - 1)
    }
  }

  #[inline]
  const fn max_unsigned(width: u8) -> Underlying {
    if width >= 128 {
      Underlying::MAX
    } else {
      (1u128 << width) - 1
    }
  }
}

use ::std::{
  intrinsics::const_eval_select,
  ops::{Add, BitAnd, BitOr, BitXor, Mul, Neg, Not, Shl, Shr, Sub},
};

impl const Add for Integral {
  type Output = Self;

  #[inline]
  fn add(self, rhs: Self) -> Self {
    self.overflowing_add(rhs).0
  }
}

impl const Sub for Integral {
  type Output = Self;

  #[inline]
  fn sub(self, rhs: Self) -> Self {
    self.overflowing_sub(rhs).0
  }
}

impl const Mul for Integral {
  type Output = Self;

  #[inline]
  fn mul(self, rhs: Self) -> Self {
    self.overflowing_mul(rhs).0
  }
}

impl const Neg for Integral {
  type Output = Self;

  /// Negate (two's complement).
  #[inline]
  fn neg(self) -> Self {
    Self::new((!self.bits).wrapping_add(1), self.width, self.signedness)
  }
}

impl const Not for Integral {
  type Output = Self;

  #[inline]
  fn not(self) -> Self {
    Self::new(!self.bits, self.width, self.signedness)
  }
}

impl const BitAnd for Integral {
  type Output = Self;

  #[inline]
  fn bitand(self, rhs: Self) -> Self {
    {
      const_eval_select(
        (true, self.width == rhs.width),
        consteval,
        notconsteval,
      );
      Self::new(self.bits & rhs.bits, self.width, self.signedness)
    }
  }
}

impl const BitOr for Integral {
  type Output = Self;

  #[inline]
  fn bitor(self, rhs: Self) -> Self {
    const_eval_select((true, self.width == rhs.width), consteval, notconsteval);
    Self::new(self.bits | rhs.bits, self.width, self.signedness)
  }
}

impl const BitXor for Integral {
  type Output = Self;

  #[inline]
  fn bitxor(self, rhs: Self) -> Self {
    const_eval_select((true, self.width == rhs.width), consteval, notconsteval);
    Self::new(self.bits ^ rhs.bits, self.width, self.signedness)
  }
}

impl const Shl<u32> for Integral {
  type Output = Self;

  #[inline]
  fn shl(self, rhs: u32) -> Self {
    let amount = rhs.min(self.width as u32);
    Self::new(self.bits << amount, self.width, self.signedness)
  }
}

impl const Shr<u32> for Integral {
  type Output = Self;

  #[inline]
  fn shr(self, rhs: u32) -> Self {
    let amount = rhs.min(self.width as u32);
    let result = if self.is_signed() {
      (self.as_signed() >> amount) as u128
    } else {
      self.bits >> amount
    };
    Self::new(result, self.width, self.signedness)
  }
}

impl const PartialOrd for Integral {
  fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
    if self.width != other.width || self.signedness != other.signedness {
      None
    } else {
      const_eval_select(
        (
          self.signedness == other.signedness,
          self.width == other.width,
        ),
        consteval,
        notconsteval,
      );

      if self.is_signed() {
        self.as_signed().cmp(&other.as_signed())
      } else {
        self.bits.cmp(&other.bits)
      }
      .into()
    }
  }
}
mod fmt {
  use ::std::fmt;

  use super::Integral;
  impl fmt::Debug for Integral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      if self.is_signed() {
        write!(f, "{}i{}", self.as_signed(), self.width)
      } else {
        write!(f, "{}u{}", self.bits, self.width)
      }
    }
  }

  impl fmt::Display for Integral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      if self.is_signed() {
        self.as_signed().fmt(f)
      } else {
        self.bits.fmt(f)
      }
    }
  }

  impl Default for Integral {
    #[inline]
    fn default() -> Self {
      Self::from(0)
    }
  }
}

macro_rules! impl_from_integral {
  ($t:ty, $width:expr, $signedness:expr) => {
    impl From<$t> for Integral {
      #[inline(always)]
      fn from(value: $t) -> Self {
        Integral::new(value, $width as u8, $signedness)
      }
    }
  };
}
impl_from_integral!(bool, 1, Unsigned);
impl_from_integral!(i8, i8::BITS, Signed);
impl_from_integral!(u8, u8::BITS, Unsigned);
impl_from_integral!(i16, i16::BITS, Signed);
impl_from_integral!(u16, u16::BITS, Unsigned);
impl_from_integral!(i32, i32::BITS, Signed);
impl_from_integral!(u32, u32::BITS, Unsigned);
impl_from_integral!(i64, i64::BITS, Signed);
impl_from_integral!(u64, u64::BITS, Unsigned);
impl_from_integral!(i128, i128::BITS, Signed);
impl_from_integral!(u128, u128::BITS, Unsigned);
impl_from_integral!(isize, isize::BITS, Signed);
impl_from_integral!(usize, usize::BITS, Unsigned);

#[cfg(test)]
#[allow(clippy::unnecessary_cast)]
mod tests {
  #[allow(unused)]
  use super::*;

  #[test]
  fn test_sign_extension() {
    let neg_one_i8 = Integral::from(-1 as i8);
    assert_eq!(neg_one_i8.as_signed(), -1);
    assert_eq!(neg_one_i8.bits(), 0xFF);

    let extended = neg_one_i8.sext(32);
    assert_eq!(extended.as_signed(), -1);
    assert_eq!(extended.bits(), 0xFFFFFFFF);
  }

  #[test]
  fn test_truncation() {
    let big = Integral::from(0x12345678 as i32);
    let small = big.trunc(8, Unsigned);
    assert_eq!(small.bits(), 0x78);
  }

  #[test]
  fn test_overflow_detection() {
    let max_i8 = Integral::new(127, 8, Signed);
    let one = Integral::new(1, 8, Signed);
    let (result, overflow) = max_i8.overflowing_add(one);
    assert!(overflow);
    assert_eq!(result.as_signed(), -128);
  }

  #[test]
  fn test_signed_comparison() {
    let neg = Integral::from(-1 as i8);
    let pos = Integral::from(1 as i8);
    assert!(neg < pos);
  }

  #[test]
  fn test_unsigned_comparison() {
    let a = Integral::from(255 as u8);
    let b = Integral::from(1 as u8);
    assert!(a > b);
  }
}
