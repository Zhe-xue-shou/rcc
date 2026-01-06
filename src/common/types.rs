use ::bitflags::bitflags;
use ::once_cell::sync::Lazy;
use ::std::{cell::LazyCell, str::FromStr};
use ::strum_macros::{Display, EnumString, IntoStaticStr};

use crate::{
  breakpoint,
  common::{error::Error, rawdecl::FunctionSpecifier},
};

// C's built-in types, I only consider x86_64 here for simplicity
#[derive(Debug, Display, IntoStaticStr, EnumString, Clone, PartialEq)]
pub enum Primitive {
  // assume 64bit
  #[strum(serialize = "char")]
  #[strum(serialize = "signed char")]
  Char, // signed char/plain char
  #[strum(serialize = "short")]
  Short, // short
  #[strum(serialize = "long")]
  #[strum(serialize = "int")]
  Int, // int/long
  #[strum(serialize = "long long")]
  LongLong, // long long
  #[strum(serialize = "unsigned char")]
  UChar, // unsigned char
  #[strum(serialize = "unsigned short")]
  UShort, // unsigned short
  #[strum(serialize = "unsigned long")]
  #[strum(serialize = "unsigned int")]
  UInt, // unsigned int/unsigned long
  #[strum(serialize = "unsigned long long")]
  ULongLong, // unsigned long long
  #[strum(serialize = "float")]
  Float, // float
  #[strum(serialize = "long double")]
  #[strum(serialize = "double")]
  Double,
  #[strum(serialize = "void")]
  Void,
  // ignore below for now
  #[strum(serialize = "_Complex")]
  #[strum(serialize = "_Complex float")]
  ComplexFloat,
  #[strum(serialize = "_Complex double")]
  #[strum(serialize = "_Complex long double")]
  ComplexDouble,
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
  pub fn size(&self) -> usize {
    self.unqualified_type.size()
  }
}
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
#[derive(Debug, Clone, PartialEq)]
pub struct Pointer {
  pub pointee: Box<QualifiedType>,
}
impl Pointer {
  pub fn new(pointee: Box<QualifiedType>) -> Self {
    Self { pointee }
  }
}
#[derive(Debug, Clone, Display)]
pub enum ArraySize {
  Constant(usize),
  Incomplete,
  // Variable, // ignore for now
}
impl Compatibility for ArraySize {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(r)) => l == r,
      (Self::Incomplete, Self::Incomplete)
      | (Self::Constant(_), Self::Incomplete)
      | (Self::Incomplete, Self::Constant(_)) => true,
    }
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(r)) => {
        if l == r {
          Some(Self::Constant(*l))
        } else {
          None
        }
      }
      (Self::Incomplete, Self::Incomplete) => Some(Self::Incomplete),
      (Self::Constant(l), Self::Incomplete) | (Self::Incomplete, Self::Constant(l)) => {
        Some(Self::Constant(*l))
      }
    }
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(_)) => Self::Constant(*l),
      (Self::Incomplete, Self::Incomplete) => Self::Incomplete,
      (Self::Constant(l), Self::Incomplete) | (Self::Incomplete, Self::Constant(l)) => {
        Self::Constant(*l)
      }
    }
  }
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
/// rules about the `metadata`. used for declaration and definition.
pub trait Compatibility {
  fn compatible(lhs: &Self, rhs: &Self) -> bool;
  #[inline]
  fn compatible_with(&self, other: &Self) -> bool {
    Self::compatible(self, other)
  }
  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized;
  #[inline]
  fn composite_with(&self, other: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    Self::composite(self, other)
  }
  /// used internally to avoid unnecessary compatibility checks
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized;
  #[inline]
  fn composite_unchecked_with(&self, other: &Self) -> Self
  where
    Self: Sized,
  {
    Self::composite_unchecked(self, other)
  }
}

