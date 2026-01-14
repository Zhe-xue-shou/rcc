use crate::common::{
  operator::Operator,
  types::{QualifiedType, Type},
};

#[macro_export(local_inner_macros)]
macro_rules! type_alias_expr {
  ($exprty:ident,$typety:ident $(, $extra:ident)*) => {
    /// likely a sophisticated version of the Two-Level Types
    /// [this article](https://blog.ezyang.com/2013/05/the-ast-typing-problem/),
    /// I probably used the Parametric Polymorphism to "tie the knot" of recursion.
    #[derive(Debug)]
    pub enum RawExpr {
      Empty, // no-op for error recovery; for empty expr should use Option<ExprTy> instead
      Constant(Constant),
      Unary(Unary),
      Binary(Binary),
      Call(Call),
      MemberAccess(MemberAccess),
      Ternary(Ternary),
      SizeOf(SizeOf),
      CStyleCast(CStyleCast),                     // (int)x
      ArraySubscript(ArraySubscript), // arr[i]
      CompoundLiteral(CompoundLiteral), // (struct Point){.x=1, .y=2}
      $(
        // Generate a variant for each extra type
        $extra($extra),
      )*
    }
    pub type Constant = crate::common::rawexpr::Constant;
    pub type Unary = crate::common::rawexpr::Unary<$exprty>;
    pub type Binary = crate::common::rawexpr::Binary<$exprty>;
    pub type Call = crate::common::rawexpr::Call<$exprty>;
    pub type MemberAccess = crate::common::rawexpr::MemberAccess<$exprty>;
    pub type Ternary = crate::common::rawexpr::Ternary<$exprty>;
    pub type SizeOf = crate::common::rawexpr::SizeOf<$exprty, $typety>;
    pub type CStyleCast = crate::common::rawexpr::CStyleCast<$exprty>;
    pub type ArraySubscript = crate::common::rawexpr::ArraySubscript<$exprty>;
    pub type CompoundLiteral = crate::common::rawexpr::CompoundLiteral;

    mod fmtrawexpr {
      use super::*;
      use ::std::fmt::Display;
      impl Display for RawExpr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          match self {
            RawExpr::Constant(c) => <Constant as Display>::fmt(c, f),
            RawExpr::Unary(u) => <Unary as Display>::fmt(u, f),
            RawExpr::Binary(b) => <Binary as Display>::fmt(b, f),
            RawExpr::Ternary(t) => <Ternary as Display>::fmt(t, f),
            RawExpr::Call(call) => <Call as Display>::fmt(call, f),
            RawExpr::Empty => ::std::write!(f, "<noop>"),
            $(
              RawExpr::$extra(inner) => <$extra as Display>::fmt(inner, f),
            )*
            _ => ::std::todo!(),
          }
        }
      }
    }
    mod cvtrawexpr {
      use crate::interconvert;
      use super::*;
      interconvert!(Constant, RawExpr);
      interconvert!(Unary, RawExpr);
      interconvert!(Binary, RawExpr);
      interconvert!(Call, RawExpr);
      interconvert!(MemberAccess, RawExpr);
      interconvert!(Ternary, RawExpr);
      interconvert!(SizeOf, RawExpr);
      interconvert!(CStyleCast, RawExpr);
      interconvert!(ArraySubscript, RawExpr);
      interconvert!(CompoundLiteral, RawExpr);
      $(
        interconvert!($extra, RawExpr);
      )*
    }
  };
}

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
#[derive(Debug)]
pub struct Unary<ExprTy> {
  pub operator: Operator,
  pub expression: Box<ExprTy>,
}
#[derive(Debug)]
pub struct Binary<ExprTy> {
  pub operator: Operator,
  pub left: Box<ExprTy>,
  pub right: Box<ExprTy>,
}
#[derive(Debug)]
pub struct Call<ExprTy> {
  pub callee: Box<ExprTy>,
  pub arguments: Vec<ExprTy>,
}
#[derive(Debug)]
pub struct MemberAccess<ExprTy> {
  pub object: Box<ExprTy>,
  pub member: String,
}
#[derive(Debug)]
pub struct Ternary<ExprTy> {
  pub condition: Box<ExprTy>,
  pub then_expr: Box<ExprTy>,
  pub else_expr: Box<ExprTy>,
}
#[derive(Debug)]
pub enum SizeOf<ExprTy, TypeTy> {
  Type(TypeTy), // ignore for now
  Expression(Box<ExprTy>),
}

