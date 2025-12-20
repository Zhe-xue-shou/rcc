use crate::{
  common::{operator::Operator, types::QualifiedType},
  parser::declaration::Initializer,
};
pub enum Expression {
  Empty, // no-op and also for error recovery
  Constant(Constant),
  Unary(Unary),
  Binary(Binary),
  Assignment(Assignment),
  Variable(Variable),
  Call(Call),
  MemberAccess(MemberAccess),
  Ternary(Ternary),
  SizeOf(SizeOf),
  Identifier(Identifier),
  Cast(Cast),                       // (int)x
  ArraySubscript(ArraySubscript),   // arr[i]
  CompoundLiteral(CompoundLiteral), // (struct Point){.x=1, .y=2}
}
pub enum Constant {
  Int8(i8),
  Int16(i16),
  Int32(i32),
  Int64(i64),
  Uint8(u8),
  Uint16(u16),
  Uint32(u32),
  Uint64(u64),
  Float32(f32),
  Float64(f64),
  Bool(bool),
  String(String),
}
pub struct Unary {
  pub operator: Operator,
  // This pattern is ubiquitous in Rust AST libraries -- Box is literally everywhere in recursive data structures.
  pub expression: Box<Expression>,
}
pub struct Binary {
  pub operator: Operator,
  pub left: Box<Expression>,
  pub right: Box<Expression>,
}
pub struct Variable {
  pub name: String,
}
pub struct Assignment {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
}

pub struct Call {
  pub callee: Box<Expression>,
  pub arguments: Vec<Expression>,
}
pub struct MemberAccess {
  pub object: Box<Expression>,
  pub member: String,
}

pub struct Ternary {
  pub condition: Box<Expression>,
  pub if_branch: Box<Expression>,
  pub else_branch: Box<Expression>,
}

pub enum SizeOf {
  // Type(String), // ignore for now
  Expression(Box<Expression>),
}

pub struct Identifier {
  pub name: String,
}
pub struct Cast {
  pub target_type: QualifiedType,
  pub expression: Box<Expression>,
}

pub struct ArraySubscript {
  pub array: Box<Expression>,
  pub index: Box<Expression>,
}

pub struct CompoundLiteral {
  pub target_type: QualifiedType,
  pub initializer: Initializer,
}

impl Constant {
  pub fn from_str(str: &String) -> Self {
    let int32 = str.clone().parse::<i32>().unwrap();
    Self::Int32(int32)
  }
}
impl Variable {
  pub fn new(name: String) -> Self {
    Self { name }
  }
}
impl Unary {
  pub fn from_operator(operator: Operator, expression: Expression) -> Option<Self> {
    match operator.unary() {
      true => Some(Self {
        operator,
        expression: Box::new(expression),
      }),
      false => None,
    }
  }
  pub fn new(operator: Operator, expression: Expression) -> Self {
    Self::from_operator(operator, expression).unwrap()
  }
}

impl Binary {
  pub fn from_operator(operator: Operator, left: Expression, right: Expression) -> Option<Self> {
    match operator.binary() {
      true => Some(Self {
        operator,
        left: Box::new(left),
        right: Box::new(right),
      }),
      false => None,
    }
  }
  pub fn new(operator: Operator, left: Expression, right: Expression) -> Self {
    Self::from_operator(operator, left, right).unwrap()
  }
}
impl Ternary {
  pub fn new(condition: Expression, if_branch: Expression, else_branch: Expression) -> Self {
    Self {
      condition: Box::new(condition),
      if_branch: Box::new(if_branch),
      else_branch: Box::new(else_branch),
    }
  }
}

impl Call {
  pub fn new(callee: Expression, arguments: Vec<Expression>) -> Self {
    Self {
      callee: Box::new(callee),
      arguments,
    }
  }
}
mod fmt {
  use crate::parser::expression::{
    Assignment, Binary, Call, Constant, Expression, Ternary, Unary, Variable,
  };
  use ::std::fmt::{Debug, Display};
  impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name)
    }
  }

  impl Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} =)", self.left, self.right)
    }
  }

  impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Expression::Constant(c) => <Constant as Display>::fmt(c, f),
        Expression::Unary(u) => <Unary as Display>::fmt(u, f),
        Expression::Binary(b) => <Binary as Display>::fmt(b, f),
        Expression::Assignment(a) => <Assignment as Display>::fmt(a, f),
        Expression::Variable(v) => <Variable as Display>::fmt(v, f),
        Expression::Ternary(t) => <Ternary as Display>::fmt(t, f),
        Expression::Call(call) => <Call as Display>::fmt(call, f),
        Expression::Empty => write!(f, "<noop>"),
        _ => todo!(),
      }
    }
  }

  impl Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for Call {
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

  impl Debug for Call {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Constant::Int8(i) => write!(f, "{}", i),
        Constant::Int16(i) => write!(f, "{}", i),
        Constant::Int32(i) => write!(f, "{}", i),
        Constant::Int64(i) => write!(f, "{}", i),
        Constant::Uint8(u) => write!(f, "{}", u),
        Constant::Uint16(u) => write!(f, "{}", u),
        Constant::Uint32(u) => write!(f, "{}", u),
        Constant::Uint64(u) => write!(f, "{}", u),
        Constant::Float32(fl) => write!(f, "{}", fl),
        Constant::Float64(fl) => write!(f, "{}", fl),
        Constant::Bool(b) => write!(f, "{}", b),
        Constant::String(s) => write!(f, "\"{}\"", s),
      }
    }
  }

  impl Debug for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for Unary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {})", self.expression, self.operator)
    }
  }

  impl Debug for Unary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for Binary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }
  impl Display for Ternary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "({} ? {} : {})",
        self.condition, self.if_branch, self.else_branch
      )
    }
  }

  impl Debug for Binary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
}
