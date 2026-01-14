use ::once_cell::sync::Lazy;
use ::std::str::FromStr;
use ::strum_macros::{Display, EnumString, IntoStaticStr};
use lilac_utils::{interconvert, make_trio_for};

use super::{Compatibility, TypeInfo};
use crate::common::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
  Primitive(Primitive),
  Array(Array),
  Pointer(Pointer),
  FunctionProto(FunctionProto),
  Enum(Enum),
  Record(Record),
  Union(Union),
}

::bitflags::bitflags! {
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
::bitflags::bitflags! {
  #[derive(Debug,Clone,Copy,PartialEq,Eq)]
  pub struct FunctionSpecifier : u8 {
    const Inline = 0x01;
    const Noreturn = 0x10;
  }
}

// C's built-in types
#[derive(Debug, Display, IntoStaticStr, EnumString, Clone, PartialEq)]
pub enum Primitive {
  #[strum(serialize = "bool")]
  #[strum(serialize = "_Bool")]
  Bool,
  #[strum(serialize = "char")]
  Char, // plain char
  #[strum(serialize = "signed char")]
  SChar, // signed char
  #[strum(serialize = "short")]
  Short,
  #[strum(serialize = "int")]
  Int,
  #[strum(serialize = "long")]
  Long,
  #[strum(serialize = "long long")]
  LongLong,
  #[strum(serialize = "unsigned char")]
  UChar,
  #[strum(serialize = "unsigned short")]
  UShort,
  #[strum(serialize = "unsigned int")]
  UInt,
  #[strum(serialize = "unsigned long")]
  ULong,
  #[strum(serialize = "unsigned long long")]
  ULongLong,
  #[strum(serialize = "float")]
  Float,
  #[strum(serialize = "double")]
  Double,
  #[strum(serialize = "long double")]
  LongDouble,
  #[strum(serialize = "void")]
  Void,
  #[strum(serialize = "nullptr_t")]
  Nullptr,
  // ignore below for now: __STDC_NO_COMPLEX__
  #[strum(serialize = "_Complex float")]
  ComplexFloat,
  #[strum(serialize = "_Complex")]
  #[strum(serialize = "_Complex double")]
  ComplexDouble,
  #[strum(serialize = "_Complex long double")]
  ComplexLongDouble,
  // wchar_t is a built-in in C++, but not C, in C it's `typedef`-ed as unsigned short on Windows
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedType {
  qualifiers: Qualifiers,
  unqualified_type: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pointer {
  pub pointee: Box<QualifiedType>,
}

#[derive(Debug, Clone, Display, PartialEq)]
pub enum ArraySize {
  Constant(usize),
  Incomplete,
  // Variable, // ignore for now
}

#[derive(Debug, Clone, PartialEq)]
pub struct Array {
  /// Array itself cannot have qualifiers, hence the QualifiedType::qualifiers of the whole array should be empty, the actual element type's qualifiers are stored here.
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

// seems not so much difference between struct and union here, but for convenience we keep them separate
#[derive(Debug, Clone, PartialEq)]
pub struct Union {
  pub name: Option<String>,
  pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumConstant {
  pub name: String,
  pub value: Option<isize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
  pub name: Option<String>,
  pub constants: Vec<EnumConstant>,
  pub underlying_type: Primitive, // must be integer type
}

impl QualifiedType {
  pub fn new(qualifiers: Qualifiers, unqualified_type: Type) -> Self {
    Self {
      qualifiers,
      unqualified_type,
    }
  }

  pub fn new_unqualified(unqualified_type: Type) -> Self {
    Self {
      qualifiers: Qualifiers::empty(),
      unqualified_type,
    }
  }

  pub fn void() -> Self {
    Self::new_unqualified(Type::void())
  }

  pub fn bool() -> Self {
    Self::new_unqualified(Type::bool())
  }

  pub fn int() -> Self {
    Self::new_unqualified(Type::int())
  }
}
impl Pointer {
  pub fn new(pointee: Box<QualifiedType>) -> Self {
    Self { pointee }
  }
}

impl Primitive {
  pub fn new(str: String) -> Self {
    Self::maybe_new(str).unwrap()
  }

  pub fn maybe_new(str: String) -> Option<Self> {
    Primitive::from_str(&str).ok()
  }
}

impl FunctionProto {
  const MAIN_PROTO_ARGS: Lazy<FunctionProto> = Lazy::new(|| {
    FunctionProto::new(
      Box::new(QualifiedType::new(
        Qualifiers::empty(),
        Type::Primitive(Primitive::Int),
      )),
      vec![QualifiedType::new(
        Qualifiers::empty(),
        Type::Pointer(Pointer::new(Box::new(QualifiedType::new(
          Qualifiers::empty(),
          Type::Primitive(Primitive::Char),
        )))),
      )],
      false,
    )
  });
  const MAIN_PROTO_EMPTY: Lazy<FunctionProto> = Lazy::new(|| {
    FunctionProto::new(
      Box::new(QualifiedType::new(
        Qualifiers::empty(),
        Type::Primitive(Primitive::Int),
      )),
      vec![],
      false,
    )
  });

  pub fn new(
    return_type: Box<QualifiedType>,
    parameter_types: Vec<QualifiedType>,
    is_variadic: bool,
  ) -> Self {
    Self {
      return_type,
      parameter_types,
      is_variadic,
    }
  }

  pub fn main_proto_validate(
    &self,
    function_specifier: FunctionSpecifier,
  ) -> Result<(), Error> {
    if self.is_variadic {
      Err(())
    } else if function_specifier.contains(FunctionSpecifier::Inline) {
      Err(())
    } else if !self.compatible_with(&Self::MAIN_PROTO_EMPTY)
      && !self.compatible_with(&Self::MAIN_PROTO_ARGS)
    {
      Err(())
    } else {
      todo!()
    }
  }
}

impl Array {
  pub fn new(element_type: Box<QualifiedType>, size: ArraySize) -> Self {
    Self { element_type, size }
  }
}
impl Enum {
  pub fn new(
    name: Option<String>,
    constants: Vec<EnumConstant>,
    underlying_type: Primitive,
  ) -> Self {
    assert!(underlying_type.is_integer());
    Self {
      name,
      constants,
      underlying_type,
    }
  }

  pub fn into_underlying_type(self) -> Primitive {
    self.underlying_type
  }
}

interconvert!(Primitive, Type);
interconvert!(Array, Type);
interconvert!(Pointer, Type);
interconvert!(FunctionProto, Type);
interconvert!(Enum, Type);
interconvert!(Record, Type);
interconvert!(Union, Type);

make_trio_for!(Primitive, Type);
make_trio_for!(Array, Type);
make_trio_for!(Pointer, Type);
make_trio_for!(FunctionProto, Type);
make_trio_for!(Enum, Type);
make_trio_for!(Record, Type);
make_trio_for!(Union, Type);

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

  pub fn is_arithmetic(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_arithmetic(),
      _ => false,
    }
  }

  pub fn void() -> Self {
    Type::Primitive(Primitive::Void)
  }

  pub fn bool() -> Self {
    Type::Primitive(Primitive::Bool)
  }

  pub fn int() -> Self {
    Type::Primitive(Primitive::Int)
  }
}
impl QualifiedType {
  pub fn is_modifiable(&self) -> bool {
    self.unqualified_type.is_modifiable()
      && !self.qualifiers.contains(Qualifiers::Const)
  }

  pub fn is_void(&self) -> bool {
    self.unqualified_type.is_void()
  }

  pub fn qualifiers(&self) -> &Qualifiers {
    &self.qualifiers
  }

  pub fn unqualified_type(&self) -> &Type {
    &self.unqualified_type
  }

  pub fn destructure(self) -> (Qualifiers, Type) {
    (self.qualifiers, self.unqualified_type)
  }
}
impl From<Type> for QualifiedType {
  fn from(value: Type) -> Self {
    QualifiedType::new_unqualified(value)
  }
}
