use ::rc_utils::interconvert;
use ::std::fmt::Debug;

use super::{
  Keyword::{self, *},
  Operator, SourceSpan,
};
use crate::types::Constant;

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
  Number(Constant),
  Identifier(String),
  String(String),
  Keyword(Keyword),
  Operator(Operator),
}

#[derive(Debug)]
pub struct Token {
  pub literal: Literal,
  pub location: SourceSpan,
}

impl Token {
  pub fn string(literal: String, location: SourceSpan) -> Self {
    Self {
      literal: Literal::String(literal),
      location,
    }
  }

  pub fn number(number: Constant, location: SourceSpan) -> Self {
    Self {
      literal: Literal::Number(number),
      location,
    }
  }

  pub fn identifier(identifier: String, location: SourceSpan) -> Self {
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

  pub fn to_owned_string(&self) -> String {
    match &self.literal {
      Literal::Identifier(str) | Literal::String(str) => str.clone(),
      Literal::Keyword(kw) => kw.to_string(),
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
impl Literal {
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

interconvert!(Keyword, Literal);
interconvert!(Operator, Literal);
interconvert!(Constant, Literal, Number);
interconvert!(String, Literal, Identifier);
// interconvert!(String, Literal, String); // this one conflicts with the above
mod cmp {
  use super::{Keyword, Literal, Operator};

  impl PartialEq<Literal> for Keyword {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      match other {
        Literal::Keyword(kw) => self == kw,
        _ => false,
      }
    }
  }
  impl PartialEq<Keyword> for Literal {
    #[inline]
    fn eq(&self, other: &Keyword) -> bool {
      match self {
        Literal::Keyword(kw) => kw == other,
        _ => false,
      }
    }
  }
  impl PartialEq<Operator> for Literal {
    #[inline]
    fn eq(&self, other: &Operator) -> bool {
      match self {
        Literal::Operator(op) => op == other,
        _ => false,
      }
    }
  }
  impl PartialEq<Literal> for Operator {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      match other {
        Literal::Operator(op) => self == op,
        _ => false,
      }
    }
  }
  impl PartialEq<Operator> for &Literal {
    #[inline]
    fn eq(&self, other: &Operator) -> bool {
      match self {
        Literal::Operator(op) => op == other,
        _ => false,
      }
    }
  }

  impl PartialEq<Keyword> for &Literal {
    #[inline]
    fn eq(&self, other: &Keyword) -> bool {
      match self {
        Literal::Keyword(kw) => kw == other,
        _ => false,
      }
    }
  }

  impl PartialEq<Literal> for &Literal {
    #[inline]
    fn eq(&self, other: &Literal) -> bool {
      PartialEq::eq(*self, other)
    }
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::{Literal, Token};

  impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "{} at loc {} {}",
        self.literal, self.location.start, self.location.end
      )
    }
  }

  impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Literal::Number(n) => write!(f, "Number({})", n),
        Literal::Identifier(id) => write!(f, "Identifier({})", id),
        Literal::String(s) => write!(f, "String({})", s),
        Literal::Keyword(kw) => write!(f, "Keyword({})", kw),
        Literal::Operator(op) => write!(f, "Operator({})", op),
      }
    }
  }
}
