use super::{
  Array, ArraySize, Enum, FunctionProto, Pointer, Primitive, Primitive::*,
  QualifiedType, Record, Type, Union,
};
pub trait TypeInfo {
  #[must_use]
  fn size(&self) -> usize;
  #[must_use]
  fn is_scalar(&self) -> bool;
  #[must_use]
  fn size_bits(&self) -> usize;
}

impl<'context> TypeInfo for QualifiedType<'context> {
  #[inline(always)]
  fn size(&self) -> usize {
    self.unqualified_type.size()
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.unqualified_type.size_bits()
  }

  #[inline(always)]
  fn is_scalar(&self) -> bool {
    self.unqualified_type.is_scalar()
  }
}
impl<'context> TypeInfo for Type<'context> {
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
  fn is_scalar(&self) -> bool {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.is_scalar() =>
      Primitive Array Pointer FunctionProto Enum Record Union
    )
  }
}
impl TypeInfo for Primitive {
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
    }
  }

  #[inline(always)]
  fn is_scalar(&self) -> bool {
    !matches!(self, Void)
  }

  #[inline]
  fn size_bits(&self) -> usize {
    match self {
      Bool => 1,
      _ => self.size() * 8,
    }
  }
}

impl<'context> TypeInfo for Array<'context> {
  fn size(&self) -> usize {
    match &self.size {
      ArraySize::Constant(sz) => sz * self.element_type.unqualified_type.size(),
      ArraySize::Incomplete => 0,
      ArraySize::Variable(_id) => todo!(), // ignore for now
    }
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn is_scalar(&self) -> bool {
    false
  }
}

impl<'context> TypeInfo for Record<'context> {
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
  fn is_scalar(&self) -> bool {
    false
  }
}

impl<'context> TypeInfo for Union<'context> {
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
  fn is_scalar(&self) -> bool {
    false
  }
}
impl<'context> TypeInfo for Pointer<'context> {
  #[inline(always)]
  fn size(&self) -> usize {
    ULongLong.size() // x86_64 LLP64 Windows
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn is_scalar(&self) -> bool {
    ULongLong.is_scalar() // shall always be true
  }
}

impl<'context> TypeInfo for FunctionProto<'context> {
  #[inline(always)]
  fn size(&self) -> usize {
    0 // function types have no size
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn is_scalar(&self) -> bool {
    false
  }
}
impl<'context> TypeInfo for Enum<'context> {
  #[inline(always)]
  fn size(&self) -> usize {
    self.underlying_type.size()
  }

  #[inline(always)]
  fn size_bits(&self) -> usize {
    self.size() * 8
  }

  #[inline(always)]
  fn is_scalar(&self) -> bool {
    assert!(self.underlying_type.is_scalar(), "never fails");
    true
  }
}
