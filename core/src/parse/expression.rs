use crate::{
  blueprints::type_alias_expr,
  common::{SourceSpan, StrRef},
  parse::declaration::{DeclSpecs, Declarator},
};

#[derive(Debug)]
pub enum Expression<'context> {
  Empty(Empty), // no-op for error recovery; for empty expr should use Option<Expression> instead
  Constant(Constant<'context>),
  Unary(Unary<'context>),
  Binary(Binary<'context>),
  Variable(Variable<'context>),
  Call(Call<'context>),
  Paren(Paren<'context>),
  MemberAccess(MemberAccess<'context>),
  Ternary(Ternary<'context>),
  SizeOf(SizeOf<'context>),
  CStyleCast(CStyleCast<'context>), // (int)x
  ArraySubscript(ArraySubscript<'context>), // arr[i]
  CompoundLiteral(CompoundLiteral), // (struct Point){.x=1, .y=2}
}
type_alias_expr! {Expression<'context> , UnprocessedType<'context>, Variable<'context>}
::rcc_utils::interconvert!(Variable, Expression, 'context);
::rcc_utils::interconvert!(Constant, Expression,'context);
::rcc_utils::interconvert!(Unary, Expression, 'context);
::rcc_utils::interconvert!(Binary, Expression, 'context);
::rcc_utils::interconvert!(Call, Expression, 'context);
::rcc_utils::interconvert!(Paren, Expression, 'context);
::rcc_utils::interconvert!(MemberAccess, Expression, 'context);
::rcc_utils::interconvert!(Ternary, Expression, 'context);
::rcc_utils::interconvert!(SizeOf, Expression, 'context);
::rcc_utils::interconvert!(CStyleCast, Expression, 'context);
::rcc_utils::interconvert!(ArraySubscript, Expression, 'context);
::rcc_utils::interconvert!(CompoundLiteral, Expression<'context>);
impl<'context> ::std::default::Default for Expression<'context> {
  #[inline(always)]
  fn default() -> Self {
    Expression::Empty(Empty::default())
  }
}

impl<'context> Variable<'context> {
  pub fn new(name: StrRef<'context>, span: SourceSpan) -> Self {
    Self { name, span }
  }
}
#[derive(Debug)]
pub struct UnprocessedType<'context> {
  pub declspecs: DeclSpecs<'context>,
  pub declarator: Declarator<'context>,
}
impl<'context> UnprocessedType<'context> {
  pub fn new(
    declspecs: DeclSpecs<'context>,
    declarator: Declarator<'context>,
  ) -> Self {
    Self {
      declspecs,
      declarator,
    }
  }
}
#[derive(Debug)]
pub struct Variable<'context> {
  pub name: StrRef<'context>,
  pub span: SourceSpan,
}

mod fmt {
  use ::rcc_utils::static_dispatch;
  use ::std::fmt::Display;

  use super::*;

  impl Display for Expression<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      static_dispatch!(
        self.fmt(f),
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
