use ::bitflags::bitflags;
use ::strum_macros::Display;
use std::fmt::Debug;
use std::str::FromStr;
use strum_macros::EnumString;

// C's built-in types
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
  Float64, // double/long double
  #[strum(serialize = "bool")]
  #[strum(serialize = "_Bool")]
  Bool, // _Bool, or just bool
  #[strum(serialize = "void")]
  Void, // void
        // others: wchar_t, complex, etc. ignored for now
}
bitflags! {
  #[derive(Copy, Clone)]
  pub struct Qualifiers: u8 {
    const Const = 0x01;
    const Volatile = 0x02;
    const Restrict = 0x04;
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
  Function(Function),
  // ignore: struct, union, enum
}
#[derive(Debug, Clone, Display)]
pub enum ArraySize {
  Constant(usize),
  Incomplete,
}
pub struct Array {
  pub element_type: Box<QualifiedType>,
  pub size: ArraySize,
}

pub struct Function {
  pub return_type: Box<QualifiedType>,
  pub parameter_types: Vec<QualifiedType>,
}

impl Type {}

impl Primitive {
  pub fn new(str: String) -> Self {
    Self::maybe_new(str).unwrap()
  }
  pub fn maybe_new(str: String) -> Option<Self> {
    Primitive::from_str(&str).ok()
  }
  pub fn as_type(self) -> Type {
    Type::Primitive(self)
  }
}
