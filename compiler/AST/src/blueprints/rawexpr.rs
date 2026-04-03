//! likely a sophisticated version of the Two-Level Types described in
//! [this article](https://blog.ezyang.com/2013/05/the-ast-typing-problem/),
//! I probably used the Parametric Polymorphism to "tie the knot" of recursion.

use ::rcc_shared::{Operator, SourceSpan};
use ::rcc_utils::{IntoWith, StrRef};

use crate::Constant;

#[macro_export]
macro_rules! type_alias_expr {
  ($exprty:ty, $typety:ty, $($extra:ident)* #[derive($($derives:ident),* $(,)*)]) => {
    type_alias_expr!(@impl $exprty, $typety, $($extra)*: [] #[derive($($derives),*)]);
  };
  ($exprty:ty, $typety:ty, $($extra:ident $(<$lts:lifetime>)*)* #[derive($($derives:ident),* $(,)*)]) =>{
    type_alias_expr!(@impl $exprty, $typety, $($extra $(<$lts>)*)* #[derive($($derives),*)]);
  };
  (@impl $exprty:ty, $typety:ty, $($extra:ident $(<$lts:lifetime>)?)* #[derive($($derives:ident),* $(,)*)]) => {
    #[derive($($derives),*)]
    pub enum RawExpr<'c> {
      // no-op for error recovery; for empty expr should use Option<ExprTy> instead
      Empty(Empty),
      Constant(Constant<'c>),
      Unary(Unary<'c>),
      Binary(Binary<'c>),
      Call(Call<'c>),
      Paren(Paren<'c>),
      MemberAccess(MemberAccess<'c>),
      Ternary(Ternary<'c>),
      SizeOf(SizeOf<'c>),
      CStyleCast(CStyleCast<'c>),
      ArraySubscript(ArraySubscript<'c>),
      CompoundLiteral(CompoundLiteral),
      $(
        // Generate a variant for each extra type
        $extra($extra $(<$lts>)?),
      )*
    }
    /// exists to avoid name clash with `Constant` in this module; this is a design mistake
    pub type Empty =  $crate::blueprints::Placeholder;
    pub type ConstantLiteral<'c> = $crate::Constant<'c>;
    /// type or expression
    pub type SizeOfKind<'c> = $crate::blueprints::RawSizeOfKind<$exprty, $typety>;
    pub type Constant<'c> = $crate::blueprints::RawConstant<'c>;
    pub type Unary<'c> = $crate::blueprints::RawUnary<$exprty>;
    pub type Binary<'c> = $crate::blueprints::RawBinary<$exprty>;
    pub type Call<'c> = $crate::blueprints::RawCall<$exprty>;
    pub type Paren<'c> = $crate::blueprints::RawParen<$exprty>;
    pub type MemberAccess<'c> = $crate::blueprints::RawMemberAccess<'c, $exprty>;
    pub type Ternary<'c> = $crate::blueprints::RawTernary<$exprty>;
    pub type SizeOf<'c> = $crate::blueprints::RawSizeOf<$exprty, $typety>;
    pub type CStyleCast<'c> = $crate::blueprints::RawCStyleCast<$exprty, $typety>;
    pub type ArraySubscript<'c> = $crate::blueprints::RawArraySubscript<$exprty>;
    pub type CompoundLiteral = $crate::blueprints::RawCompoundLiteral;

    mod fmtrawexpr {
      use super::*;
      impl<'c> ::std::fmt::Display for RawExpr<'c> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
          ::rcc_utils::static_dispatch!(
            self,
            |variant| variant.fmt(f) =>
            Empty Constant Unary Binary Variable Call Paren MemberAccess Ternary SizeOf CStyleCast ArraySubscript CompoundLiteral
            $($extra)*
          )
        }
      }
    }
    mod cvtrawexpr {
      use super::*;
      use ::rcc_shared::SourceSpan;

      ::rcc_utils::interconvert!(Empty, RawExpr<'c>);
      ::rcc_utils::interconvert!(Constant, RawExpr,'c);
      ::rcc_utils::interconvert!(Unary, RawExpr, 'c);
      ::rcc_utils::interconvert!(Binary, RawExpr, 'c);
      ::rcc_utils::interconvert!(Call, RawExpr, 'c);
      ::rcc_utils::interconvert!(Paren, RawExpr, 'c);
      ::rcc_utils::interconvert!(MemberAccess, RawExpr, 'c);
      ::rcc_utils::interconvert!(Ternary, RawExpr, 'c);
      ::rcc_utils::interconvert!(SizeOf, RawExpr, 'c);
      ::rcc_utils::interconvert!(CStyleCast, RawExpr, 'c);
      ::rcc_utils::interconvert!(ArraySubscript, RawExpr, 'c);
      ::rcc_utils::interconvert!(CompoundLiteral, RawExpr<'c>);
      $(
        ::rcc_utils::interconvert!($extra, RawExpr, $($lts)?);
      )*

      ::rcc_utils::make_trio_for!(Empty, RawExpr<'c>);
      ::rcc_utils::make_trio_for!(Constant, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(Unary, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(Binary, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(Call, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(Paren, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(MemberAccess, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(Ternary, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(SizeOf, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(CStyleCast, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(ArraySubscript, RawExpr, 'c);
      ::rcc_utils::make_trio_for!(CompoundLiteral, RawExpr<'c>);
      $(
        ::rcc_utils::make_trio_for!($extra, RawExpr, $($lts)?);
      )*

      impl<'c> ::rcc_utils::IntoWith<SourceSpan, RawExpr<'c>> for ConstantLiteral<'c> {
        fn into_with(self, span: SourceSpan) -> RawExpr<'c> {
          RawExpr::Constant(self.into_with(span))
        }
      }

      impl<'c> ::rcc_utils::IntoWith<SourceSpan, RawExpr<'c>> for SizeOfKind<'c> {
        fn into_with(self, span: SourceSpan) -> RawExpr<'c> {
          RawExpr::SizeOf(self.into_with(span))
        }
      }
    }

    mod getspan {
      use super::*;
      use ::rcc_shared::SourceSpan;
      impl<'c> RawExpr<'c> {
        pub fn span(&self) -> SourceSpan {
          match self {
            RawExpr::Empty(_) => SourceSpan::default(),
            RawExpr::Constant(c) => c.span,
            RawExpr::Unary(u) => u.span,
            RawExpr::Binary(b) => b.span,
            RawExpr::Call(call) => call.span,
            RawExpr::Paren(p) => p.span,
            RawExpr::MemberAccess(ma) => ma.span,
            RawExpr::Ternary(t) => t.span,
            RawExpr::SizeOf(sizeof) => sizeof.span,
            RawExpr::CStyleCast(cast) => cast.span,
            RawExpr::ArraySubscript(arrsub) => arrsub.span,
            RawExpr::CompoundLiteral(cl) => cl.span,
            $(
              RawExpr::$extra(inner) => inner.span,
            )*
          }
        }
      }
    }

    ::rcc_utils::static_assert!(
      ::std::mem::size_of::<RawExpr>() <= 80, // compoundassign has maximun size now
      "RawExpr size exceeds 64 bytes",
    );

  };
}

#[derive(Debug, Clone)]
pub struct RawConstant<'c> {
  pub inner: Constant<'c>,
  pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct RawUnary<ExprTy> {
  pub operator: Operator,
  pub operand: Box<ExprTy>,
  pub kind: super::UnaryKind,
  pub span: SourceSpan,
}
#[derive(Debug, Clone)]
pub struct RawBinary<ExprTy> {
  pub operator: Operator,
  pub left: Box<ExprTy>,
  pub right: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug, Clone)]
pub struct RawCall<ExprTy> {
  pub callee: Box<ExprTy>,
  pub arguments: Vec<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug, Clone)]
pub struct RawParen<ExprTy> {
  pub expr: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug, Clone)]
pub struct RawMemberAccess<'c, ExprTy> {
  pub object: Box<ExprTy>,
  pub member: StrRef<'c>,
  pub span: SourceSpan,
}
#[derive(Debug, Clone)]
pub struct RawTernary<ExprTy> {
  pub condition: Box<ExprTy>,
  pub then_expr: Option<Box<ExprTy>>,
  pub else_expr: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug, Clone)]
pub enum RawSizeOfKind<ExprTy, TypeTy> {
  Type(Box<TypeTy>),
  Expression(Box<ExprTy>),
}

#[derive(Debug, Clone)]
pub struct RawSizeOf<ExprTy, TypeTy> {
  pub sizeof: RawSizeOfKind<ExprTy, TypeTy>,
  pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct RawCStyleCast<ExprTy, TypeTy> {
  pub target_type: Box<TypeTy>,
  pub expr: Box<ExprTy>,
  pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct RawArraySubscript<ExprTy> {
  pub array: Box<ExprTy>,
  pub index: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug, Clone)]
pub struct RawCompoundLiteral {
  // pub target_type: Box<QualifiedType>,
  // pub initializer: Initializer,
  pub span: SourceSpan,
}

impl<'c> RawConstant<'c> {
  pub fn new(constant: impl Into<Constant<'c>>, span: SourceSpan) -> Self {
    Self {
      inner: constant.into(),
      span,
    }
  }
}

impl<'c> ::std::ops::Deref for RawConstant<'c> {
  type Target = Constant<'c>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<'c> IntoWith<SourceSpan, RawConstant<'c>> for Constant<'c> {
  fn into_with(self, span: SourceSpan) -> RawConstant<'c> {
    RawConstant::new(self, span)
  }
}

impl<'c> ::std::ops::Add<SourceSpan> for Constant<'c> {
  type Output = RawConstant<'c>;

  #[inline]
  fn add(self, rhs: SourceSpan) -> Self::Output {
    self.into_with(rhs)
  }
}

use ::rcc_shared::Number;
impl<'c> IntoWith<SourceSpan, RawConstant<'c>> for Number {
  fn into_with(self, span: SourceSpan) -> RawConstant<'c> {
    match self {
      Self::Integral(integral) => RawConstant::new(integral, span),
      Self::Floating(floating) => RawConstant::new(floating, span),
    }
  }
}

impl<ExprTy> RawUnary<ExprTy> {
  pub fn new(
    operator: Operator,
    operand: ExprTy,
    kind: super::UnaryKind,
    span: SourceSpan,
  ) -> Self {
    debug_assert!(operator.unary(), "not a unary operator! got {:?}", operator);
    Self {
      operator,
      operand: operand.into(),
      kind,
      span,
    }
  }

  #[inline(always)]
  pub fn prefix(operator: Operator, operand: ExprTy, span: SourceSpan) -> Self {
    debug_assert!(operator.unary(), "not a unary operator! got {:?}", operator);
    Self::new(operator, operand, super::UnaryKind::Prefix, span)
  }

  #[inline(always)]
  pub fn postfix(
    operator: Operator,
    operand: ExprTy,
    span: SourceSpan,
  ) -> Self {
    debug_assert!(operator.unary(), "not a unary operator! got {:?}", operator);
    Self::new(operator, operand, super::UnaryKind::Postfix, span)
  }
}

impl<ExprTy> RawBinary<ExprTy> {
  pub fn from_operator(
    operator: Operator,
    left: ExprTy,
    right: ExprTy,
    span: SourceSpan,
  ) -> Option<Self> {
    match operator.binary() {
      true => Some(Self {
        operator,
        left: left.into(),
        right: right.into(),
        span,
      }),
      false => None,
    }
  }

  pub fn from_operator_unchecked(
    operator: Operator,
    left: ExprTy,
    right: ExprTy,
    span: SourceSpan,
  ) -> Self {
    debug_assert!(operator.binary());
    Self {
      operator,
      left: left.into(),
      right: right.into(),
      span,
    }
  }

  #[inline]
  pub fn new(
    operator: Operator,
    left: ExprTy,
    right: ExprTy,
    span: SourceSpan,
  ) -> Self {
    Self::from_operator(operator, left, right, span)
      .expect("not a binary operator! use `from_operator` instead")
  }
}
impl<ExprTy> RawTernary<ExprTy> {
  pub fn new(
    condition: ExprTy,
    then_expr: ExprTy,
    else_expr: ExprTy,
    span: SourceSpan,
  ) -> Self {
    Self {
      condition: condition.into(),
      then_expr: Some(then_expr.into()),
      else_expr: else_expr.into(),
      span,
    }
  }

  pub fn elvis(condition: ExprTy, else_expr: ExprTy, span: SourceSpan) -> Self {
    Self {
      condition: condition.into(),
      then_expr: None,
      else_expr: else_expr.into(),
      span,
    }
  }

  #[inline]
  pub fn is_elvis(&self) -> bool {
    self.then_expr.is_some()
  }
}

impl<ExprTy> RawArraySubscript<ExprTy> {
  pub fn new(array: ExprTy, index: ExprTy, span: SourceSpan) -> Self {
    Self {
      array: array.into(),
      index: index.into(),
      span,
    }
  }
}
impl<ExprTy, TypeTy> RawSizeOf<ExprTy, TypeTy> {
  pub fn new(sizeof: RawSizeOfKind<ExprTy, TypeTy>, span: SourceSpan) -> Self {
    Self { sizeof, span }
  }
}

impl<ExprTy, TypeTy> IntoWith<SourceSpan, RawSizeOf<ExprTy, TypeTy>>
  for RawSizeOfKind<ExprTy, TypeTy>
{
  fn into_with(self, span: SourceSpan) -> RawSizeOf<ExprTy, TypeTy> {
    RawSizeOf::new(self, span)
  }
}
impl<ExprTy, TypeTy> RawCStyleCast<ExprTy, TypeTy> {
  pub fn new(
    target_type: Box<TypeTy>,
    expr: Box<ExprTy>,
    span: SourceSpan,
  ) -> Self {
    Self {
      target_type,
      expr,
      span,
    }
  }
}
impl<ExprTy> RawCall<ExprTy> {
  pub fn new(callee: ExprTy, arguments: Vec<ExprTy>, span: SourceSpan) -> Self {
    Self {
      callee: callee.into(),
      arguments,
      span,
    }
  }
}
impl<ExprTy> RawParen<ExprTy> {
  pub fn new(expr: ExprTy, span: SourceSpan) -> Self {
    Self {
      expr: expr.into(),
      span,
    }
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::*;

  impl<'c> Display for RawConstant<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.inner)
    }
  }

  impl<ExprTy: Display> Display for RawCall<ExprTy> {
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
  impl<ExprTy: Display> Display for RawUnary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      if matches!(self.operator, Operator::PlusPlus | Operator::MinusMinus) {
        write!(f, "({} {}{})", self.operand, self.kind, self.operator)
      } else {
        write!(f, "({} {})", self.operand, self.operator)
      }
    }
  }
  impl<ExprTy: Display> Display for RawBinary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }
  impl<ExprTy: Display> Display for RawTernary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match &self.then_expr {
        Some(then_expr) => write!(
          f,
          "({} ? {} : {})",
          self.condition, then_expr, self.else_expr
        ),

        None => write!(f, "({} ?: {})", self.condition, self.else_expr),
      }
    }
  }
  impl<ExprTy: Display> Display for RawParen<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({})", self.expr)
    }
  }
  impl<ExprTy: Display, TypeTy: Display> Display for RawSizeOf<ExprTy, TypeTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.sizeof)
    }
  }
  impl<ExprTy: Display, TypeTy: Display> Display
    for RawSizeOfKind<ExprTy, TypeTy>
  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        RawSizeOfKind::Type(typ) => write!(f, "sizeof({})", typ),
        RawSizeOfKind::Expression(expr) => write!(f, "sizeof({})", expr),
      }
    }
  }
  // noop display impl for the rest
  impl<ExprTy: Display> Display for RawArraySubscript<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "(({})[{}])", self.array, self.index)
    }
  }
  impl Display for RawCompoundLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "<compound literal - not implemented>")
    }
  }
  impl<ExprTy: Display, TypeTy> Display for RawCStyleCast<ExprTy, TypeTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "<C-style cast - not implemented>")
    }
  }
  impl<'c, ExprTy: Display> Display for RawMemberAccess<'c, ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({}.{})", self.object, self.member)
    }
  }
}
