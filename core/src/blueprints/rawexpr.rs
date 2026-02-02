use ::rc_utils::{Dummy, IntoWith};

use crate::{
  common::{Operator, SourceSpan},
  types::{Constant, QualifiedType},
};

#[macro_export(local_inner_macros)]
macro_rules! type_alias_expr {
  ($exprty:ident,$typety:ident $(, $extra:ident)*) => {
    /// likely a sophisticated version of the Two-Level Types described in
    /// [this article](https://blog.ezyang.com/2013/05/the-ast-typing-problem/),
    /// I probably used the Parametric Polymorphism to "tie the knot" of recursion.
    #[derive(Debug)]
    pub enum RawExpr {
      Empty, // no-op for error recovery; for empty expr should use Option<ExprTy> instead
      Constant(Constant),
      Unary(Unary),
      Binary(Binary),
      Call(Call),
      Paren(Paren),
      MemberAccess(MemberAccess),
      Ternary(Ternary),
      SizeOf(SizeOf),
      CStyleCast(CStyleCast),                     // (int)x
      ArraySubscript(ArraySubscript), // arr[i]
      CompoundLiteral(CompoundLiteral), // (struct Point){.x=1, .y=2}
      $(
        // Generate a variant for each extra type
        $extra($extra),
      )*
    }
    /// exists to avoid name clash with `Constant` in this module; this is a design mistake
    pub type ConstantLiteral = $crate::types::Constant;
    /// type or expression
    pub type SizeOfKind = $crate::blueprints::RawSizeOfKind<$exprty, $typety>;
    pub type Constant = $crate::blueprints::RawConstant;
    pub type Unary = $crate::blueprints::RawUnary<$exprty>;
    pub type Binary = $crate::blueprints::RawBinary<$exprty>;
    pub type Call = $crate::blueprints::RawCall<$exprty>;
    pub type Paren = $crate::blueprints::RawParen<$exprty>;
    pub type MemberAccess = $crate::blueprints::RawMemberAccess<$exprty>;
    pub type Ternary = $crate::blueprints::RawTernary<$exprty>;
    pub type SizeOf = $crate::blueprints::RawSizeOf<$exprty, $typety>;
    pub type CStyleCast = $crate::blueprints::RawCStyleCast<$exprty>;
    pub type ArraySubscript = $crate::blueprints::RawArraySubscript<$exprty>;
    pub type CompoundLiteral = $crate::blueprints::RawCompoundLiteral;

    mod fmtrawexpr {
      use super::*;
      use ::std::fmt::Display;
      impl Display for RawExpr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          match self {
            RawExpr::Constant(c) => <Constant as Display>::fmt(c, f),
            RawExpr::Unary(u) => <Unary as Display>::fmt(u, f),
            RawExpr::Binary(b) => <Binary as Display>::fmt(b, f),
            RawExpr::Ternary(t) => <Ternary as Display>::fmt(t, f),
            RawExpr::Call(call) => <Call as Display>::fmt(call, f),
            RawExpr::Paren(p) => <Paren as Display>::fmt(p, f),
            RawExpr::Empty => ::std::write!(f, "<noop>"),
            $(
              RawExpr::$extra(inner) => <$extra as Display>::fmt(inner, f),
            )*
            _ => ::std::todo!(),
          }
        }
      }
    }
    mod cvtrawexpr {
      use super::*;

      ::rc_utils::interconvert!(Constant, RawExpr);
      ::rc_utils::interconvert!(Unary, RawExpr);
      ::rc_utils::interconvert!(Binary, RawExpr);
      ::rc_utils::interconvert!(Call, RawExpr);
      ::rc_utils::interconvert!(Paren, RawExpr);
      ::rc_utils::interconvert!(MemberAccess, RawExpr);
      ::rc_utils::interconvert!(Ternary, RawExpr);
      ::rc_utils::interconvert!(SizeOf, RawExpr);
      ::rc_utils::interconvert!(CStyleCast, RawExpr);
      ::rc_utils::interconvert!(ArraySubscript, RawExpr);
      ::rc_utils::interconvert!(CompoundLiteral, RawExpr);
      $(
        ::rc_utils::interconvert!($extra, RawExpr);
      )*

      ::rc_utils::make_trio_for!(Constant, RawExpr);
      ::rc_utils::make_trio_for!(Unary, RawExpr);
      ::rc_utils::make_trio_for!(Binary, RawExpr);
      ::rc_utils::make_trio_for!(Call, RawExpr);
      ::rc_utils::make_trio_for!(Paren, RawExpr);
      ::rc_utils::make_trio_for!(MemberAccess, RawExpr);
      ::rc_utils::make_trio_for!(Ternary, RawExpr);
      ::rc_utils::make_trio_for!(SizeOf, RawExpr);
      ::rc_utils::make_trio_for!(CStyleCast, RawExpr);
      ::rc_utils::make_trio_for!(ArraySubscript, RawExpr);
      ::rc_utils::make_trio_for!(CompoundLiteral, RawExpr);
      $(
        ::rc_utils::make_trio_for!($extra, RawExpr);
      )*

      impl From<ConstantLiteral> for RawExpr {
        fn from(constant: ConstantLiteral) -> Self {
          RawExpr::Constant(constant.into())
        }
      }

      impl ::rc_utils::IntoWith<SourceSpan, RawExpr> for ConstantLiteral {
        fn into_with(self, span: SourceSpan) -> RawExpr {
          RawExpr::Constant(self.into_with(span))
        }
      }

      impl From<SizeOfKind> for RawExpr {
        fn from(sizeof: SizeOfKind) -> Self {
          RawExpr::SizeOf(sizeof.into())
        }
      }

      impl ::rc_utils::IntoWith<SourceSpan, RawExpr> for SizeOfKind {
        fn into_with(self, span: SourceSpan) -> RawExpr {
          RawExpr::SizeOf(self.into_with(span))
        }
      }
    }

    mod getspan {
      use super::*;
      use $crate::common::SourceSpan;
      use ::rc_utils::Dummy;
      impl RawExpr {
        pub fn span(&self) -> SourceSpan {
          match self {
            RawExpr::Empty => SourceSpan::dummy(),
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

    ::rc_utils::static_assert!(
      ::std::mem::size_of::<RawExpr>() <= 64,
      "RawExpr size exceeds 64 bytes",
    );

  };
}
#[derive(Debug)]
pub struct RawConstant {
  pub constant: Constant,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawUnary<ExprTy> {
  pub operator: Operator,
  pub operand: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawBinary<ExprTy> {
  pub operator: Operator,
  pub left: Box<ExprTy>,
  pub right: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawCall<ExprTy> {
  pub callee: Box<ExprTy>,
  pub arguments: Vec<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawParen<ExprTy> {
  pub expr: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawMemberAccess<ExprTy> {
  pub object: Box<ExprTy>,
  pub member: String,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawTernary<ExprTy> {
  pub condition: Box<ExprTy>,
  pub then_expr: Box<ExprTy>,
  pub else_expr: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub enum RawSizeOfKind<ExprTy, TypeTy> {
  Type(Box<TypeTy>), // ignore for now
  Expression(Box<ExprTy>),
}

#[derive(Debug)]
pub struct RawSizeOf<ExprTy, TypeTy> {
  pub sizeof: RawSizeOfKind<ExprTy, TypeTy>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawCStyleCast<ExprTy> {
  pub target_type: Box<QualifiedType>,
  pub expr: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawArraySubscript<ExprTy> {
  pub array: Box<ExprTy>,
  pub index: Box<ExprTy>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawCompoundLiteral {
  pub target_type: Box<QualifiedType>,
  // pub initializer: Initializer,
  pub span: SourceSpan,
}

impl RawConstant {
  pub fn new(constant: Constant, span: SourceSpan) -> Self {
    Self { constant, span }
  }
}

// impl deref for rawconstant to constant
impl std::ops::Deref for RawConstant {
  type Target = Constant;

  fn deref(&self) -> &Self::Target {
    &self.constant
  }
}

impl From<Constant> for RawConstant {
  fn from(constant: Constant) -> Self {
    Self::new(constant, SourceSpan::dummy())
  }
}

impl IntoWith<SourceSpan, RawConstant> for Constant {
  fn into_with(self, span: SourceSpan) -> RawConstant {
    RawConstant::new(self, span)
  }
}

impl<ExprTy> RawUnary<ExprTy> {
  pub fn from_operator(
    operator: Operator,
    operand: ExprTy,
    span: SourceSpan,
  ) -> Option<Self> {
    match operator.unary() {
      true => Some(Self {
        operator,
        operand: operand.into(),
        span,
      }),
      false => None,
    }
  }

  pub fn new(operator: Operator, operand: ExprTy, span: SourceSpan) -> Self {
    Self::from_operator(operator, operand, span).unwrap()
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

  pub fn new(
    operator: Operator,
    left: ExprTy,
    right: ExprTy,
    span: SourceSpan,
  ) -> Self {
    Self::from_operator(operator, left, right, span).unwrap()
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
      then_expr: then_expr.into(),
      else_expr: else_expr.into(),
      span,
    }
  }
}
impl<ExprTy, TypeTy> RawSizeOf<ExprTy, TypeTy> {
  pub fn new(sizeof: RawSizeOfKind<ExprTy, TypeTy>, span: SourceSpan) -> Self {
    Self { sizeof, span }
  }
}
impl<ExprTy, TypeTy> From<RawSizeOfKind<ExprTy, TypeTy>>
  for RawSizeOf<ExprTy, TypeTy>
{
  fn from(sizeof: RawSizeOfKind<ExprTy, TypeTy>) -> Self {
    Self::new(sizeof, SourceSpan::dummy())
  }
}

impl<ExprTy, TypeTy> IntoWith<SourceSpan, RawSizeOf<ExprTy, TypeTy>>
  for RawSizeOfKind<ExprTy, TypeTy>
{
  fn into_with(self, span: SourceSpan) -> RawSizeOf<ExprTy, TypeTy> {
    RawSizeOf::new(self, span)
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

  impl Display for RawConstant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.constant)
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
      write!(f, "({} {})", self.operand, self.operator)
    }
  }
  impl<ExprTy: Display> Display for RawBinary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }
  impl<ExprTy: Display> Display for RawTernary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "({} ? {} : {})",
        self.condition, self.then_expr, self.else_expr
      )
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
}
