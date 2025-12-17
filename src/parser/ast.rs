use ::std::fmt::{Debug, Display};

use crate::common::keyword::Keyword;
use crate::parser::statement::{Statement, VarDef};
use crate::parser::types::{Primitive, Type};

pub struct Program {
  pub declarations: Vec<Declaration>,
}
pub enum Declaration {
  Function(FunctionDef),
  Variable(VarDef),
}

pub struct FunctionDef {
  name: String,
  parameters: Vec<(String, Type)>,
  body: Block,
  return_type: Type,
}
impl Display for FunctionDef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Function {}(", self.name)?;
    for (i, (param_name, param_type)) in self.parameters.iter().enumerate() {
      write!(f, "{}: {}", param_name, param_type)?;
      if i != self.parameters.len() - 1 {
        write!(f, ", ")?;
      }
    }
    write!(f, ") -> {:?} ", self.return_type)?;
    write!(f, "{{\n")?;
    for stmt in &self.body.statements {
      write!(f, "  {:?}\n", stmt)?;
    }
    write!(f, "}}")
  }
}
impl Debug for FunctionDef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

pub struct Block {
  pub statements: Vec<Statement>,
}

impl Display for Block {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{{\n")?;
    for stmt in &self.statements {
      write!(f, "  {:?}\n", stmt)?;
    }
    write!(f, "}}")
  }
}

impl Debug for Block {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Block {
  pub fn new() -> Self {
    Self {
      statements: Vec::new(),
    }
  }
}

impl Program {
  pub fn new() -> Self {
    Self {
      declarations: Vec::new(),
    }
  }
}

impl FunctionDef {
  pub fn new(
    name: String,
    parameters: Vec<(String, Type)>,
    body: Block,
    return_type: Type,
  ) -> Self {
    Self {
      name,
      parameters,
      body,
      return_type,
    }
  }
}

impl Keyword {
  pub fn to_type(&self) -> Option<Primitive> {
    Primitive::maybe_new(self.to_string())
  }
}
