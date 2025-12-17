use crate::common::keyword::Keyword;
use crate::common::operator::Operator;
use std::fmt::{Debug, Display};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Literal {
  Number(String),
  Identifier(String),
  String(String),
  Keyword(Keyword),
  Operator(Operator),
}

#[derive(Debug, Clone, Copy)]
pub struct SourceLocation {
  pub line: u32,
  pub column: u32,
}

pub struct Token {
  pub literal: Literal,
  pub location: SourceLocation,
}
impl Debug for Token {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

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

impl Token {
  pub fn string(literal: String, location: SourceLocation) -> Self {
    Self {
      literal: Literal::String(literal),
      location,
    }
  }
  pub fn number(number: String, location: SourceLocation) -> Self {
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
