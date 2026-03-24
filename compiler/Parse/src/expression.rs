use ::rcc_ast::type_alias_expr;
use ::rcc_shared::SourceSpan;
use ::rcc_utils::StrRef;

use crate::declaration::{DeclSpecs, Declarator};

#[derive(Debug)]
pub enum Expression<'c> {
  Empty(Empty), // no-op for error recovery; for empty expr should use Option<Expression> instead
  Constant(Constant<'c>),
  Unary(Unary<'c>),
  Binary(Binary<'c>),
  Variable(Variable<'c>),
  Call(Call<'c>),
  Paren(Paren<'c>),
  MemberAccess(MemberAccess<'c>),
  Ternary(Ternary<'c>),
  SizeOf(SizeOf<'c>),
  CStyleCast(CStyleCast<'c>),         // (int)x
  ArraySubscript(ArraySubscript<'c>), // arr[i]
  CompoundLiteral(CompoundLiteral),   // (struct Point){.x=1, .y=2}
}
type_alias_expr! {Expression<'c> , UnprocessedType<'c>, Variable<'c> #[derive(Debug)]}
::rcc_utils::interconvert!(Variable, Expression, 'c);
::rcc_utils::interconvert!(Constant, Expression,'c);
::rcc_utils::interconvert!(Unary, Expression, 'c);
::rcc_utils::interconvert!(Binary, Expression, 'c);
::rcc_utils::interconvert!(Call, Expression, 'c);
::rcc_utils::interconvert!(Paren, Expression, 'c);
::rcc_utils::interconvert!(MemberAccess, Expression, 'c);
::rcc_utils::interconvert!(Ternary, Expression, 'c);
::rcc_utils::interconvert!(SizeOf, Expression, 'c);
::rcc_utils::interconvert!(CStyleCast, Expression, 'c);
::rcc_utils::interconvert!(ArraySubscript, Expression, 'c);
::rcc_utils::interconvert!(CompoundLiteral, Expression<'c>);
impl<'c> ::std::default::Default for Expression<'c> {
  #[inline(always)]
  fn default() -> Self {
    Expression::Empty(Empty::default())
  }
}

impl<'c> Variable<'c> {
  pub fn new(name: StrRef<'c>, span: SourceSpan) -> Self {
    Self { name, span }
  }
}
#[derive(Debug)]
pub struct UnprocessedType<'c> {
  pub declspecs: DeclSpecs<'c>,
  pub declarator: Declarator<'c>,
}
impl<'c> UnprocessedType<'c> {
  pub fn new(declspecs: DeclSpecs<'c>, declarator: Declarator<'c>) -> Self {
    Self {
      declspecs,
      declarator,
    }
  }
}
#[derive(Debug)]
pub struct Variable<'c> {
  pub name: StrRef<'c>,
  pub span: SourceSpan,
}

mod fmt {
  use ::std::fmt::Display;

  use super::*;

  impl Display for Expression<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      ::rcc_utils::static_dispatch!(
        self,
        |variant| variant.fmt(f) =>
        Empty Constant Unary Binary Variable Call Paren MemberAccess Ternary SizeOf CStyleCast ArraySubscript CompoundLiteral
      )
    }
  }
  impl Display for Variable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name)
    }
  }
  impl Display for UnprocessedType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{} {}", self.declspecs, self.declarator)
    }
  }
}
