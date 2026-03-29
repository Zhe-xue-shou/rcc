use ::rcc_utils::{
  BuiltinFloat, NumTo, const_pre, ensure_is_pod, signed_type_of,
  underlying_type_of,
};

type Underlying = u128;
use ::rcc_utils::ToU128 as ToUnderlying;
type SignedUnderlying = signed_type_of!(u128);
use ::rcc_utils::ToI128 as ToSignedUnderlying;
const __MAX_ZU: u8 = 128;

#[derive(Debug, Copy, Hash, ::std::marker::ConstParamTy)]
#[derive_const(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Format {
  /// standard IEEE float.
  IEEE32 = 32,
  /// standard IEEE double.
  IEEE64 = 64,
}
// IEEE128,      // 'long double' Quad precision

use Format::*;
impl Format {
  pub const fn size_bits(&self) -> usize {
    match self {
      IEEE32 => 32,
      IEEE64 => 64,
    }
  }
}

// union Bits {
//   ieee32: f32,
//   ieee64: f64,
// }

/// you can see the doc of [`Integral`] for more information. Unlike the feature-rich [`Integral`],
/// this struct is essentially a simple wrapper around the raw bits of a floating-point number, along with its format.
/// That being said, I didn't reference
/// [LLVM's APFloat](https://github.com/llvm/llvm-project/blob/main/llvm/include/llvm/ADT/APFloat.h) at all,
/// which is a far more comprehensive and complex implementation.
#[derive(Debug, Copy, Hash)]
#[derive_const(Clone, PartialEq, Eq)]
pub struct Floating {
  // i dont thinks its need ed to use the u128 jere, just a tagger union would be fine.
  bits: Underlying,
  format: Format,
}

ensure_is_pod!(Floating);

impl Floating {
  pub const IEEE32_ONE: Floating = Floating::from(1.0f32);
  pub const IEEE32_ZERO: Floating = Floating::from(0.0f32);
  pub const IEEE64_ONE: Floating = Floating::from(1.0f64);
  pub const IEEE64_ZERO: Floating = Floating::from(0.0f64);
  pub const MAX_SUPPORTED_SIZE_BITS: u8 = self::__MAX_ZU;
}

impl Floating {
  pub const fn new<T: [const] ToUnderlying>(bits: T, format: Format) -> Self {
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

  pub const fn one(format: Format) -> Self {
    match format {
      IEEE32 => Floating::from(1.0f32),
      IEEE64 => Floating::from(1.0f64),
    }
  }
}

impl Floating {
  pub const fn is_zero(&self) -> bool {
    match self.format {
      // pos zero: 0x00000000, neg zero: 0x80000000
      IEEE32 => (self.bits as underlying_type_of!(f32) & 0x7FFF_FFFF) == 0,
      IEEE64 =>
        (self.bits as underlying_type_of!(f64) & 0x7FFF_FFFF_FFFF_FFFF) == 0,
    }
  }

  pub const fn is_infinite(&self) -> bool {
    match self.format {
      // Exponent all 1s, Fraction all 0s
      IEEE32 =>
        (self.bits as underlying_type_of!(f32) & 0x7FFF_FFFF) == 0x7F80_0000,
      IEEE64 =>
        (self.bits as underlying_type_of!(f64) & 0x7FFF_FFFF_FFFF_FFFF)
          == 0x7FF0_0000_0000_0000,
    }
  }

  pub const fn is_nan(&self) -> bool {
    match self.format {
      // Exponent all 1s, Fraction non-zero
      IEEE32 =>
        (self.bits as underlying_type_of!(f32) & 0x7FFF_FFFF) > 0x7F80_0000,
      IEEE64 =>
        (self.bits as underlying_type_of!(f64) & 0x7FFF_FFFF_FFFF_FFFF)
          > 0x7FF0_0000_0000_0000,
    }
  }

  pub const fn is_finite(&self) -> bool {
    match self.format {
      // Exponent is NOT all 1s
      IEEE32 =>
        (self.bits as underlying_type_of!(f32) & 0x7F80_0000) != 0x7F80_0000,
      IEEE64 =>
        (self.bits as underlying_type_of!(f64) & 0x7FF0_0000_0000_0000)
          != 0x7FF0_0000_0000_0000,
    }
  }

  pub const fn cast(self, format: Format) -> Self {
    match (self.format, format) {
      (IEEE32, IEEE64) => Floating::from(f32::from_bits(
        self.bits as underlying_type_of!(f32),
      ) as f64),
      (IEEE64, IEEE32) => Floating::from(f64::from_bits(
        self.bits as underlying_type_of!(f64),
      ) as f32),
      (IEEE32, IEEE32) | (IEEE64, IEEE64) => self,
    }
  }

