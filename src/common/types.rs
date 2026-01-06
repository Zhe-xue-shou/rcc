use ::bitflags::bitflags;
use ::strum_macros::{Display, IntoStaticStr};
use std::fmt::Debug;
use std::str::FromStr;
use strum_macros::EnumString;

// C's built-in types, I only consider x86_64 here for simplicity
#[derive(Debug, Display, IntoStaticStr, EnumString, Clone, PartialEq)]
pub enum Primitive {
  // assume 64bit
  #[strum(serialize = "char")]
  #[strum(serialize = "signed char")]
  Int8, // signed char/plain char
  #[strum(serialize = "short")]
  Int16, // short
  #[strum(serialize = "long")]
  #[strum(serialize = "int")]
  Int32, // int/long
  #[strum(serialize = "long long")]
  Int64, // long long
  #[strum(serialize = "unsigned char")]
  Uint8, // unsigned char
  #[strum(serialize = "unsigned short")]
  Uint16, // unsigned short
  #[strum(serialize = "unsigned long")]
  #[strum(serialize = "unsigned int")]
  Uint32, // unsigned int/unsigned long
  #[strum(serialize = "unsigned long long")]
  Uint64, // unsigned long long
  #[strum(serialize = "float")]
  Float32, // float
  #[strum(serialize = "long double")]
  #[strum(serialize = "double")]
  Float64,
  #[strum(serialize = "void")]
  Void,
  // ignore below for now
  #[strum(serialize = "_Complex")]
  #[strum(serialize = "_Complex float")]
  ComplexFloat32,
  #[strum(serialize = "_Complex double")]
  #[strum(serialize = "_Complex long double")]
  ComplexFloat64,
  #[strum(serialize = "bool")]
  #[strum(serialize = "_Bool")]
  Bool,
  // wchar_t is a built-in in C++, but not C.
}

bitflags! {
/// type-specifier-qualifier:
/// -    type-specifier
/// -    type-qualifier
/// -    alignment-specifier (don't care)
///
/// specifier would be merged into `Type` directly, so here only have qualifiers
  #[derive(Debug, Copy, Clone, PartialEq)]
  pub struct Qualifiers: u8 {
    const Const = 0x01;
    const Volatile = 0x02;
    const Restrict = 0x04;
    const Atomic = 0x08; // ignore for now
  }
}
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedType {
  pub qualifiers: Qualifiers,
  pub unqualified_type: Type,
}
impl QualifiedType {
  pub fn new(qualifiers: Qualifiers, unqualified_type: Type) -> Self {
    Self {
      qualifiers,
      unqualified_type,
    }
  }
}
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, Display, PartialEq)]
pub enum ArraySize {
  Constant(usize),
  Incomplete,
  // Variable, // ignore for now
}
#[derive(Debug, Clone, PartialEq)]
pub struct Array {
  pub element_type: Box<QualifiedType>,
  pub size: ArraySize,
}
/// function types themselves don't have qualifiers, but pointers to them can.
/// so the functionproto's qualifiers must be dropped.
///
/// ```c
/// int func(int a, float b);
/// int (*pfunc)(int, float) = &func;
/// int (*const cpfunc)(int, float) = &func;
///
/// const int* cptr_func(int a, float b);
/// const int* (*pfunc2)(int, float) = &cptr_func;
/// const int* (*const cpfunc2)(int, float) = &cptr_func;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionProto {
  pub return_type: Box<QualifiedType>,
  pub parameter_types: Vec<QualifiedType>,
  pub is_variadic: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
  pub name: String,
  pub field_type: QualifiedType,
}
// ignore unnamed/anonymous structs/unions for now
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
  pub name: Option<String>,
  pub fields: Vec<Field>,
}

// seems not so much difference between struct and union here
type Union = Record;
#[derive(Debug, Clone, PartialEq)]
pub struct EnumConstant {
  pub name: String,
  pub value: Option<isize>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
  pub name: Option<String>,
  pub constants: Vec<EnumConstant>,
}

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
  pub fn size(&self) -> usize {
    match self {
      Primitive::Int8 | Primitive::Uint8 => 1,
      Primitive::Int16 | Primitive::Uint16 => 2,
      Primitive::Int32 | Primitive::Uint32 | Primitive::Float32 => 4,
      Primitive::Int64 | Primitive::Uint64 | Primitive::Float64 => 8,
      Primitive::Void => 0,
      _ => unimplemented!(),
    }
  }
}
impl FunctionProto {
  pub fn new(
    return_type: QualifiedType,
    parameter_types: Vec<QualifiedType>,
    is_variadic: bool,
  ) -> Self {
    Self {
      return_type: Box::new(return_type),
      parameter_types,
      is_variadic,
    }
  }
}

impl Type {
  pub fn function_proto(&self) -> &FunctionProto {
    match self {
      Type::FunctionProto(function_proto) => function_proto,
      _ => panic!("Type is not FunctionProto"),
    }
  }
}
mod fmt {
  use super::{Array, ArraySize, FunctionProto, QualifiedType, Qualifiers, Type};
  use ::std::fmt::Display;

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
        Type::FunctionProto(proto) => write!(f, "{}", proto),
        _ => todo!(),
      }
    }
  }
}
