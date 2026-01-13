use super::{
  Array, ArraySize, Enum, FunctionProto, Pointer, Primitive, QualifiedType, Record, Type, TypeInfo,
  Union,
};

impl TypeInfo for QualifiedType {
  #[inline]
  fn size(&self) -> usize {
    self.unqualified_type.size()
  }
  #[inline]
  fn is_scalar(&self) -> bool {
    self.unqualified_type.is_scalar()
  }
}
macro_rules! dispatch_down {
  ($name:ident, $ty:ty) => {
    fn $name(&self) -> $ty {
      match self {
        Type::Primitive(p) => p.$name(),
        Type::Pointer(p) => p.$name(),
        Type::Enum(e) => e.$name(),
        Type::Record(r) => r.$name(),
        Type::Union(u) => u.$name(),
        Type::Array(a) => a.$name(),
        Type::FunctionProto(f) => f.$name(),
      }
    }
  };
}
impl TypeInfo for Type {
  dispatch_down!(size, usize);
  dispatch_down!(is_scalar, bool);
}
impl TypeInfo for Primitive {
  fn size(&self) -> usize {
    // x86_64 sizes
    match self {
      Primitive::Nullptr => Primitive::ULongLong.size(),
      Primitive::Void => 0,
      Primitive::Bool => 1,
      Primitive::Char => 1,
      Primitive::SChar => 1,
      Primitive::Short => 2,
      Primitive::Int => 4,
      Primitive::Long => 4, // LLP64 Windows
      Primitive::LongLong => 8,
      Primitive::UChar => 1,
      Primitive::UShort => 2,
      Primitive::UInt => 4,
      Primitive::ULong => 4,
      Primitive::ULongLong => 8,
      Primitive::Float => 4,
      Primitive::Double => 8,
      Primitive::LongDouble => 8,
      Primitive::ComplexFloat => 8,
      Primitive::ComplexDouble => 16,
      Primitive::ComplexLongDouble => 16,
    }
  }

  fn is_scalar(&self) -> bool {
    match self {
      Primitive::Void => false,
      _ => true,
    }
  }
}

impl TypeInfo for Array {
  fn size(&self) -> usize {
    match &self.size {
      ArraySize::Constant(sz) => sz * self.element_type.unqualified_type.size(),
      ArraySize::Incomplete => 0,
    }
  }

  fn is_scalar(&self) -> bool {
    false
  }
}

impl TypeInfo for Record {
  fn size(&self) -> usize {
    self
      .fields
      .iter()
      .map(|f| f.field_type.unqualified_type.size())
      .sum() // rough, padding and alignment not considered -- incomplete type has no members anyway so this handles it too
  }
  #[inline]
  fn is_scalar(&self) -> bool {
    false
  }
}

impl TypeInfo for Union {
  fn size(&self) -> usize {
    self
      .fields
      .iter()
      .map(|f| f.field_type.unqualified_type.size())
      .max()
      .unwrap_or(0) // ditto
  }
  #[inline]
  fn is_scalar(&self) -> bool {
    false
  }
}
impl TypeInfo for Pointer {
  #[inline]
  fn size(&self) -> usize {
    Primitive::ULongLong.size() // x86_64 LLP64 Windows
  }
  #[inline]
  fn is_scalar(&self) -> bool {
    true
  }
}

impl TypeInfo for FunctionProto {
  #[inline]
  fn size(&self) -> usize {
    0 // function types have no size
  }
  #[inline]
  fn is_scalar(&self) -> bool {
    false
  }
}
impl TypeInfo for Enum {
  #[inline]
  fn size(&self) -> usize {
    self.underlying_type.size()
  }
  #[inline]
  fn is_scalar(&self) -> bool {
    true
  }
}
