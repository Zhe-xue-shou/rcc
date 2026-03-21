use ::rcc_utils::{
  BuiltinFloat, NumTo, ToI128, ToU128, ensure_is_pod, underlying_type_of,
};

#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Hash, ::std::marker::ConstParamTy,
)]
pub enum Format {
  /// standard IEEE float.
  IEEE32 = 32,
  /// standard IEEE double.
  IEEE64 = 64,
}
// IEEE128,      // 'long double' Quad precision

use Format::*;

// union Bits {
//   ieee32: f32,
//   ieee64: f64,
// }

/// you can see the doc of [`Integral`] for more information. Unlike the feature-rich [`Integral`],
/// this struct is essentially a simple wrapper around the raw bits of a floating-point number, along with its format.
/// That being said, I didn't reference
/// [LLVM's APFloat](https://github.com/llvm/llvm-project/blob/main/llvm/include/llvm/ADT/APFloat.h) at all,
/// which is a far more comprehensive and complex implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Floating {
  // i dont thinks its need ed to use the u128 jere, just a tagger union would be fine.
  bits: u128,
  format: Format,
}

ensure_is_pod!(Floating);

impl ::std::fmt::Display for Floating {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    match self.format {
      IEEE32 => f32::from_bits(self.bits as underlying_type_of!(f32)).fmt(f),
      IEEE64 => f64::from_bits(self.bits as underlying_type_of!(f64)).fmt(f),
    }
  }
}

impl ::std::fmt::LowerExp for Floating {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.format {
      IEEE32 => f32::from_bits(self.bits as underlying_type_of!(f32)).fmt(f),
      IEEE64 => f64::from_bits(self.bits as underlying_type_of!(f64)).fmt(f),
    }
  }
}
impl Floating {
  pub const fn new<T: [const] ::rcc_utils::ToU128>(
    bits: T,
    format: Format,
  ) -> Self {
    Self {
      bits: bits.to_u128(),
      format,
    }
  }

  pub const fn format(&self) -> Format {
    self.format
  }

  pub const fn zero(format: Format) -> Self {
    match format {
      IEEE32 => Floating::from(0.0f32),
      IEEE64 => Floating::from(0.0f64),
    }
  }
}

impl Floating {
  pub fn is_zero(&self) -> bool {
    match self.format {
      // pos zero: 0x00000000, neg zero: 0x80000000
      IEEE32 => (self.bits as underlying_type_of!(f32) & 0x7FFF_FFFF) == 0,
      IEEE64 =>
        (self.bits as underlying_type_of!(f64) & 0x7FFF_FFFF_FFFF_FFFF) == 0,
    }
  }

  pub fn is_infinite(&self) -> bool {
    match self.format {
      // Exponent all 1s, Fraction all 0s
      IEEE32 =>
        (self.bits as underlying_type_of!(f32) & 0x7FFF_FFFF) == 0x7F80_0000,
      IEEE64 =>
        (self.bits as underlying_type_of!(f64) & 0x7FFF_FFFF_FFFF_FFFF)
          == 0x7FF0_0000_0000_0000,
    }
  }

  pub fn is_nan(&self) -> bool {
    match self.format {
      // Exponent all 1s, Fraction non-zero
      IEEE32 =>
        (self.bits as underlying_type_of!(f32) & 0x7FFF_FFFF) > 0x7F80_0000,
      IEEE64 =>
        (self.bits as underlying_type_of!(f64) & 0x7FFF_FFFF_FFFF_FFFF)
          > 0x7FF0_0000_0000_0000,
    }
  }

  pub fn is_finite(&self) -> bool {
    match self.format {
      // Exponent is NOT all 1s
      IEEE32 =>
        (self.bits as underlying_type_of!(f32) & 0x7F80_0000) != 0x7F80_0000,
      IEEE64 =>
        (self.bits as underlying_type_of!(f64) & 0x7FF0_0000_0000_0000)
          != 0x7FF0_0000_0000_0000,
    }
  }

  pub fn cast(self, format: Format) -> Floating {
    match (self.format, format) {
      (IEEE32, IEEE64) => {
        let f = f32::from_bits(self.bits as underlying_type_of!(f32));
        Floating::from(f as f64)
      },
      (IEEE64, IEEE32) => {
        let f = f64::from_bits(self.bits as underlying_type_of!(f64));
        Floating::from(f as f32)
      },
      (IEEE32, IEEE32) | (IEEE64, IEEE64) => self,
    }
  }
}

impl const From<f32> for Floating {
  fn from(val: f32) -> Self {
    Floating::new(val.to_bits() as u128, IEEE32)
  }
}

