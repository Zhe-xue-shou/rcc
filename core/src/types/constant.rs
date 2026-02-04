use ::rc_utils::{IntoWith, interconvert, make_trio_for};

use crate::{
  blueprints::Placeholder as Nullptr,
  common::{FloatFormat, Floating, Integral, Signedness},
  diagnosis::{DiagData, DiagMeta, Severity},
  types::Type,
};

/// This class is **mixed**.
///
/// In lexer, it only represents numbers.
/// In parser and later stages, it represents all kinds of constant values.
///
/// Also in later stage, it has a redundant variant tag -- like a discriminated union;
///
/// it's not needed since the type system can represent the type of the constant -- an indiscriminated union would be sufficient.
///
/// discrepancy: string literals are not constant values in C `char[N]`
/// (but in C++, it is, though. verified by clangd's AST: `const char[N]`.)
#[derive(Debug, PartialEq, Clone)]
pub enum Constant {
  Integral(Integral),
  Floating(Floating),
  String(String),
  Nullptr(Nullptr),
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
    macro_rules! int_conv {
      ($t:ty, $signess:ident) => {
        match num.parse::<$t>() {
          Ok(v) => (
            Integral::new(v, (<$t>::BITS) as u8, Signedness::$signess).into(),
            None,
          ),
          Err(e) => (
            Integral::default().into(),
            Some(
              DiagData::InvalidNumberFormat(e.to_string())
                .into_with(Severity::Error),
            ),
          ),
        }
      };
    }
    macro_rules! float_conv {
      ($t:ty, $format:expr) => {
        match num.parse::<$t>() {
          Ok(v) => (Floating::new(v.to_bits(), $format).into(), None),
          Err(e) => (
            Floating::new(<$t>::default().to_bits(), $format).into(),
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
      (None, false) => int_conv!(i32, Signed),
      // default to double
      (None, true) => float_conv!(f64, FloatFormat::IEEE64),
      // integer with suffix
      (Some(suf), false) => match suf {
        "u" | "U" => int_conv!(u32, Unsigned),
        "l" | "L" => int_conv!(i64, Signed),
        "ll" | "LL" => int_conv!(i64, Signed),
        "ul" | "uL" | "Ul" | "UL" | "lu" | "lU" | "Lu" | "LU" =>
          int_conv!(u64, Unsigned),
        "ull" | "uLL" | "Ull" | "ULL" | "llu" | "llU" | "LLu" | "LLU" =>
          int_conv!(u64, Unsigned),
        "z" | "Z" => int_conv!(isize, Signed),
        "uz" | "uZ" | "Uz" | "UZ" | "zu" | "zU" | "Zu" | "ZU" =>
          int_conv!(usize, Unsigned),
        _ => (
          Integral::default().into(),
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
        "f" | "F" => float_conv!(f32, FloatFormat::IEEE32),
        "l" | "L" => float_conv!(f64, FloatFormat::IEEE64),
        _ => (
          Floating::default().into(),
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

  pub const fn is_char_array(&self) -> bool {
    matches!(self, Self::String(_))
  }

  pub fn unqualified_type(&self) -> Type {
    match self {
      Self::Integral(integral) => integral.unqualified_type(),
      Self::Floating(floating) => floating.unqualified_type(),
      Self::String(str) => Type::char_array(str.len() + 1),
      Self::Nullptr(_) => Type::nullptr(),
    }
  }

  pub fn is_zero(&self) -> bool {
    match self {
      Self::Integral(integral) => integral.is_zero(),
      Self::Floating(floating) => floating.is_zero(),
      Self::String(_) => false,
      Self::Nullptr(_) => true,
    }
  }

  pub fn to_boolean(self) -> Constant {
    match self {
      Self::Integral(integral) =>
        Constant::Integral(Integral::from_bool(!integral.is_zero())),
      Self::Floating(floating) =>
        Constant::Integral(Integral::from_bool(!floating.is_zero())),
      Self::String(_) => Constant::Integral(Integral::from_bool(true)),
      Self::Nullptr(_) => Constant::Integral(Integral::from_bool(false)),
    }
  }

  pub fn to_integral(self, width: u8, signedness: Signedness) -> Constant {
    match self {
      Self::Integral(integral) =>
        Constant::Integral(integral.cast(width, signedness)),
      Self::Floating(floating) =>
        Constant::Integral(floating.to_integral(width, signedness)),
      _ => unreachable!("handled elsewhere"),
    }
  }

  pub fn to_floating(self, format: FloatFormat) -> Constant {
    match self {
      Self::Integral(integral) =>
        Constant::Floating(integral.to_floating(format)),
      Self::Floating(floating) => Constant::Floating(floating),
      _ => unreachable!("handled elsewhere"),
    }
  }
}
interconvert!(Integral, Constant);
interconvert!(Floating, Constant);
interconvert!(String, Constant);
interconvert!(Nullptr, Constant);

make_trio_for!(Integral, Constant);
make_trio_for!(Floating, Constant);
make_trio_for!(Nullptr, Constant);
make_trio_for!(String, Constant);