impl Compatibility for Enum {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    todo!()
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    todo!()
  }

  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
}
impl Compatibility for Record {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    todo!()
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    todo!()
  }

  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
}
impl Compatibility for Pointer {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    QualifiedType::compatible(&lhs.pointee, &rhs.pointee)
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      return None;
    }
    let pointee = QualifiedType::composite_unchecked(&lhs.pointee, &rhs.pointee);
    Some(Self::new(Box::new(pointee)))
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    let pointee = QualifiedType::composite_unchecked(&lhs.pointee, &rhs.pointee);
    Self::new(Box::new(pointee))
  }
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
      Primitive::Char | Primitive::UChar => 1,
      Primitive::Short | Primitive::UShort => 2,
      Primitive::Int | Primitive::UInt | Primitive::Float => 4,
      Primitive::LongLong | Primitive::ULongLong | Primitive::Double => 8,
      Primitive::Void => 0,
      _ => unimplemented!(),
    }
  }
}
impl Compatibility for Primitive {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    lhs == rhs
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs))
    }
  }
  fn composite_unchecked(lhs: &Self, _rhs: &Self) -> Self
  where
    Self: Sized,
  {
    lhs.clone()
  }
}
impl FunctionProto {
  const MAIN_PROTO_EMPTY: Lazy<FunctionProto> = Lazy::new(|| {
    FunctionProto::new(
      QualifiedType::new(Qualifiers::empty(), Type::Primitive(Primitive::Int)),
      vec![],
      false,
    )
  });
  const MAIN_PROTO_ARGS: Lazy<FunctionProto> = Lazy::new(|| {
    FunctionProto::new(
      QualifiedType::new(Qualifiers::empty(), Type::Primitive(Primitive::Int)),
      vec![QualifiedType::new(
        Qualifiers::empty(),
        Type::Pointer(Pointer::new(Box::new(QualifiedType::new(
          Qualifiers::empty(),
          Type::Primitive(Primitive::Char),
        )))),
      )],
      true,
    )
  });
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
  pub fn main_proto_validate(&self, function_specifier: FunctionSpecifier) -> Result<(), Error> {
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
impl Compatibility for FunctionProto {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    if lhs.is_variadic != rhs.is_variadic {
      return false;
    }
    // 6.7.7.4.13: For two function types to be compatible, both shall specify compatible return types.
    if !QualifiedType::compatible(&lhs.return_type, &rhs.return_type) {
      return false;
    }
    if lhs.parameter_types.len() != rhs.parameter_types.len() {
      return false;
    }
    // THIS IS A NASTY EXCEPTION:
    //  In the determination of type compatibility and of a composite type,
    //     each parameter declared with function or array type is taken as having the
    //     adjusted type and each parameter declared with qualified type is taken as having the unqualified
    //     version of its declared type.
    for (lparam, rparam) in lhs.parameter_types.iter().zip(rhs.parameter_types.iter()) {
      if !Type::compatible(&lparam.unqualified_type, &rparam.unqualified_type) {
        return false;
      }
    }

    true
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      return None;
    } else {
      Some(Self::composite_unchecked(lhs, rhs))
    }
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    let return_type = QualifiedType::composite_unchecked(&lhs.return_type, &rhs.return_type);
    let mut parameter_types = Vec::new();
    for (lparam, rparam) in lhs.parameter_types.iter().zip(rhs.parameter_types.iter()) {
      let param_type = QualifiedType::new(
        // this is actually not strictly correct -
        // e.g., const decl + non-const def -> var is const, non-const decl + const def -> var is non-const
        lparam.qualifiers | rparam.qualifiers,
        Type::composite_unchecked(&lparam.unqualified_type, &rparam.unqualified_type),
      );
      parameter_types.push(param_type);
    }
    Self::new(return_type, parameter_types, lhs.is_variadic)
  }
}
impl Type {
  #[inline]
  pub fn is_function_proto(&self) -> bool {
    matches!(self, Type::FunctionProto(_))
  }
  #[inline]
  pub fn is_pointer(&self) -> bool {
    matches!(self, Type::Pointer(_))
  }
  #[inline]
  pub fn is_array(&self) -> bool {
    matches!(self, Type::Array(_))
  }
  #[inline]
  pub fn is_primitive(&self) -> bool {
    matches!(self, Type::Primitive(_))
  }
  #[inline]
  pub fn is_enumeration(&self) -> bool {
    matches!(self, Type::Enum(_))
  }
  #[inline]
  pub fn is_record(&self) -> bool {
    matches!(self, Type::Record(_))
  }
  #[inline]
  pub fn is_variant(&self) -> bool {
    matches!(self, Type::Union(_))
  }
  #[inline]
  pub fn function_proto(&self) -> &FunctionProto {
    match self {
      Type::FunctionProto(function_proto) => function_proto,
      _ => panic!("Type is not FunctionProto"),
    }
  }
  #[inline]
  pub fn pointer(&self) -> &Pointer {
    match self {
      Type::Pointer(pointer) => pointer,
      _ => panic!("Type is not Pointer"),
    }
  }
  #[inline]
  pub fn array(&self) -> &Array {
    match self {
      Type::Array(array_type) => array_type,
      _ => panic!("Type is not Array"),
    }
  }
  #[inline]
  pub fn primitive(&self) -> &Primitive {
    match self {
      Type::Primitive(primitive) => primitive,
      _ => panic!("Type is not Primitive"),
    }
  }
  #[inline]
  pub fn enumeration(&self) -> &Enum {
    match self {
      Type::Enum(enum_type) => enum_type,
      _ => panic!("Type is not Enum"),
    }
  }
  #[inline]
  pub fn record(&self) -> &Record {
    match self {
      Type::Record(record_type) => record_type,
      _ => panic!("Type is not Record"),
    }
  }
  // union is a (soft) keyword in Rust, so use 'variant' here(from std::variant in C++)
  #[inline]
  pub fn variant(&self) -> &Union {
    match self {
      Type::Union(variant_type) => variant_type,
      _ => panic!("Type is not Union"),
    }
  }
  #[inline]
  pub fn to_function_proto(self) -> FunctionProto {
    match self {
      Type::FunctionProto(function_proto) => function_proto,
      _ => panic!("Type is not FunctionProto"),
    }
  }
  #[inline]
  pub fn to_pointer(self) -> Pointer {
    match self {
      Type::Pointer(pointer) => pointer,
      _ => panic!("Type is not Pointer"),
    }
  }