impl const From<f64> for Floating {
  fn from(val: f64) -> Self {
    Floating::new(val.to_bits() as u128, IEEE64)
  }
}

impl ::std::default::Default for Floating {
  fn default() -> Self {
    Self {
      bits: f64::default().to_bits() as u128,
      format: IEEE64,
    }
  }
}

use ::std::ops::{Add, Div, Mul, Neg, Not, Sub};

use super::{Integral, Signedness};

macro_rules! impl_op {
  ($trait:ident, $method:ident, $op:tt) => {
    impl $trait for Floating {
      type Output = Self;
      #[inline]
      fn $method(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(
          self.format, rhs.format,
          "Cannot perform operation on Floating values of different formats"
        );
        match self.format {
          IEEE32 => {
            Floating::from(
              f32::from_bits(self.bits as underlying_type_of!(f32))
                $op f32::from_bits(rhs.bits as underlying_type_of!(f32)),
            )
          },
          IEEE64 => {
            Floating::from(
              f64::from_bits(self.bits as underlying_type_of!(f64))
                $op f64::from_bits(rhs.bits as underlying_type_of!(f64)),
            )
          },
        }
      }
    }
  };
}

macro_rules! impl_all_ops {
  ($($trait:ident, $method:ident, $op:tt);* $(;)?) => {
    $(
      impl_op!($trait, $method, $op);
    )*
  };
}

impl_all_ops! {
  Add, add, +;
  Sub, sub, -;
  Mul, mul, *;
  Div, div, /;
}

impl Neg for Floating {
  type Output = Self;

  fn neg(self) -> Self::Output {
    match self.format {
      IEEE32 => {
        let lhs_f = f32::from_bits(self.bits as underlying_type_of!(f32));
        Floating::from(-lhs_f)
      },
      IEEE64 => {
        let lhs_f = f64::from_bits(self.bits as underlying_type_of!(f64));
        Floating::from(-lhs_f)
      },
    }
  }
}

impl Not for Floating {
  type Output = bool;

  #[inline(always)]
  fn not(self) -> Self::Output {
    self.is_zero()
  }
}

impl PartialOrd for Floating {
  fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
    debug_assert_eq!(
      self.format, other.format,
      "Cannot compare Floating values of different formats"
    );
    match self.format {
      IEEE32 => f32::partial_cmp(
        &f32::from_bits(self.bits as underlying_type_of!(f32)),
        &f32::from_bits(other.bits as underlying_type_of!(f32)),
      ),
      IEEE64 => f64::partial_cmp(
        &f64::from_bits(self.bits as underlying_type_of!(f64)),
        &f64::from_bits(other.bits as underlying_type_of!(f64)),
      ),
    }
  }
}

impl Integral {
  #[inline]
  pub fn to_floating(self, format: Format) -> Floating {
    match format {
      IEEE32 => Floating::from(self.to_builtin::<u32>() as f32),
      IEEE64 => Floating::from(self.to_builtin::<u64>() as f64),
    }
  }
}

impl Floating {
  #[inline]
  pub fn to_integral(self, width: u8, signedness: Signedness) -> Integral {
    debug_assert!(width > 0 && width <= 128);
    match (self.format, signedness) {
      // Float to signed
      (IEEE32, Signedness::Signed) => {
        let f = f32::from_bits(self.bits.to());
        // Clamp to target range to avoid UB
        let clamped = clamp_float_to_signed(f, width);
        Integral::from_signed(clamped, width)
      },
      (IEEE64, Signedness::Signed) => {
        let f = f64::from_bits(self.bits.to());
        let clamped = clamp_float_to_signed(f, width);
        Integral::from_signed(clamped, width)
      },
      // Float to unsigned
      (IEEE32, Signedness::Unsigned) => {
        let f = f32::from_bits(self.bits.to());
        let clamped = clamp_float_to_unsigned(f, width);
        Integral::from_unsigned(clamped, width)
      },
      (IEEE64, Signedness::Unsigned) => {
        let f = f64::from_bits(self.bits.to());
        let clamped = clamp_float_to_unsigned(f, width);
        Integral::from_unsigned(clamped, width)
      },
    }
  }
}

/// Clamp a float to the range of a signed integer with given width.
fn clamp_float_to_signed<F: ToI128 + BuiltinFloat>(f: F, width: u8) -> i128 {
  f.to_i128()
    .clamp(-(1i128 << (width - 1)), (1i128 << (width - 1)) - 1)
}

/// Clamp a float to the range of an unsigned integer with given width.
fn clamp_float_to_unsigned<F: ToU128 + BuiltinFloat>(f: F, width: u8) -> u128 {
  f.to_u128().clamp(0, (1u128 << width) - 1)
}
