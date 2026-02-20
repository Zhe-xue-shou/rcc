use ::rcc_utils::{SmallString, ensure_is_pod, interconvert};
use ::std::{fmt::Debug, str::FromStr};

use super::{
  Keyword::{self, *},
  Operator, SourceSpan, StrRef,
};
/// strictly speaking this isn't counted as cyclic dependency,
/// the [`Constant`] type looks similiar so used in here too.
use crate::types::Constant;

#[derive(Debug, PartialEq, Clone)]
pub enum Literal<'context> {
  Number(Constant<'context>),
  Identifier(StrRef<'context>),
  String(StrRef<'context>),
  Keyword(Keyword),
  Operator(Operator),
}

ensure_is_pod!(Literal);

#[derive(Debug)]
pub struct Token<'context> {
  pub literal: Literal<'context>,
  pub location: SourceSpan,
}

ensure_is_pod!(Token);

impl<'context> Token<'context> {
  pub fn character(character: char, location: SourceSpan) -> Self {
    Self {
      literal: Literal::Number(Constant::Character(character)),
      location,
    }
  }

  pub fn string(literal: StrRef<'context>, location: SourceSpan) -> Self {
    Self {
      literal: Literal::String(literal),
      location,
    }
  }

  pub fn number(number: Constant<'context>, location: SourceSpan) -> Self {
    Self {
      literal: Literal::Number(number),
      location,
    }
  }

  pub fn identifier(
    identifier: StrRef<'context>,
    location: SourceSpan,
  ) -> Self {
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

  pub fn to_owned_string(&self) -> SmallString {
    match &self.literal {
      Literal::Identifier(str) | Literal::String(str) =>
        SmallString::from_str(str).expect("never fails"),
      Literal::Keyword(kw) => kw.to_string().into(),
      _ => panic!("should not call this: {:?}", self.literal),
    }
  }

  /// transform `[tab:lex.diagraph]` alternative tokens into their operator equivalents
  pub fn transform_alternative(self) -> Self {
    match self.literal {
      Literal::Keyword(ref keyword) => match keyword {
        And => Self::operator(Operator::And, self.location),
        Or => Self::operator(Operator::Or, self.location),
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
impl<'context> Literal<'context> {
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

interconvert!(Keyword, Literal<'context>);
interconvert!(Operator, Literal<'context>);
interconvert!(Constant, Literal,'context, Number);
// interconvert!(SmallString, Literal, Identifier);
// interconvert!(String, Literal, String); // this one conflicts with the above
mod cmp {
  use super::{Keyword, Literal, Operator};

  impl<'context> PartialEq<Literal<'context>> for Keyword {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      match other {
        Literal::Keyword(kw) => self == kw,
        _ => false,
      }
    }
  }
  impl<'context> PartialEq<Keyword> for Literal<'context> {
    #[inline]
    fn eq(&self, other: &Keyword) -> bool {
      match self {
        Literal::Keyword(kw) => kw == other,
        _ => false,
      }
    }
  }
  impl<'context> PartialEq<Operator> for Literal<'context> {
    #[inline]
    fn eq(&self, other: &Operator) -> bool {
      match self {
        Literal::Operator(op) => op == other,
        _ => false,
      }
    }
  }
  impl<'context> PartialEq<Literal<'context>> for Operator {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      match other {
        Literal::Operator(op) => self == op,
        _ => false,
      }
    }
  }
  impl<'context> PartialEq<Operator> for &Literal<'context> {
    #[inline]
    fn eq(&self, other: &Operator) -> bool {
      match self {
        Literal::Operator(op) => op == other,
        _ => false,
      }
    }
  }

  impl<'context> PartialEq<Keyword> for &Literal<'context> {
    #[inline]
    fn eq(&self, other: &Keyword) -> bool {
      match self {
        Literal::Keyword(kw) => kw == other,
        _ => false,
      }
    }
  }

  impl<'context> PartialEq<Literal<'context>> for &Literal<'context> {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      PartialEq::eq(*self, other)
    }
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::{Literal, Token};

  impl<'context> Display for Token<'context> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "{} at loc [{} {})",
        self.literal, self.location.start, self.location.end
      )
    }
  }

  impl<'context> Display for Literal<'context> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      ::rcc_utils::static_dispatch!(self.fmt(f), Operator Number String Identifier Keyword)
    }
  }
}
