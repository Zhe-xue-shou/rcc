use ::strum_macros::{Display, EnumString, IntoStaticStr};

pub mod compatible;
pub mod fmt;
pub mod promotion;
pub mod type_info;
pub mod types;

bitflags::bitflags! {
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
  pub qualifiers: Qualifiers,
  pub unqualified_type: Type,
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

/// rules about the `metadata`. used for declaration and definition.
#[allow(unused)]
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
#[allow(unused)]
pub trait TypeInfo {
  fn size(&self) -> usize;
  fn is_scalar(&self) -> bool;
}
pub trait Promotion {
  #[must_use]
  fn promote(self) -> (Self, CastType)
  where
    Self: Sized;
}
#[derive(Debug, Display)]
pub enum CastType {
  Noop, // don't use this for implicit casts - in that case no cast is needed; only used for explicit casts like (int)x where x is already int
  ToVoid, // (void)expr

  LValueToRValue,         // Read value from a variable (6.3.2.1)
  ArrayToPointerDecay,    // int[10] -> int*
  FunctionToPointerDecay, // void f() -> void(*)()
  NullptrToPointer,       // nullptr -> ptr

  IntegralCast, // int -> long, unsigned -> int - bit widening/narrowing
  IntegralToFloating, // int -> float
  IntegralToBoolean, // int -> bool (val != 0)

  FloatingCast,       // float -> double
  FloatingToIntegral, // float -> int
  FloatingToBoolean,  // float -> bool (val != 0.0)

  IntegralToPointer, // int -> ptr (addr 0 is null)
  PointerToIntegral,
  PointerToBoolean, // ptr -> bool (ptr != 0)
  BitCast, // pesudo cast; no actual conversion, just reinterpret the bits

  // ^^^ those exist in Clang's frontend too
  // vvv custom casts
  NullptrToIntegral, // nullptr -> int
  NullptrToBoolean,  // nullptr -> bool
}
