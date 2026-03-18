use ::rcc_utils::ensure_is_pod;

use crate::{
  blueprints::Placeholder as Nullptr,
  common::{FloatFormat, Floating, Integral, RefEq, Signedness, StrRef},
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
/// TODO: named constants `constexpr` and constant integral
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constant<'c> {
  Integral(Integral),
  Floating(Floating),
  Nullptr(Nullptr),
  String(StrRef<'c>),
  Address(StrRef<'c>),
}
ensure_is_pod!(Constant);
pub type ConstantRef<'c> = &'c Constant<'c>;
pub type ConstantRefMut<'c> = &'c mut Constant<'c>;
impl RefEq for ConstantRef<'_> {
  fn ref_eq(lhs: Self, rhs: Self) -> bool
  where
    Self: PartialEq + Sized,
  {
    let ref_eq = ::std::ptr::eq(lhs, rhs);
    if const { cfg!(debug_assertions) } {
      let actual_eq = lhs == rhs;
      if ref_eq != actual_eq {
        eprintln!(
          "INTERNAL ERROR: comparing by pointer address result did not match 
          the actual result: {:p}: {:?} and {:p}: {:?}
        ",
          lhs, lhs, rhs, rhs
        );
      }
      return actual_eq;
    }
    ref_eq
  }
}
impl<'c> Constant<'c> {
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
  pub const NULLPTR: Self = Self::Nullptr(Nullptr);

  /// parse a numeric literal with optional suffix, if fails, return an error message and the default value of the Constant
  pub fn parse(
    num: &str,
    base: u32,
    suffix: Option<&str>,
    is_floating: bool,
  ) -> (Self, Option<DiagMeta<'c>>) {
    use ::rcc_utils::IntoWith;

    macro_rules! int_conv {
      ($t:ty, $signess:ident) => {
        match <$t>::from_str_radix(num, base) {
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
      Self::String(s) => s.is_empty(),
      Self::Nullptr(_) => true,
      Self::Address(_) => false,
    }
  }

  pub fn to_boolean(self) -> Self {
    match self {
      Self::Integral(integral) =>
        Constant::Integral(Integral::from_bool(!integral.is_zero())),
      Self::Floating(floating) =>
        Constant::Integral(Integral::from_bool(!floating.is_zero())),
      Self::String(s) => Constant::Integral(Integral::from_bool(s.is_empty())),
      Self::Nullptr(_) => Constant::Integral(Integral::from_bool(false)),
      Self::Address(_) => Constant::Integral(Integral::from_bool(true)),
    }
  }

  pub fn to_integral(self, width: u8, signedness: Signedness) -> Self {
    match self {
      Self::Integral(integral) =>
        Constant::Integral(integral.cast(width, signedness)),
      Self::Floating(floating) =>
        Constant::Integral(floating.to_integral(width, signedness)),
      _ => unreachable!("handled elsewhere"),
    }
  }

  pub fn to_floating(self, format: FloatFormat) -> Self {
    match self {
      Self::Integral(integral) => Self::Floating(integral.to_floating(format)),
      Self::Floating(floating) => Self::Floating(floating),
      _ => unreachable!("handled elsewhere"),
    }
  }

  pub fn is_address(&self) -> bool {
    matches!(self, Constant::Address(_))
  }
}
::rcc_utils::interconvert!(Integral, Constant<'c>);
::rcc_utils::interconvert!(Floating, Constant<'c>);
// ::rcc_utils::interconvert!(???, Constant, String);
::rcc_utils::interconvert!(Nullptr, Constant<'c>);
// ::rcc_utils::interconvert!(???, Constant, Address);

::rcc_utils::make_trio_for!(Integral, Constant<'c>);
::rcc_utils::make_trio_for!(Floating, Constant<'c>);
::rcc_utils::make_trio_for!(Nullptr, Constant<'c>);

// ::rcc_utils::make_trio_for!(???, Constant, String);
// ::rcc_utils::make_trio_for!(???, Constant, Address);
