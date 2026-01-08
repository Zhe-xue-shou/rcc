use crate::{
  parser::declaration::{DeclSpecs, Declarator},
  type_alias_expr,
};

#[derive(Debug)]
pub enum Expression {
  Empty, // no-op for error recovery; for empty expr should use Option<Expression> instead
  Constant(Constant),
  Unary(Unary),
  Binary(Binary),
  Assignment(Assignment),
  Variable(Variable),
  Call(Call),
  MemberAccess(MemberAccess),
  Ternary(Ternary),
  SizeOf(SizeOf),
  Cast(Cast),                       // (int)x
  ArraySubscript(ArraySubscript),   // arr[i]
  CompoundLiteral(CompoundLiteral), // (struct Point){.x=1, .y=2}
}
type_alias_expr! {Expression, UnprocessedType, Variable}
impl Variable {
  pub fn new(name: String) -> Self {
    Self { name }
  }
}
#[derive(Debug)]
pub struct UnprocessedType {
  pub declspecs: DeclSpecs,
  pub declarator: Declarator,
}
impl UnprocessedType {
  pub fn new(declspecs: DeclSpecs, declarator: Declarator) -> Self {
    Self {
      declspecs,
      declarator,
    }
  }
}
#[derive(Debug)]
pub struct Variable {
  pub name: String,
}
mod fmt {
  use super::{Assignment, Binary, Call, Constant, Expression, Ternary, Unary, Variable};
  use ::std::fmt::Display;

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
  impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name)
    }
  }
}