  #[inline]
  pub fn to_array(self) -> Array {
    match self {
      Type::Array(array_type) => array_type,
      _ => panic!("Type is not Array"),
    }
  }

  #[inline]
  pub fn to_primitive(self) -> Primitive {
    match self {
      Type::Primitive(primitive) => primitive,
      _ => panic!("Type is not Primitive"),
    }
  }
  #[inline]
  pub fn to_enumeration(self) -> Enum {
    match self {
      Type::Enum(enum_type) => enum_type,
      _ => panic!("Type is not Enum"),
    }
  }
  #[inline]
  pub fn to_record(self) -> Record {
    match self {
      Type::Record(record_type) => record_type,
      _ => panic!("Type is not Record"),
    }
  }
  #[inline]
  pub fn to_variant(self) -> Union {
    match self {
      Type::Union(variant_type) => variant_type,
      _ => panic!("Type is not Union"),
    }
  }
}
impl Type {
  pub fn size(&self) -> usize {
    match self {
      Type::Primitive(p) => p.size(),
      Type::Pointer(_) => Primitive::ULongLong.size(),
      Type::Enum(_) => Primitive::LongLong.size(),
      Type::Record(r) => r
        .fields
        .iter()
        .map(|f| f.field_type.unqualified_type.size())
        .sum(), // rough, padding and alignment not considered -- incomplete type has no members anyway so this handles it too
      Type::Union(u) => u
        .fields
        .iter()
        .map(|f| f.field_type.unqualified_type.size())
        .max()
        .unwrap_or(0), // ditto
      _ => unimplemented!(),
    }
  }
}

