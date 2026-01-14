use super::{QualifiedType, Type};

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
  String(String),
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
  ];

  /// parse a numeric literal with optional suffix, if fails, return an error message and the default value of the Constant
  pub fn parse(
    num: &str,
    suffix: Option<&str>,
    is_floating: bool,
  ) -> (Self, Option<String>) {
    match (suffix, is_floating) {
      (None, false) => {
        // default to int
        match num.parse::<i32>() {
          Ok(i) => (Constant::Int(i), None),
          Err(e) => (
            Constant::Int(0),
            Some(format!("Failed to parse integer literal {}: {}", num, e)),
          ),
        }
      },
      (None, true) => {
        // default to double
        match num.parse::<f64>() {
          Ok(f) => (Constant::Double(f), None),
          Err(e) => (
            Constant::Double(0.0),
            Some(format!("Failed to parse floating literal {}: {}", num, e)),
          ),
        }
      },
      (Some(suf), false) => {
        // integer with suffix
        match suf {
          "u" | "U" => match num.parse::<u32>() {
            Ok(u) => (Constant::UInt(u), None),
            Err(e) => (
              Constant::UInt(0),
              Some(format!(
                "Failed to parse unsigned integer literal {}: {}",
                num, e
              )),
            ),
          },
          "l" | "L" => match num.parse::<i64>() {
            Ok(i) => (Constant::LongLong(i), None),
            Err(e) => (
              Constant::LongLong(0),
              Some(format!(
                "Failed to parse long long integer literal {}: {}",
                num, e
              )),
            ),
          },
          "ll" | "LL" => match num.parse::<i64>() {
            Ok(i) => (Constant::LongLong(i), None),
            Err(e) => (
              Constant::LongLong(0),
              Some(format!(
                "Failed to parse long long integer literal {}: {}",
                num, e
              )),
            ),
          },
          "ul" | "uL" | "Ul" | "UL" | "lu" | "lU" | "Lu" | "LU" => {
            match num.parse::<u64>() {
              Ok(u) => (Constant::ULongLong(u), None),
              Err(e) => (
                Constant::ULongLong(0),
                Some(format!(
                  "Failed to parse unsigned long long integer literal {}: {}",
                  num, e
                )),
              ),
            }
          },
          "ull" | "uLL" | "Ull" | "ULL" | "llu" | "llU" | "LLu" | "LLU" =>
            match num.parse::<u64>() {
              Ok(u) => (Constant::ULongLong(u), None),
              Err(e) => (
                Constant::ULongLong(0),
                Some(format!(
                  "Failed to parse unsigned long long integer literal {}: {}",
                  num, e
                )),
              ),
            },
          "z" | "Z" => match num.parse::<isize>() {
            Ok(i) => (Constant::LongLong(i as i64), None),
            Err(e) => (
              Constant::LongLong(0),
              Some(format!(
                "Failed to parse size_t integer literal {}: {}",
                num, e
              )),
            ),
          },
          "uz" | "uZ" | "Uz" | "UZ" | "zu" | "zU" | "Zu" | "ZU" => {
            match num.parse::<usize>() {
              Ok(u) => (Constant::ULongLong(u as u64), None),
              Err(e) => (
                Constant::ULongLong(0),
                Some(format!(
                  "Failed to parse unsigned size_t integer literal {}: {}",
                  num, e
                )),
              ),
            }
          },
          _ => (
            Constant::Int(0),
            Some(format!("unsupported integer literal suffix: {}", suf)),
          ),
        }
      },
      (Some(suf), true) => {
        // floating with suffix
        match suf {
          "f" | "F" => match num.parse::<f32>() {
            Ok(f) => (Constant::Float(f), None),
            Err(e) => (
              Constant::Float(0.0),
              Some(format!("Failed to parse float literal {}: {}", num, e)),
            ),
          },
          "l" | "L" => match num.parse::<f64>() {
            Ok(f) => (Constant::Double(f), None),
            Err(e) => (
              Constant::Double(0.0),
              Some(format!(
                "Failed to parse long double literal {}: {}",
                num, e
              )),
            ),
          },
          _ => (
            Constant::Double(0.0),
            Some(format!("unsupported floating literal suffix: {}", suf)),
          ),
        }
      },
    }
  }

  pub fn unqualified_type(&self) -> Type {
    use super::{Array, ArraySize, Primitive, Qualifiers};

    match self {
      Self::Char(_) => Type::Primitive(Primitive::Char),
      Self::Short(_) => Type::Primitive(Primitive::Short),
      Self::Int(_) => Type::Primitive(Primitive::Int),
      Self::LongLong(_) => Type::Primitive(Primitive::LongLong),
      Self::UChar(_) => Type::Primitive(Primitive::UChar),
      Self::UShort(_) => Type::Primitive(Primitive::UShort),
      Self::UInt(_) => Type::Primitive(Primitive::UInt),
      Self::ULongLong(_) => Type::Primitive(Primitive::ULongLong),
      Self::Float(_) => Type::Primitive(Primitive::Float),
      Self::Double(_) => Type::Primitive(Primitive::Double),
      Self::Bool(_) => Type::Primitive(Primitive::Bool),
      // in C, char[N] is the type of string literal - although it's stored in read-only memory
      // in C++ it's const char[N]
      // ^^^ verified by clangd's AST
      Self::String(str) => Type::Array(Array::new(
        Box::new(QualifiedType::new(
          Qualifiers::empty(),
          Type::Primitive(Primitive::Char),
        )),
        // this is wrong for multi-byte characters, but let's ignore that for now
        ArraySize::Constant(str.len() + 1 /* null terminator */),
      )),
    }
  }

  pub fn is_char_array(&self) -> bool {
    matches!(self, Self::String(_))
  }
}
