use crate::common::operator::Operator;
pub enum Expression {
  Constant(Constant),
  Unary(Unary),
  Binary(Binary),
  Assignment(Assignment),
  Variable(Variable),
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
  pub(crate) operator: Operator,
  // This pattern is ubiquitous in Rust AST libraries -- Box is literally everywhere in recursive data structures.
  pub(crate) expression: Box<Expression>,
}
pub struct Binary {
  pub(crate) operator: Operator,
  pub(crate) left: Box<Expression>,
  pub(crate) right: Box<Expression>,
}
pub struct Variable {
  pub(crate) name: String,
}
pub struct Assignment {
  pub(crate) left: Box<Expression>,
  pub(crate) right: Box<Expression>,
}

impl Constant {
  pub fn from_str(str: &String) -> Self {
    let int32 = str.clone().parse::<i32>().unwrap();
    Self::Int32(int32)
  }
}
impl Variable{
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
