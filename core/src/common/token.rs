use ::std::{fmt::Debug, path::PathBuf, rc::Rc};

use crate::{
  common::{keyword::Keyword, operator::Operator},
  types::Constant,
};

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
  Number(Constant),
  Identifier(String),
  String(String),
  Keyword(Keyword),
  Operator(Operator),
}

#[derive(Debug, Default)]
pub struct SourceLocation {
  pub file: Rc<PathBuf>,
  pub line_string: Rc<String>,
  pub line: u32,
  pub column: u32,
}
#[derive(Debug)]
pub struct Token {
  pub literal: Literal,
  pub location: SourceLocation,
}

impl Token {
  pub fn string(literal: String, location: SourceLocation) -> Self {
    Self {
      literal: Literal::String(literal),
      location,
    }
  }

  pub fn number(number: Constant, location: SourceLocation) -> Self {
    Self {
      literal: Literal::Number(number),
      location,
    }
  }

  pub fn identifier(identifier: String, location: SourceLocation) -> Self {
    Self {
      literal: Literal::Identifier(identifier),
      location,
    }
  }

  pub fn keyword(keyword: Keyword, location: SourceLocation) -> Self {
    Self {
      literal: Literal::Keyword(keyword),
      location,
    }
  }

  pub fn operator(operator: Operator, location: SourceLocation) -> Self {
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
    matches!(
      self,
      Keyword::Const | Keyword::Volatile | Keyword::Restrict | Keyword::Atomic
    )
  }

  pub fn is_storage_class(&self) -> bool {
    matches!(
      self,
      Keyword::Auto
        | Keyword::Register
        | Keyword::Static
        | Keyword::Extern
        | Keyword::Typedef
    )
  }

  pub fn is_function_specifier(&self) -> bool {
    matches!(self, Keyword::Inline | Keyword::Noreturn)
  }

  /// this isn't exhaustive, need to check typedefs in parser
  pub fn is_type_specifier(&self) -> bool {
    matches!(
      self,
      Keyword::Void
        | Keyword::Char
        | Keyword::Short
        | Keyword::Int
        | Keyword::Long
        | Keyword::Float
        | Keyword::Double
        | Keyword::Signed
        | Keyword::Unsigned
        | Keyword::Bool
        | Keyword::Struct
        | Keyword::Union
        | Keyword::Enum
    )
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::{Literal, Token};

  impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match &self.literal {
        Literal::Number(n) => write!(f, "Number({})", n),
        Literal::Identifier(id) => write!(f, "Identifier({})", id),
        Literal::String(s) => write!(f, "String({})", s),
        Literal::Keyword(kw) => write!(f, "Keyword({})", kw),
        Literal::Operator(op) => write!(f, "Operator({})", op),
      }
      .and_then(|_| {
        write!(
          f,
          " at line {}, column {}",
          self.location.line, self.location.column
        )
      })
    }
  }
}
