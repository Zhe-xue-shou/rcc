// C's built-in types
#[derive(
  Debug,
  Clone,
  PartialEq,
  ::strum_macros::Display,
  ::strum_macros::IntoStaticStr,
  ::strum_macros::EnumString,
)]
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