  pub const fn abs(self) -> Self {
    match self.format {
      IEEE32 => Floating::from(
        f32::from_bits(self.bits as underlying_type_of!(f32)).abs(),
      ),
      IEEE64 => Floating::from(
        f64::from_bits(self.bits as underlying_type_of!(f64)).abs(),
      ),
    }
  }
}

impl const From<f32> for Floating {
  fn from(val: f32) -> Self {
    Floating::new(val.to_bits() as Underlying, IEEE32)
  }
}

impl const From<f64> for Floating {
  fn from(val: f64) -> Self {
    Floating::new(val.to_bits() as Underlying, IEEE64)
  }
}

impl ::std::default::Default for Floating {
  fn default() -> Self {
    Self {
      bits: f64::default().to_bits() as Underlying,
      format: IEEE64,
    }
  }
}

use super::{Integral, Signedness};

mod ops {
  use ::std::ops::{Add, Div, Mul, Neg, Not, Sub};

  use super::*;
  macro_rules! impl_op {
  ($trait:ident, $method:ident, $op:tt) => {
    impl const $trait for Floating {
      type Output = Self;
      #[inline]
      fn $method(self, rhs: Self) -> Self::Output {
        const_pre + (
          self.format, rhs.format,
          concat!("Cannot perform ", stringify!($op), " on Floating values of different formats")
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

  impl const Neg for Floating {
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

  impl const Not for Floating {
    type Output = bool;

    #[inline(always)]
    fn not(self) -> Self::Output {
      self.is_zero()
    }
  }

  impl const PartialOrd for Floating {
    fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
      const_pre
        + (
          self.format,
          other.format,
          "Cannot compare Floating values of different formats",
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
}

impl Integral {
  #[inline]
  pub const fn to_floating(self, format: Format) -> Floating {
    match format {
      IEEE32 => Floating::from(self.to_builtin::<u32>() as f32),
      IEEE64 => Floating::from(self.to_builtin::<u64>() as f64),
    }
  }
}

impl Floating {
  #[inline]
  pub const fn to_integral(
    self,
    width: u8,
    signedness: Signedness,
  ) -> Integral {
    debug_assert!(width > 0 && width <= Self::MAX_SUPPORTED_SIZE_BITS);
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
const fn clamp_float_to_signed<F: [const] ToSignedUnderlying + BuiltinFloat>(
  f: F,
  width: u8,
) -> SignedUnderlying {
  f.to_i128().clamp(
    -((1 as SignedUnderlying) << (width - 1)),
    ((1 as SignedUnderlying) << (width - 1)) - 1,
  )
}

/// Clamp a float to the range of an unsigned integer with given width.
const fn clamp_float_to_unsigned<F: [const] ToUnderlying + BuiltinFloat>(
  f: F,
  width: u8,
) -> Underlying {
  f.to_u128().clamp(0, ((1 as Underlying) << width) - 1)
}
mod fmt {
  use super::*;
  macro_rules! impl_fmt {
    ($trait:ident, $method:ident) => {
      impl ::std::fmt::$trait for Floating {
        fn $method(
          &self,
          f: &mut ::std::fmt::Formatter<'_>,
        ) -> ::std::fmt::Result {
          match self.format {
            IEEE32 =>
              f32::from_bits(self.bits as underlying_type_of!(f32)).$method(f),
            IEEE64 =>
              f64::from_bits(self.bits as underlying_type_of!(f64)).$method(f),
          }
        }
      }
    };
  }

  macro_rules! impl_all_fmt {
    ($($trait:ident, $method:ident);* $(;)?) => {
      $(
        impl_fmt!($trait, $method);
      )*
    };
  }
  impl_all_fmt! {
    Display, fmt;
    LowerExp, fmt;
    UpperExp, fmt;
  }
}

#[cfg(test)]
#[allow(clippy::unnecessary_cast)]
#[allow(non_upper_case_globals)]
mod tests {
  macro_rules! const_assert_approx_eq {
    ($a:expr, $b:expr, $epsilon:expr) => {
      const _: () = assert!(($a - $b).abs() <= $epsilon);
    };
  }

  use ::rcc_utils::static_assert_eq;
  use ::std::{f32, f64};

  #[allow(unused)]
  use super::*;

  #[test]
  const fn test_floating() {
    const F1: Floating = Floating::from(f32::consts::PI);
    const F2: Floating = Floating::from(f64::consts::PI);
    static_assert_eq!(F1.format(), Format::IEEE32);
    static_assert_eq!(F2.format(), Format::IEEE64);
    const_assert_approx_eq!(F1.cast(Format::IEEE64), F2, 1e-7f64.into());
    const_assert_approx_eq!(F2.cast(Format::IEEE32), F1, f32::EPSILON.into());
  }
}
