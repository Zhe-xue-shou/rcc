use ::once_cell::sync::Lazy;
use ::rc_utils::{IntoWith, interconvert, make_trio_for};
use ::std::{rc::Rc, str::FromStr};
use ::strum_macros::{Display, EnumString, IntoStaticStr};

use super::{Compatibility, Constant, TypeInfo};
use crate::{
  common::{FloatFormat, Floating, Integral, Signedness},
  diagnosis::{DiagData::*, DiagMeta, Severity},
};

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
  #[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
  pub struct Qualifiers: u8 {
    const Const = 0x01;
    const Volatile = 0x02;
    const Restrict = 0x04;
    const Atomic = 0x08; // ignore for now
  }
}
::bitflags::bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
  /// 6.2.5.24: The void type comprises an empty set of values; it is an incomplete object type that cannot be completed.
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
  unqualified_type: Rc<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pointer {
  pub pointee: QualifiedType,
}
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionId {
  id: usize,
}

#[derive(Debug, Clone, Display, PartialEq)]
pub enum ArraySize {
  Constant(usize),
  /// unspecified size
  Incomplete,
  /// unsupported dynamic size, but i kept it here for the `full` type category
  ///
  /// TODO: if this holds an expression -- it's a cyclic reference of mod `type` and mod `analyzer::expression`?!
  Variable(ExpressionId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Array {
  /// Array itself cannot have qualifiers,
  /// hence the QualifiedType::qualifiers of the whole array should be empty,
  /// the actual element type's qualifiers are stored here.
  pub element_type: QualifiedType,
  pub size: ArraySize,
  // These are not elem's, but the arraysize's. static is a hint for optimization;
  // pub qualifiers: Qualifiers,
  // pub is_static: bool,
}

/// function types themselves don't have qualifiers, but pointers to them can.
/// so the functionproto's qualifiers must be dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionProto {
  pub return_type: QualifiedType,
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
  pub fn new(qualifiers: Qualifiers, unqualified_type: Rc<Type>) -> Self {
    Self {
      qualifiers,
      unqualified_type,
    }
  }

  pub const fn new_unqualified(unqualified_type: Rc<Type>) -> Self {
    Self {
      qualifiers: Qualifiers::empty(),
      unqualified_type,
    }
  }

  pub fn void() -> Self {
    Self::new_unqualified(Type::void().into())
  }

  pub fn bool() -> Self {
    Self::new_unqualified(Type::bool().into())
  }

  pub fn int() -> Self {
    Self::new_unqualified(Type::int().into())
  }

  pub fn float() -> Self {
    Self::new_unqualified(Type::float().into())
  }

  pub fn nullptr() -> Self {
    Self::new_unqualified(Type::nullptr().into())
  }

  pub fn char() -> Self {
    Self::new_unqualified(Type::char().into())
  }
}
impl ::std::ops::Deref for QualifiedType {
  type Target = Type;

  fn deref(&self) -> &Self::Target {
    &self.unqualified_type
  }
}
impl Pointer {
  pub fn new(pointee: QualifiedType) -> Self {
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
  #[allow(clippy::declare_interior_mutable_const)]
  const MAIN_PROTO_ARGS: Lazy<FunctionProto> = Lazy::new(|| {
    FunctionProto::new(
      Primitive::Int.into(),
      vec![
        Primitive::Int.into(),
        Pointer::new(Pointer::new(Primitive::Char.into()).into()).into(),
      ],
      false,
    )
  });
  #[allow(clippy::declare_interior_mutable_const)]
  const MAIN_PROTO_EMPTY: Lazy<FunctionProto> =
    Lazy::new(|| FunctionProto::new(Primitive::Int.into(), vec![], false));

  pub fn new(
    return_type: QualifiedType,
    parameter_types: Vec<QualifiedType>,
    is_variadic: bool,
  ) -> Self {
    Self {
      return_type,
      parameter_types,
      is_variadic,
    }
  }

  #[allow(clippy::borrow_interior_mutable_const)]
  pub fn main_proto_validate(
    &self,
    function_specifier: FunctionSpecifier,
  ) -> Result<(), DiagMeta> {
    if self.is_variadic {
      Err(
        MainFunctionProtoMismatch("main function cannot be variadic")
          .into_with(Severity::Error),
      )
    } else if function_specifier.contains(FunctionSpecifier::Inline) {
      Err(
        MainFunctionProtoMismatch("main function cannot be inline")
          .into_with(Severity::Error),
      )
    } else if !self.compatible_with(&Self::MAIN_PROTO_EMPTY)
      && !self.compatible_with(&Self::MAIN_PROTO_ARGS)
    {
      Err(
        MainFunctionProtoMismatch(
          "main function must have either no parameters or two parameters \
           (int argc, char** argv)",
        )
        .into_with(Severity::Error),
      )
    } else {
      Ok(())
    }
  }
}

impl Array {
  pub fn new(element_type: QualifiedType, size: ArraySize) -> Self {
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

macro_rules! to_qualified_type {
  ($ty:ty) => {
    impl From<$ty> for QualifiedType {
      fn from(value: $ty) -> Self {
        QualifiedType::new_unqualified(Type::from(value).into())
      }
    }

    impl From<$ty> for Box<QualifiedType> {
      fn from(value: $ty) -> Self {
        Box::new(QualifiedType::from(value))
      }
    }
  };
}

to_qualified_type!(Primitive);
to_qualified_type!(Array);
to_qualified_type!(Pointer);
to_qualified_type!(FunctionProto);
to_qualified_type!(Enum);
to_qualified_type!(Record);
to_qualified_type!(Union);

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

  pub const fn bool() -> Self {
    Type::Primitive(Primitive::Bool)
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
impl QualifiedType {
  pub fn with_qualifiers(mut self, qualifiers: Qualifiers) -> Self {
    self.qualifiers |= qualifiers;
    self
  }

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

  pub fn destructure(self) -> (Qualifiers, Rc<Type>) {
    (self.qualifiers, self.unqualified_type)
  }
}
impl From<Type> for QualifiedType {
  #[inline]
  fn from(value: Type) -> Self {
    QualifiedType::new_unqualified(value.into())
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
    }
  }
}
