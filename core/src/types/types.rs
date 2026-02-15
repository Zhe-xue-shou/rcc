use super::{
  Array, ArraySize, Constant, Enum, FunctionProto, Pointer, Primitive,
  QualifiedType, Record, TypeInfo, Union,
};
use crate::common::{FloatFormat, Floating, Integral, Signedness};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
  Primitive(Primitive),
  Pointer(Pointer),
  Array(Array),
  FunctionProto(FunctionProto),
  Enum(Enum),
  Record(Record),
  Union(Union),
}

#[repr(transparent)]
pub struct TypeRef<'tcx> {
  inner: &'tcx Type,
}

impl Type {
  pub fn is_modifiable(&self) -> bool {
    if self.size() == 0 {
      false
    } else {
      match self {
        Type::Array(_) => false,
        _ => true, // todo: struct/union with const member
      }
    }
  }

  pub fn is_void(&self) -> bool {
    matches!(self, Type::Primitive(Primitive::Void))
  }

  pub fn is_integer(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_integer(),
      _ => false,
    }
  }

  pub fn is_floating_point(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_floating_point(),
      _ => false,
    }
  }

  pub fn is_arithmetic(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_arithmetic(),
      _ => false,
    }
  }
}
impl Type {
  pub const fn void() -> Self {
    Type::Primitive(Primitive::Void)
  }

  pub const fn bool_type() -> Self {
    // Type::Primitive(Primitive::Bool)
    Self::int()
  }

  pub const fn int() -> Self {
    Type::Primitive(Primitive::Int)
  }

  pub const fn float() -> Self {
    Type::Primitive(Primitive::Float)
  }

  pub const fn double() -> Self {
    Type::Primitive(Primitive::Double)
  }

  pub const fn nullptr() -> Self {
    Type::Primitive(Primitive::Nullptr)
  }

  pub const fn uintptr() -> Self {
    Type::Primitive(Primitive::ULongLong)
  }

  pub const fn ptrdiff() -> Self {
    Type::Primitive(Primitive::LongLong)
  }

  pub const fn char() -> Self {
    Type::Primitive(Primitive::Char)
  }

  pub fn char_array(length: usize) -> Self {
    Type::Array(Array {
      element_type: QualifiedType::new_unqualified(
        Type::Primitive(Primitive::Char).into(),
      ),
      size: ArraySize::Constant(length),
    })
  }
}

impl Integral {
  pub fn unqualified_type(&self) -> Type {
    if self.signedness() == Signedness::Signed {
      match self.width() {
        Self::WIDTH_CHAR => Type::Primitive(Primitive::SChar),
        Self::WIDTH_SHORT => Type::Primitive(Primitive::Short),
        Self::WIDTH_INT => Type::Primitive(Primitive::Int),
        // Self::WIDTH_LONG => Type::Primitive(Primitive::Long),
        Self::WIDTH_LONG_LONG => Type::Primitive(Primitive::LongLong),
        _ => Type::Primitive(Primitive::Int), // default
      }
    } else {
      match self.width() {
        Self::WIDTH_CHAR => Type::Primitive(Primitive::UChar),
        Self::WIDTH_SHORT => Type::Primitive(Primitive::UShort),
        Self::WIDTH_INT => Type::Primitive(Primitive::UInt),
        // Self::WIDTH_LONG => Type::Primitive(Primitive::ULong),
        Self::WIDTH_LONG_LONG => Type::Primitive(Primitive::ULongLong),
        _ => Type::Primitive(Primitive::UInt), // default
      }
    }
  }
}

impl Floating {
  pub const fn unqualified_type(&self) -> Type {
    use FloatFormat::*;
    match self.format() {
      IEEE32 => Type::float(),
      IEEE64 => Type::double(),
    }
  }
}

impl Constant {
  pub fn unqualified_type(&self) -> Type {
    match self {
      Self::Integral(integral) => integral.unqualified_type(),
      Self::Floating(floating) => floating.unqualified_type(),
      Self::String(str) => Type::char_array(str.len() + 1),
      Self::Nullptr(_) => Type::nullptr(),
      Self::Address(_) => Pointer::new(QualifiedType::void()).into(),
    }
  }
}
