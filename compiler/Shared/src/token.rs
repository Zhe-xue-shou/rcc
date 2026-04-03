use ::rcc_utils::{StrRef, ensure_is_pod, interconvert};

use super::{
  Keyword::{self, *},
  Number, Operator, SourceSpan,
};
/// strictly speaking this isn't counted as cyclic dependency,
/// the [`Constant`] type looks similiar so used in here too.

#[derive(Debug, PartialEq, Clone)]
pub enum Literal<'c> {
  Number(Number),
  Identifier(StrRef<'c>),
  String(StrRef<'c>),
  Keyword(Keyword),
  Operator(Operator),
}

ensure_is_pod!(Literal);

#[derive(Debug)]
pub struct Token<'c> {
  pub literal: Literal<'c>,
  pub location: SourceSpan,
}

ensure_is_pod!(Token);

impl<'c> Token<'c> {
  pub fn character(character: char, location: SourceSpan) -> Self {
    Self {
      literal: Literal::Number(Number::Integral((character as i32).into())),
      location,
    }
  }

  pub fn string(literal: StrRef<'c>, location: SourceSpan) -> Self {
    Self {
      literal: Literal::String(literal),
      location,
    }
  }

  pub fn number(number: Number, location: SourceSpan) -> Self {
    Self {
      literal: Literal::Number(number),
      location,
    }
  }

  pub fn identifier(identifier: StrRef<'c>, location: SourceSpan) -> Self {
    Self {
      literal: Literal::Identifier(identifier),
      location,
    }
  }

  pub fn keyword(keyword: Keyword, location: SourceSpan) -> Self {
    Self {
      literal: Literal::Keyword(keyword),
      location,
    }
  }

  pub fn operator(operator: Operator, location: SourceSpan) -> Self {
    Self {
      literal: Literal::Operator(operator),
      location,
    }
  }

  /// transform `[tab:lex.diagraph]` alternative tokens into their operator equivalents
  pub fn transform_alternative(self) -> Self {
    match self.literal {
      Literal::Keyword(ref keyword) => match keyword {
        And => Self::operator(Operator::LogicalAnd, self.location),
        Or => Self::operator(Operator::LogicalOr, self.location),
        Not => Self::operator(Operator::Not, self.location),
        Xor => Self::operator(Operator::Caret, self.location),
        Bitand => Self::operator(Operator::Ampersand, self.location),
        Bitor => Self::operator(Operator::Pipe, self.location),
        Compl => Self::operator(Operator::Tilde, self.location),
        _ => self,
      },
      _ => self,
    }
  }
}
impl<'c> Literal<'c> {
  pub fn is_qualifier(&self) -> bool {
    match self {
      Literal::Keyword(kw) => kw.is_qualifier(),
      _ => false,
    }
  }

  pub fn is_storage_class(&self) -> bool {
    match self {
      Literal::Keyword(kw) => kw.is_storage_class(),
      _ => false,
    }
  }

  pub fn is_function_specifier(&self) -> bool {
    match self {
      Literal::Keyword(kw) => kw.is_function_specifier(),
      _ => false,
    }
  }
}

impl Keyword {
  pub fn is_qualifier(&self) -> bool {
    matches!(self, Const | Volatile | Restrict | Atomic)
  }

  pub fn is_storage_class(&self) -> bool {
    matches!(self, Auto | Register | Static | Extern | Typedef)
  }

  pub fn is_function_specifier(&self) -> bool {
    matches!(self, Inline | Noreturn)
  }

  /// this isn't exhaustive, need to check typedefs in parser
  pub fn is_type_specifier(&self) -> bool {
    matches!(
      self,
      Void
        | Char
        | Short
        | Int
        | Long
        | Float
        | Double
        | Signed
        | Unsigned
        | Bool
        | Struct
        | Union
        | Enum
    )
  }
}

interconvert!(Keyword, Literal<'c>);
interconvert!(Operator, Literal<'c>);
interconvert!(Number, Literal<'c>);

mod cmp {
  use super::{Keyword, Literal, Operator};

  impl<'c> PartialEq<Literal<'c>> for Keyword {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      match other {
        Literal::Keyword(kw) => self == kw,
        _ => false,
      }
    }
  }
  impl<'c> PartialEq<Keyword> for Literal<'c> {
    #[inline]
    fn eq(&self, other: &Keyword) -> bool {
      match self {
        Literal::Keyword(kw) => kw == other,
        _ => false,
      }
    }
  }
  impl<'c> PartialEq<Operator> for Literal<'c> {
    #[inline]
    fn eq(&self, other: &Operator) -> bool {
      match self {
        Literal::Operator(op) => op == other,
        _ => false,
      }
    }
  }
  impl<'c> PartialEq<Literal<'c>> for Operator {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      match other {
        Literal::Operator(op) => self == op,
        _ => false,
      }
    }
  }
  impl<'c> PartialEq<Operator> for &Literal<'c> {
    #[inline]
    fn eq(&self, other: &Operator) -> bool {
      match self {
        Literal::Operator(op) => op == other,
        _ => false,
      }
    }
  }

  impl<'c> PartialEq<Keyword> for &Literal<'c> {
    #[inline]
    fn eq(&self, other: &Keyword) -> bool {
      match self {
        Literal::Keyword(kw) => kw == other,
        _ => false,
      }
    }
  }

  impl<'c> PartialEq<Literal<'c>> for &Literal<'c> {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      PartialEq::eq(*self, other)
    }
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::{Literal, Number, Token};

  impl<'c> Display for Token<'c> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "{} at loc [{} {})",
        self.literal, self.location.start, self.location.end
      )
    }
  }

  impl<'c> Display for Literal<'c> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      ::rcc_utils::static_dispatch!(self, |variant| variant.fmt(f) => Operator Number String Identifier Keyword)
    }
  }

  impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      ::rcc_utils::static_dispatch!(self, |variant| variant.fmt(f) => Integral Floating)
    }
  }
}