impl Compatibility for Type {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    match (lhs, rhs) {
      (Type::Primitive(l), Type::Primitive(r)) => Primitive::compatible(l, r),
      (Type::Pointer(l), Type::Pointer(r)) => Pointer::compatible(l, r),
      (Type::Array(l), Type::Array(r)) => Array::compatible(l, r),
      (Type::FunctionProto(l), Type::FunctionProto(r)) => FunctionProto::compatible(l, r),
      (Type::Enum(l), Type::Enum(r)) => Enum::compatible(l, r),
      (Type::Record(l), Type::Record(r)) => Record::compatible(l, r),
      (Type::Union(l), Type::Union(r)) => Union::compatible(l, r),
      _ => false,
    }
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs))
    }
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Type::Primitive(l), Type::Primitive(r)) => {
        Type::Primitive(Primitive::composite_unchecked(l, r))
      }
      (Type::Pointer(l), Type::Pointer(r)) => Type::Pointer(Pointer::composite_unchecked(l, r)),
      (Type::Array(l), Type::Array(r)) => Type::Array(Array::composite_unchecked(l, r)),
      (Type::FunctionProto(l), Type::FunctionProto(r)) => {
        Type::FunctionProto(FunctionProto::composite_unchecked(l, r))
      }
      (Type::Enum(l), Type::Enum(r)) => Type::Enum(Enum::composite_unchecked(l, r)),
      (Type::Record(l), Type::Record(r)) => Type::Record(Record::composite_unchecked(l, r)),
      (Type::Union(l), Type::Union(r)) => Type::Union(Union::composite_unchecked(l, r)),
      _ => {
        breakpoint!();
        unreachable!()
      }
    }
  }
}

impl Compatibility for QualifiedType {
  fn compatible(lhs: &QualifiedType, rhs: &QualifiedType) -> bool {
    // 6.2.7.1: Two types are compatible types if they are the same.
    if lhs == rhs {
      return true;
    }
    // 6.7.4.1.11: For two qualified types to be compatible, both shall have the identically qualified version of a compatible type.
    if lhs.qualifiers != rhs.qualifiers {
      return false;
    }
    <Type as Compatibility>::compatible(&lhs.unqualified_type, &rhs.unqualified_type)
  }
  fn composite(lhs: &QualifiedType, rhs: &QualifiedType) -> Option<QualifiedType> {
    if !QualifiedType::compatible(lhs, rhs) {
      return None;
    }
    Some(Self::composite_unchecked(lhs, rhs))
  }
  fn composite_unchecked(lhs: &QualifiedType, rhs: &QualifiedType) -> QualifiedType
  where
    Self: Sized,
  {
    // there's some nasty rules about merging qualifiers for function types -- handled in analyzer
    // also nasty rules about arrays -- todo
    // struct, enum, union -- todo
    // alignment specifier -- don't care
    // QualifiedType::new(qualifiers, lhs.unqualified_type.clone())
    todo!()
  }
}

impl Array {
  pub fn new(element_type: Box<QualifiedType>, size: ArraySize) -> Self {
    Self { element_type, size }
  }
}
impl Compatibility for Array {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    if !QualifiedType::compatible(&lhs.element_type, &rhs.element_type) {
      false
    } else {
      ArraySize::compatible(&lhs.size, &rhs.size)
    }
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs))
    }
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    let element_type =
      <QualifiedType as Compatibility>::composite_unchecked(&lhs.element_type, &rhs.element_type);
    let size = ArraySize::composite_unchecked(&lhs.size, &rhs.size);
    Self::new(Box::new(element_type), size)
  }
}
mod compare {
  use super::ArraySize;

  impl PartialEq for ArraySize {
    fn eq(&self, other: &Self) -> bool {
      match (self, other) {
        (Self::Constant(lhs), Self::Constant(rhs)) => lhs == rhs,
        (Self::Incomplete, Self::Incomplete)
        | (Self::Constant(_), Self::Incomplete)
        | (Self::Incomplete, Self::Constant(_)) => true,
      }
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
