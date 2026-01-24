use ::rc_utils::interconvert;

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
  Variable(Variable),
  Call(Call),
  Paren(Paren),
  MemberAccess(MemberAccess),
  Ternary(Ternary),
  SizeOf(SizeOf),
  CStyleCast(CStyleCast),           // (int)x
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
interconvert!(Variable, Expression);
interconvert!(Constant, Expression);
interconvert!(Unary, Expression);
interconvert!(Binary, Expression);
interconvert!(Call, Expression);
interconvert!(Paren, Expression);
interconvert!(MemberAccess, Expression);
interconvert!(Ternary, Expression);
interconvert!(SizeOf, Expression);
interconvert!(CStyleCast, Expression);
interconvert!(ArraySubscript, Expression);
interconvert!(CompoundLiteral, Expression);

mod fmt {
  use ::std::fmt::Display;

  use super::{
    Binary, Call, Constant, Expression, Expression::*, SizeOf, Ternary, Unary,
    UnprocessedType, Variable,
  };
  use crate::parser::expression::Paren;

  impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Constant(c) => <Constant as Display>::fmt(c, f),
        Unary(u) => <Unary as Display>::fmt(u, f),
        Binary(b) => <Binary as Display>::fmt(b, f),
        Variable(v) => <Variable as Display>::fmt(v, f),
        Ternary(t) => <Ternary as Display>::fmt(t, f),
        Call(call) => <Call as Display>::fmt(call, f),
        SizeOf(s) => <SizeOf as Display>::fmt(s, f),
        Empty => write!(f, "<noop>"),
        Paren(p) => <Paren as Display>::fmt(p, f),
        _ => todo!("{:#?}", self),
      }
    }
  }
  impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name)
    }
  }
  impl Display for UnprocessedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{} {}", self.declspecs, self.declarator)
    }
  }
}
