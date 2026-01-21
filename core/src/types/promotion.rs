//! applied during unary operations, including (implicit) unary inside binary operations

use rc_utils::breakpoint;

use super::{CastType, Primitive, QualifiedType, Type};
pub trait Promotion {
  #[must_use]
  fn promote(self) -> (Self, CastType)
  where
    Self: Sized;
}

impl Primitive {
  pub fn is_integer(&self) -> bool {
    self.is_signed_integer() || self.is_unsigned()
  }

  pub fn is_signed_integer(&self) -> bool {
    matches!(
      self,
      Primitive::Char
        | Primitive::SChar
        | Primitive::Short
        | Primitive::Int
        | Primitive::Long
        | Primitive::LongLong
    )
  }

  pub fn is_arithmetic(&self) -> bool {
    self.is_integer() || self.is_floating_point()
  }

  pub fn is_signed(&self) -> bool {
    self.is_signed_integer() || self.is_floating_point()
  }

  pub fn is_unsigned(&self) -> bool {
    matches!(
      self,
      Primitive::Bool
        | Primitive::UChar
        | Primitive::UShort
        | Primitive::UInt
        | Primitive::ULong
        | Primitive::ULongLong
    )
  }

  pub fn integer_rank(&self) -> u8 {
    // bitmask has no use here, just a unique value for each rank
    match self {
      Primitive::Bool => 0x01,
      Primitive::Char | Primitive::SChar | Primitive::UChar => 0x02,
      Primitive::Short | Primitive::UShort => 0x04,
      Primitive::Int | Primitive::UInt => 0x08,
      Primitive::Long | Primitive::ULong => 0x10,
      Primitive::LongLong | Primitive::ULongLong => 0x20,
      _ => {
        breakpoint!();
        panic!("Not an integer type");
      },
    }
  }

  pub fn is_floating_point(&self) -> bool {
    matches!(
      self,
      Primitive::Float | Primitive::Double | Primitive::LongDouble
    ) || self.is_complex()
  }

  pub fn floating_rank(&self) -> u8 {
    match self {
      Primitive::Float | Primitive::ComplexFloat => 0x01,
      Primitive::Double | Primitive::ComplexDouble => 0x02,
      Primitive::LongDouble | Primitive::ComplexLongDouble => 0x04,
      _ => {
        breakpoint!();
        panic!("Not a floating point type");
      },
    }
  }

  pub fn is_complex(&self) -> bool {
    matches!(
      self,
      Primitive::ComplexFloat
        | Primitive::ComplexDouble
        | Primitive::ComplexLongDouble
    )
  }

  pub fn is_void(&self) -> bool {
    matches!(self, Primitive::Void)
  }

  pub fn is_nullptr(&self) -> bool {
    matches!(self, Primitive::Nullptr)
  }

  #[must_use]
  pub fn integer_promotion(self) -> (Primitive, CastType) {
    assert!(self.is_integer(), "Type {:?} is not an integer type", self);

    if self.integer_rank() < Primitive::Int.integer_rank() {
      (Primitive::Int, CastType::IntegralCast)
    } else {
      (self, CastType::Noop)
    }
  }

  /// floating promotion is only happened during variadic function calls and the old `functionnoproto`, which i wont implement
  #[must_use]
  pub fn floating_promotion(self) -> (Primitive, CastType) {
    assert!(
      self.is_floating_point(),
      "Type {:?} is not a floating point type",
      self
    );
    if self.floating_rank() < Primitive::Double.floating_rank() {
      (Primitive::Double, CastType::FloatingCast)
    } else {
      (self, CastType::Noop)
    }
  }

  #[must_use]
  pub fn into_unsigned(self) -> Primitive {
    match self {
      Primitive::Bool => Primitive::Bool,
      Primitive::Char => Primitive::UChar,
      Primitive::SChar => Primitive::UChar,
      Primitive::Short => Primitive::UShort,
      Primitive::Int => Primitive::UInt,
      Primitive::Long => Primitive::ULong,
      Primitive::LongLong => Primitive::ULongLong,
      _ => {
        breakpoint!();
        panic!("Type {:?} is not a signed integer type", self);
      },
    }
  }
}
impl Type {
  pub fn is_unsigned(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_unsigned(),
      Type::Pointer(_) => true,
      Type::Enum(e) => e.underlying_type.is_unsigned(),
      _ => false,
    }
  }

  pub fn is_signed(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_signed(),
      Type::Enum(e) => e.underlying_type.is_signed(),
      _ => false,
    }
  }
}
impl Promotion for Primitive {
  fn promote(self) -> (Self, CastType) {
    if self.is_integer() {
      self.integer_promotion()
    } else {
      (self, CastType::Noop)
    }
  }
}
impl Promotion for Type {
  fn promote(self) -> (Self, CastType) {
    match self {
      Self::Primitive(p) => {
        let (promoted, cast_type) = p.promote();
        (Self::Primitive(promoted), cast_type)
      },
      _ => (self, CastType::Noop),
    }
  }
}
impl Promotion for QualifiedType {
  fn promote(self) -> (Self, CastType) {
    let (qualifiers, promoted) = self.destructure();
    let (promoted, cast_type) = promoted.promote();

    (Self::new(qualifiers, promoted), cast_type)
  }
}
