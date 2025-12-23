use ::bitflags::bitflags;
use ::strum_macros::Display;
use std::fmt::Debug;
use std::str::FromStr;
use strum_macros::EnumString;

// C's built-in types, I only consider x86_64 here for simplicity
#[derive(Debug, Display, EnumString)]
pub enum Primitive {
  // assume 64bit
  #[strum(serialize = "char")]
  #[strum(serialize = "signed char")]
  Int8, // signed char/plain char
  #[strum(serialize = "short")]
  Int16, // short
  #[strum(serialize = "int")]
  #[strum(serialize = "long")]
  Int32, // int/long
  #[strum(serialize = "long long")]
  Int64, // long long
  #[strum(serialize = "unsigned char")]
  Uint8, // unsigned char
  #[strum(serialize = "unsigned short")]
  Uint16, // unsigned short
  #[strum(serialize = "unsigned int")]
  #[strum(serialize = "unsigned long")]
  Uint32, // unsigned int/unsigned long
  #[strum(serialize = "unsigned long long")]
  Uint64, // unsigned long long
  #[strum(serialize = "float")]
  Float32, // float
  #[strum(serialize = "double")]
  #[strum(serialize = "long double")]
  Float64,
  #[strum(serialize = "void")]
  Void,
  // ignore below for now
  #[strum(serialize = "_Complex")]
  #[strum(serialize = "_Complex float")]
  ComplexFloat32,
  #[strum(serialize = "_Complex double")]
  ComplexFloat64,
  #[strum(serialize = "_Complex long double")]
  ComplexFloat128,
  #[strum(serialize = "bool")]
  #[strum(serialize = "_Bool")]
  Bool, // _Bool, or just bool
        // others: wchar_t, etc.
}
bitflags! {
/// type-specifier-qualifier:
/// -    type-specifier
/// -    type-qualifier
/// -    alignment-specifier (don't care)
///
/// specifier would be merged into `Type` directly, so here only have qualifiers
  #[derive(Copy, Clone, PartialEq)]
  pub struct Qualifiers: u8 {
    const Const = 0x01;
    const Volatile = 0x02;
    const Restrict = 0x04;
    const Atomic = 0x08; // ignore for now
  }
}
pub struct QualifiedType {
  pub qualifiers: Qualifiers,
  pub unqualified_type: Type,
}
pub enum Type {
  Primitive(Primitive),
  Array(Array),
  Pointer(Box<QualifiedType>),
  FunctionProto(FunctionProto),
  Enum(Enum),
  Record(Record),
  Union(Union),
  Typedef(String),
}
#[derive(Debug, Clone, Display)]
pub enum ArraySize {
  Constant(usize),
  Incomplete,
  // Variable, // ignore for now
}
pub struct Array {
  pub element_type: Box<QualifiedType>,
  pub size: ArraySize,
}

pub struct FunctionProto {
  pub return_type: Box<QualifiedType>,
  pub parameter_types: Vec<QualifiedType>,
  pub is_variadic: bool,
}
pub struct Field {
  pub name: String,
  pub field_type: QualifiedType,
}
// ignore unnamed/anonymous structs/unions for now
pub struct Record {
  pub name: Option<String>,
  pub fields: Vec<Field>,
}

// seems not so much difference between struct and union here
type Union = Record;

pub struct EnumConstant {
  pub name: String,
  pub value: Option<isize>,
}

pub struct Enum {
  pub name: Option<String>,
  pub constants: Vec<EnumConstant>,
}

impl Type {}

impl Primitive {
  pub fn new(str: String) -> Self {
    Self::maybe_new(str).unwrap()
  }
  pub fn maybe_new(str: String) -> Option<Self> {
    Primitive::from_str(&str).ok()
  }
  pub fn to_type(self) -> Type {
    Type::Primitive(self)
  }
}

mod fmt {
  use super::{Array, ArraySize, FunctionProto, QualifiedType, Qualifiers, Type};
  use ::std::fmt::{Debug, Display};

  impl Display for Qualifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let mut qualifiers = Vec::new();
      if self.contains(Qualifiers::Const) {
        qualifiers.push("const");
      }
      if self.contains(Qualifiers::Volatile) {
        qualifiers.push("volatile");
      }
      if self.contains(Qualifiers::Restrict) {
        qualifiers.push("restrict");
      }
      write!(f, "{}", qualifiers.join(" "))
    }
  }

  impl Display for QualifiedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      if self.qualifiers.is_empty() {
        write!(f, "{}", self.unqualified_type)
      } else {
        write!(f, "{} {}", self.qualifiers, self.unqualified_type)
      }
    }
  }
  impl Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}[", self.element_type)?;
      match &self.size {
        ArraySize::Constant(sz) => write!(f, "{}", sz)?,
        ArraySize::Incomplete => write!(f, "")?,
      }
      write!(f, "]")
    }
  }

  impl Display for FunctionProto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}(", self.return_type)?;
      for (i, param) in self.parameter_types.iter().enumerate() {
        if i > 0 {
          write!(f, ", ")?;
        }
        write!(f, "{}", param)?;
      }
      write!(f, ")")
    }
  }

  impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Type::Primitive(builtin) => write!(f, "{}", builtin),
        _ => todo!(),
      }
    }
  }

  impl Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
}
