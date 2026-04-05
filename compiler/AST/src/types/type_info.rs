use ::rcc_adt::{Floating, Integral};

use super::{
  Array, ArraySize, Enum, FunctionProto, Pointer,
  Primitive::{self, *},
  QualifiedType, Record, Type, Union,
};
use crate::Constant;

pub const trait TypeInfo<'c> {
  #[must_use]
  fn size(&self) -> usize;
  #[must_use]
  fn size_bits(&self) -> usize;
  #[must_use]
  fn default_value(&self) -> Constant<'c>;
  #[must_use]
  fn extent(&self) -> usize;
  #[inline(always)]
  #[must_use]
  fn is_complete(&self) -> bool {
    self.size() != 0
  }
  #[inline(always)]
  #[must_use]
  fn is_scalar(&self) -> bool {
    self.extent() == 1
  }
}

impl<'c> TypeInfo<'c> for QualifiedType<'c> {
  #[inline(always)]
  fn size(&self) -> usize {
    self.unqualified_type.size()
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.unqualified_type.size_bits()
  }

  #[inline(always)]
  fn default_value(&self) -> Constant<'c> {
    self.unqualified_type.default_value()
  }

  #[inline(always)]
  fn extent(&self) -> usize {
    self.unqualified_type.extent()
  }
}
impl<'c> TypeInfo<'c> for Type<'c> {
  #[inline]
  fn size(&self) -> usize {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.size() =>
      Primitive Array Pointer FunctionProto Enum Record Union
    )
  }

  #[inline]
  fn size_bits(&self) -> usize {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.size_bits() =>
      Primitive Array Pointer FunctionProto Enum Record Union
    )
  }

  #[inline]
  fn default_value(&self) -> Constant<'c> {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.default_value() =>
      Primitive Array Pointer FunctionProto Enum Record Union
    )
  }

  #[inline]
  fn extent(&self) -> usize {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.extent() =>
      Primitive Array Pointer FunctionProto Enum Record Union
    )
  }
}
impl<'c> const TypeInfo<'c> for Primitive {
  /// integral size should be aligned with method `Primitive::integer_width()`.
  fn size(&self) -> usize {
    // x86_64 sizes
    match self {
      Nullptr => ULongLong.size(),
      Void => 0,
      Bool => 1,
      Char => 1,
      SChar => 1,
      Short => 2,
      Int => 4,
      Long => 8, // x86_64 linux
      LongLong => 8,
      UChar => 1,
      UShort => 2,
      UInt => 4,
      ULong => 8,
      ULongLong => 8,
      Float => 4,
      Double => 8,
      LongDouble => 8,
      ComplexFloat => 8,
      ComplexDouble => 16,
      ComplexLongDouble => 16,
      __IRBit => panic!("invalid call"),
    }
  }

  #[inline]
  fn size_bits(&self) -> usize {
    match self {
      __IRBit => 1,
      _ => self.size() * 8,
    }
  }

  #[inline]
  fn default_value(&self) -> Constant<'c> {
    match self {
      Nullptr => Constant::Nullptr(),
      Void => panic!("void type has no value"),
      _ if self.is_integer() => Constant::Integral(Integral::new(
        0,
        self.size_bits() as u8,
        self.is_signed().into(),
      )),
      _ if self.is_floating_point() =>
        Constant::Floating(Floating::zero(self.floating_format())),
      _ => unreachable!(),
    }
  }

  #[inline(always)]
  fn extent(&self) -> usize {
    use Primitive::*;
    match self {
      Void => 0,
      _ => 1,
    }
  }
}

impl<'c> TypeInfo<'c> for Array<'c> {
  fn size(&self) -> usize {
    match &self.size {
      ArraySize::Constant(sz) => sz * self.element_type.unqualified_type.size(),
      ArraySize::Incomplete => 0,
      ArraySize::Variable(_id) => todo!("VLA"), // ignore for now
    }
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline]
  fn default_value(&self) -> Constant<'c> {
    panic!("default value for non-scalar type should not be requested");
  }

  // dont inline this...
  // #[inline(never)]
  fn extent(&self) -> usize {
    self.element_type.extent() + 1
  }
}

impl<'c> TypeInfo<'c> for Record<'c> {
  fn size(&self) -> usize {
    self
      .fields
      .iter()
      .map(|f| f.field_type.unqualified_type.size())
      .sum() // rough, padding and alignment not considered -- incomplete type has no members anyway so this handles it too
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn default_value(&self) -> Constant<'c> {
    panic!("default value for non-scalar type should not be requested");
  }

  fn extent(&self) -> usize {
    1
  }
}

impl<'c> TypeInfo<'c> for Union<'c> {
  fn size(&self) -> usize {
    self
      .fields
      .iter()
      .map(|f| f.field_type.unqualified_type.size())
      .max()
      .unwrap_or(0) // ditto
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn default_value(&self) -> Constant<'c> {
    panic!("default value for non-scalar type should not be requested");
  }

  #[inline(always)]
  fn extent(&self) -> usize {
    1
  }
}
impl<'c> const TypeInfo<'c> for Pointer<'c> {
  #[inline(always)]
  fn size(&self) -> usize {
    ULongLong.size() // x86_64 LLP64 Windows
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn default_value(&self) -> Constant<'c> {
    Constant::Nullptr()
  }

  #[inline(always)]
  fn extent(&self) -> usize {
    1
  }
}

impl<'c> TypeInfo<'c> for FunctionProto<'c> {
  #[inline(always)]
  fn size(&self) -> usize {
    0 // function types have no size
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn default_value(&self) -> Constant<'c> {
    panic!("default value for non-scalar type should not be requested");
  }

  /// not meaningful.
  #[inline(always)]
  fn extent(&self) -> usize {
    0
  }
}
impl<'c> TypeInfo<'c> for Enum<'c> {
  #[inline(always)]
  fn size(&self) -> usize {
    self.underlying_type.size()
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn default_value(&self) -> Constant<'c> {
    self.underlying_type.default_value()
  }

  #[inline(always)]
  fn extent(&self) -> usize {
    1
  }
}
