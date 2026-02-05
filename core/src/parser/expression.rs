use ::rc_utils::interconvert;

use crate::{
  common::SourceSpan,
  parser::declaration::{DeclSpecs, Declarator},
  type_alias_expr,
};

#[derive(Debug)]
pub enum Expression {
  Empty(Empty), // no-op for error recovery; for empty expr should use Option<Expression> instead
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
  pub fn new(name: String, span: SourceSpan) -> Self {
    Self { name, span }
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
  pub span: SourceSpan,
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
  use ::rc_utils::static_dispatch;
  use ::std::fmt::Display;

  use super::*;

  impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      static_dispatch!(
        self.fmt(f),
        Empty Constant Unary Binary Variable Call Paren MemberAccess Ternary SizeOf CStyleCast ArraySubscript CompoundLiteral
      )
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