#[derive(Debug)]
pub struct CStyleCast<ExprTy> {
  pub target_type: QualifiedType,
  pub expression: Box<ExprTy>,
}
#[derive(Debug)]
pub struct ArraySubscript<ExprTy> {
  pub array: Box<ExprTy>,
  pub index: Box<ExprTy>,
}
#[derive(Debug)]
pub struct CompoundLiteral {
  pub target_type: QualifiedType,
  // pub initializer: Initializer,
}

impl<ExprTy> Unary<ExprTy> {
  pub fn from_operator(operator: Operator, expression: ExprTy) -> Option<Self> {
    match operator.unary() {
      true => Some(Self {
        operator,
        expression: Box::new(expression),
      }),
      false => None,
    }
  }

  pub fn new(operator: Operator, expression: ExprTy) -> Self {
    Self::from_operator(operator, expression).unwrap()
  }
}

impl<ExprTy> Binary<ExprTy> {
  pub fn from_operator(
    operator: Operator,
    left: ExprTy,
    right: ExprTy,
  ) -> Option<Self> {
    match operator.binary() {
      true => Some(Self {
        operator,
        left: Box::new(left),
        right: Box::new(right),
      }),
      false => None,
    }
  }

  pub fn new(operator: Operator, left: ExprTy, right: ExprTy) -> Self {
    Self::from_operator(operator, left, right).unwrap()
  }
}
impl<ExprTy> Ternary<ExprTy> {
  pub fn new(condition: ExprTy, then_expr: ExprTy, else_expr: ExprTy) -> Self {
    Self {
      condition: Box::new(condition),
      then_expr: Box::new(then_expr),
      else_expr: Box::new(else_expr),
    }
  }
}

impl<ExprTy> Call<ExprTy> {
  pub fn new(callee: ExprTy, arguments: Vec<ExprTy>) -> Self {
    Self {
      callee: Box::new(callee),
      arguments,
    }
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::{Binary, Call, Constant, Ternary, Unary};

  impl<ExprTy: Display> Display for Call<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}(", self.callee)?;
      for (i, arg) in self.arguments.iter().enumerate() {
        write!(f, "{}", arg)?;
        if i != self.arguments.len() - 1 {
          write!(f, ", ")?;
        }
      }
      write!(f, ")")
    }
  }

  impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Constant::Char(i) => write!(f, "{}", i),
        Constant::Short(i) => write!(f, "{}", i),
        Constant::Int(i) => write!(f, "{}", i),
        Constant::LongLong(i) => write!(f, "{}", i),
        Constant::UChar(u) => write!(f, "{}", u),
        Constant::UShort(u) => write!(f, "{}", u),
        Constant::UInt(u) => write!(f, "{}", u),
        Constant::ULongLong(u) => write!(f, "{}", u),
        Constant::Float(fl) => write!(f, "{}", fl),
        Constant::Double(fl) => write!(f, "{}", fl),
        Constant::Bool(b) => write!(f, "{}", b),
        Constant::String(s) => write!(f, "\"{}\"", s),
      }
    }
  }
  impl<ExprTy: Display> Display for Unary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {})", self.expression, self.operator)
    }
  }
  impl<ExprTy: Display> Display for Binary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }
  impl<ExprTy: Display> Display for Ternary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "({} ? {} : {})",
        self.condition, self.then_expr, self.else_expr
      )
    }
  }
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
    use crate::common::types::{Array, ArraySize, Primitive, Qualifiers};

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
