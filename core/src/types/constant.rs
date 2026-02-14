use ::rcc_utils::{IntoWith, SmallString};

use crate::{
  blueprints::Placeholder as Nullptr,
  common::{FloatFormat, Floating, Integral, Signedness},
  diagnosis::{DiagData, DiagMeta, Severity},
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
///
/// TODO: named constants `constexpr` and AddressConstant +- constant integral
#[derive(Debug, PartialEq, Clone)]
pub enum Constant {
  Integral(Integral),
  Floating(Floating),
  String(SmallString),
  Nullptr(Nullptr),
  Address(SmallString),
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
    // msvc extensions
    "i8", "i16", "i32", "i64", // signed int
    "ui8", "ui16", "ui32", "ui64", // unsigned int
    // unsupported
    "wb", "WB", // _BitInt
    "uwb", "uWB", "Uwb", "UWB", // unsigned _BitInt
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
          Ok(v) => (Integral::from(v).into(), None),
          Err(e) => (
            Integral::new(
              <$t>::default(),
              <$t>::BITS as u8,
              Signedness::$signess,
            )
            .into(),
            Some(
              DiagData::InvalidNumberFormat(e.to_string())
                .into_with(Severity::Error),
            ),
          ),
        }
      };
    }
    macro_rules! float_conv {
      ($t:ty, $format:ident) => {
        match num.parse::<$t>() {
          Ok(v) => (Floating::from(v).into(), None),
          Err(e) => (
            Floating::new(<$t>::default().to_bits(), FloatFormat::$format)
              .into(),
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
      (None, true) => float_conv!(f64, IEEE64),
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
        "i8" => int_conv!(i8, Signed),
        "i16" => int_conv!(i16, Signed),
        "i32" => int_conv!(i32, Signed),
        "i64" => int_conv!(i64, Signed),
        "ui8" => int_conv!(u8, Unsigned),
        "ui16" => int_conv!(u16, Unsigned),
        "ui32" => int_conv!(u32, Unsigned),
        "ui64" => int_conv!(u64, Unsigned),
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
        "f" | "F" => float_conv!(f32, IEEE32),
        "l" | "L" => float_conv!(f64, IEEE64),
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

  pub fn is_zero(&self) -> bool {
    match self {
      Self::Integral(integral) => integral.is_zero(),
      Self::Floating(floating) => floating.is_zero(),
      Self::String(_) => false,
      Self::Nullptr(_) => true,
      Self::Address(_) => false,
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
      Self::Address(_) => Constant::Integral(Integral::from_bool(true)),
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
::rcc_utils::interconvert!(Integral, Constant);
::rcc_utils::interconvert!(Floating, Constant);
::rcc_utils::interconvert!(SmallString, Constant, String);
::rcc_utils::interconvert!(Nullptr, Constant);
// ::rcc_utils::interconvert!(SmallString, Constant, Address);

::rcc_utils::make_trio_for!(Integral, Constant);
::rcc_utils::make_trio_for!(Floating, Constant);
::rcc_utils::make_trio_for!(Nullptr, Constant);
::rcc_utils::make_trio_for!(SmallString, Constant, String);
::rcc_utils::make_trio_for!(SmallString, Constant, Address);
