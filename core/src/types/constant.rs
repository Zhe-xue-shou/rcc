use ::rc_utils::IntoWith;

use super::{QualifiedType, Type};
use crate::diagnosis::{DiagData, DiagMeta, Severity};

/// This class is **mixed**.
///
/// In lexer, it only represents numbers.
/// In parser and later stages, it represents all kinds of constant values.
///
/// Also in later stage, it has a redundant variant tag -- like a discriminated union;
///
/// it's not needed since the type system can represent the type of the constant -- an indiscriminated union would be sufficient.
///
/// TODO: move StringLiteral and Nullptr out, so that it did not need [`Drop`].
#[derive(Debug, PartialEq, Clone)]
pub enum Constant {
  Char(i8),
  Short(i16),
  Int(i32),
  LongLong(i64),
  UChar(u8),
  UShort(u16),
  UInt(u32),
  ULongLong(u64),
  Float(f32),
  Double(f64),
  Bool(bool),
  StringLiteral(String),
  Nullptr,
}

impl Constant {
  pub const FLOATING_SUFFIXES: &'static [&'static str] = &[
    "f", "F", // float
    "l", "L", // long double
    // unsupported
    "df", "DF", // _Decimal32
    "dd", "DD", // _Decimal64
    "dl", "DL", // _Decimal128
  ];
  // literal suffixes
  pub const INTEGER_SUFFIXES: &'static [&'static str] = &[
    "u", "U", // unsigned
    "l", "L", // long
    "ll", "LL", // long long
    "ul", "uL", "Ul", "UL", "lu", "lU", "Lu", "LU", // unsigned long
    "ull", "uLL", "Ull", "ULL", "llu", "llU", "LLu",
    "LLU", // unsigned long long
    "uz", "uZ", "Uz", "UZ", "zu", "zU", "Zu", "ZU", // size_t
    "z", "Z", // size_t's signed version
    // unsupported
    "wb", "WB", // _BitInt
    "uwb", "uWB", "Uwb", "UWB", // unsigned _BitInt
    // msvc extensions
    "i8", "i16", "i32", "i64", // signed int
    "ui8", "ui16", "ui32", "ui64", // unsigned int
  ];

  /// parse a numeric literal with optional suffix, if fails, return an error message and the default value of the Constant
  pub fn parse(
    num: &str,
    suffix: Option<&str>,
    is_floating: bool,
  ) -> (Self, Option<DiagMeta>) {
    macro_rules! conv {
      ($t:ty, $variant:ident) => {
        match num.parse::<$t>() {
          Ok(v) => (Constant::$variant(v), None),
          Err(e) => (
            Constant::$variant(Default::default()),
            Some(
              DiagData::InvalidNumberFormat(e.to_string())
                .into_with(Severity::Error),
            ),
          ),
        }
      };
      ($t:ty, $variant:ident, $cast:ty) => {
        match num.parse::<$t>() {
          Ok(v) => (Constant::$variant(v as $cast), None),
          Err(e) => (
            Constant::$variant(Default::default()),
            Some(
              DiagData::InvalidNumberFormat(e.to_string())
                .into_with(Severity::Error),
            ),
          ),
        }
      };
    }
    match (suffix, is_floating) {
      // default to int
      (None, false) => conv!(i32, Int),
      // default to double
      (None, true) => conv!(f64, Double),
      // integer with suffix
      (Some(suf), false) => match suf {
        "u" | "U" => conv!(u32, UInt),
        "l" | "L" => conv!(i64, LongLong),
        "ll" | "LL" => conv!(i64, LongLong),
        "ul" | "uL" | "Ul" | "UL" | "lu" | "lU" | "Lu" | "LU" =>
          conv!(u64, ULongLong),
        "ull" | "uLL" | "Ull" | "ULL" | "llu" | "llU" | "LLu" | "LLU" =>
          conv!(u64, ULongLong),
        "z" | "Z" => conv!(isize, LongLong, i64),
        "uz" | "uZ" | "Uz" | "UZ" | "zu" | "zU" | "Zu" | "ZU" =>
          conv!(usize, ULongLong, u64),
        _ => (
          Constant::Int(Default::default()),
          Some(
            DiagData::InvalidNumberFormat(format!(
              "unsupported integer literal suffix: {}",
              suf
            ))
            .into_with(Severity::Error),
          ),
        ),
      },
      // floating with suffix
      (Some(suf), true) => match suf {
        "f" | "F" => conv!(f32, Float),
        "l" | "L" => conv!(f64, Double),
        _ => (
          Constant::Double(Default::default()),
          Some(
            DiagData::InvalidNumberFormat(format!(
              "unsupported floating literal suffix: {}",
              suf
            ))
            .into_with(Severity::Error),
          ),
        ),
      },
    }
  }

  pub fn unqualified_type(&self) -> Type {
    use super::{Array, ArraySize, Primitive::*};

    match self {
      Self::Char(_) => Char.into(),
      Self::Short(_) => Short.into(),
      Self::Int(_) => Int.into(),
      Self::LongLong(_) => LongLong.into(),
      Self::UChar(_) => UChar.into(),
      Self::UShort(_) => UShort.into(),
      Self::UInt(_) => UInt.into(),
      Self::ULongLong(_) => ULongLong.into(),
      Self::Float(_) => Float.into(),
      Self::Double(_) => Double.into(),
      Self::Bool(_) => Bool.into(),
      Self::Nullptr => Nullptr.into(),
      // in C, char[N] is the type of string literal - although it's stored in read-only memory
      // in C++ it's const char[N]
      // ^^^ verified by clangd's AST
      Self::StringLiteral(str) => Array::new(
        QualifiedType::char().into(),
        // this is wrong for multi-byte characters, but let's ignore that for now
        ArraySize::Constant(str.len() + 1 /* null terminator */),
      )
      .into(),
    }
  }

  pub fn is_char_array(&self) -> bool {
    matches!(self, Self::StringLiteral(_))
  }

  pub fn is_integer(&self) -> bool {
    matches!(
      self,
      Self::Char(_)
        | Self::Short(_)
        | Self::Int(_)
        | Self::LongLong(_)
        | Self::UChar(_)
        | Self::UShort(_)
        | Self::UInt(_)
        | Self::ULongLong(_)
    )
  }

  pub fn is_floating(&self) -> bool {
    matches!(self, Self::Float(_) | Self::Double(_))
  }

  pub fn is_boolean(&self) -> bool {
    matches!(self, Self::Bool(_))
  }

  pub fn is_zero(&self) -> bool {
    match self {
      Self::Char(c) => *c == 0,
      Self::Short(s) => *s == 0,
      Self::Int(i) => *i == 0,
      Self::LongLong(l) => *l == 0,
      Self::UChar(u) => *u == 0,
      Self::UShort(u) => *u == 0,
      Self::UInt(u) => *u == 0,
      Self::ULongLong(u) => *u == 0,
      Self::Float(f) => *f == 0.0,
      Self::Double(d) => *d == 0.0,
      Self::Bool(b) => !*b,
      Self::Nullptr => true,
      Self::StringLiteral(s) => s.is_empty(),
    }
  }

  pub fn is_nullptr(&self) -> bool {
    matches!(self, Self::Nullptr)
  }
}

impl TryFrom<Constant> for usize {
  type Error = DiagMeta;

  fn try_from(value: Constant) -> Result<Self, Self::Error> {
    match value {
      Constant::Char(c) if c >= 0 => Ok(c as Self),
      Constant::Short(s) if s >= 0 => Ok(s as Self),
      Constant::Int(i) if i >= 0 => Ok(i as Self),
      Constant::LongLong(l) if l >= 0 => Ok(l as Self),
      Constant::UChar(u) => Ok(u as Self),
      Constant::UShort(u) => Ok(u as Self),
      Constant::UInt(u) => Ok(u as Self),
      Constant::ULongLong(u) => Ok(u as Self),
      Constant::Bool(b) => Ok(if b { 1 } else { 0 }),
      Constant::Nullptr => Ok(0),
      _ => Err(DiagMeta::new(
        Severity::Error,
        DiagData::InvalidConversion(
          "Array declaration size must be a non-negative integer".to_string(),
        ),
      )),
    }
  }
}
