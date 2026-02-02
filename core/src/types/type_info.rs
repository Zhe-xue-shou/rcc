use super::{
  Array, ArraySize, Enum, FunctionProto, Pointer, Primitive, Primitive::*,
  QualifiedType, Record, Union,
};
#[allow(unused)]
#[::enum_dispatch::enum_dispatch(Type)]
pub trait TypeInfo {
  fn size(&self) -> usize;
  fn is_scalar(&self) -> bool;
}

impl TypeInfo for QualifiedType {
  #[inline]
  fn size(&self) -> usize {
    self.unqualified_type().size()
  }

  #[inline]
  fn is_scalar(&self) -> bool {
    self.unqualified_type().is_scalar()
  }
}
#[allow(unused)]
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
// // enum_dispatch replaces these.
// impl TypeInfo for Type {
//   dispatch_down!(size, usize);

//   dispatch_down!(is_scalar, bool);
// }
impl TypeInfo for Primitive {
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
      Long => 4, // LLP64 Windows
      LongLong => 8,
      UChar => 1,
      UShort => 2,
      UInt => 4,
      ULong => 4,
      ULongLong => 8,
      Float => 4,
      Double => 8,
      LongDouble => 8,
      ComplexFloat => 8,
      ComplexDouble => 16,
      ComplexLongDouble => 16,
    }
  }

  fn is_scalar(&self) -> bool {
    !matches!(self, Void)
  }
}

impl TypeInfo for Array {
  fn size(&self) -> usize {
    match &self.size {
      ArraySize::Constant(sz) =>
        sz * self.element_type.unqualified_type().size(),
      ArraySize::Incomplete => 0,
      ArraySize::Variable(_id) => todo!(), // ignore for now
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
      .map(|f| f.field_type.unqualified_type().size())
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
      .map(|f| f.field_type.unqualified_type().size())
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
    ULongLong.size() // x86_64 LLP64 Windows
  }

  #[inline]
  fn is_scalar(&self) -> bool {
    ULongLong.is_scalar() // shall always be true
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
    assert!(self.underlying_type.is_scalar(), "never fails");
    true
  }
}
