use ::rc_utils::static_assert;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ::std::marker::ConstParamTy)]
pub enum Format {
  /// standard IEEE float.
  IEEE32 = 32,
  /// standard IEEE double.
  IEEE64 = 64,
}
// IEEE128,      // 'long double' Quad precision

use Format::*;

type Underlying = u128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Floating {
  bits: Underlying,
  format: Format,
}

static_assert!(
  ::std::mem::needs_drop::<Floating>() == false,
  "Floating should be a POD type without drop"
);

impl ::std::fmt::Display for Floating {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    match self.format {
      IEEE32 => write!(f, "{}", f32::from_bits(self.bits as u32)),
      IEEE64 => write!(f, "{}", f64::from_bits(self.bits as u64)),
    }
  }
}
impl Floating {
  pub fn new<T: ::rc_utils::ToU128>(bits: T, format: Format) -> Self {
    Self {
      bits: bits.to_u128(),
      format,
    }
  }

  pub const fn format(&self) -> Format {
    self.format
  }
}

impl Floating {
  pub fn is_zero(&self) -> bool {
    match self.format {
      // pos zero: 0x00000000, neg zero: 0x80000000
      IEEE32 => (self.bits as u32 & 0x7FFF_FFFF) == 0,
      IEEE64 => (self.bits as u64 & 0x7FFF_FFFF_FFFF_FFFF) == 0,
    }
  }

  pub fn is_infinite(&self) -> bool {
    match self.format {
      // Exponent all 1s, Fraction all 0s
      IEEE32 => (self.bits as u32 & 0x7FFF_FFFF) == 0x7F80_0000,
      IEEE64 =>
        (self.bits as u64 & 0x7FFF_FFFF_FFFF_FFFF) == 0x7FF0_0000_0000_0000,
    }
  }

  pub fn is_nan(&self) -> bool {
    match self.format {
      // Exponent all 1s, Fraction non-zero
      IEEE32 => (self.bits as u32 & 0x7FFF_FFFF) > 0x7F80_0000,
      IEEE64 =>
        (self.bits as u64 & 0x7FFF_FFFF_FFFF_FFFF) > 0x7FF0_0000_0000_0000,
    }
  }

  pub fn is_finite(&self) -> bool {
    match self.format {
      // Exponent is NOT all 1s
      IEEE32 => (self.bits as u32 & 0x7F80_0000) != 0x7F80_0000,
      IEEE64 =>
        (self.bits as u64 & 0x7FF0_0000_0000_0000) != 0x7FF0_0000_0000_0000,
    }
  }
}

impl From<f32> for Floating {
  fn from(val: f32) -> Self {
    Floating::new(val.to_bits() as Underlying, IEEE32)
  }
}

impl From<f64> for Floating {
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

use ::std::ops::{Add, Div, Mul, Neg, Not, Sub};

macro_rules! impl_op {
  ($trait:ident, $method:ident, $op:tt) => {
    impl $trait for Floating {
      type Output = Self;

      fn $method(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(
          self.format, rhs.format,
          "Cannot perform operation on Floating values of different formats"
        );
        match self.format {
          IEEE32 => {
            let lhs_f = f32::from_bits(self.bits as u32);
            let rhs_f = f32::from_bits(rhs.bits as u32);
            Floating::from(lhs_f $op rhs_f)
          },
          IEEE64 => {
            let lhs_f = f64::from_bits(self.bits as u64);
            let rhs_f = f64::from_bits(rhs.bits as u64);
            Floating::from(lhs_f $op rhs_f)
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
        let lhs_f = f32::from_bits(self.bits as u32);
        Floating::from(-lhs_f)
      },
      IEEE64 => {
        let lhs_f = f64::from_bits(self.bits as u64);
        Floating::from(-lhs_f)
      },
    }
  }
}

impl Not for Floating {
  type Output = bool;

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
        &f32::from_bits(self.bits as u32),
        &f32::from_bits(other.bits as u32),
      ),
      IEEE64 => f64::partial_cmp(
        &f64::from_bits(self.bits as u64),
        &f64::from_bits(other.bits as u64),
      ),
    }
  }
}

#[cfg(test)]
mod tests {

  use crate::common::Floating;

  #[test]
  fn test_f() {
    let f = Floating::from(f32::INFINITY);
    assert!(f.is_infinite());
  }
}
