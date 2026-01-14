use crate::{common::operator::Operator, types::QualifiedType};
#[macro_export(local_inner_macros)]
macro_rules! type_alias_expr {
  ($exprty:ident,$typety:ident $(, $extra:ident)*) => {
    /// likely a sophisticated version of the Two-Level Types
    /// [this article](https://blog.ezyang.com/2013/05/the-ast-typing-problem/),
    /// I probably used the Parametric Polymorphism to "tie the knot" of recursion.
    #[derive(Debug)]
    pub enum RawExpr {
      Empty, // no-op for error recovery; for empty expr should use Option<ExprTy> instead
      Constant(Constant),
      Unary(Unary),
      Binary(Binary),
      Call(Call),
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
    pub type Constant = crate::types::Constant;
    pub type Unary = crate::common::rawexpr::Unary<$exprty>;
    pub type Binary = crate::common::rawexpr::Binary<$exprty>;
    pub type Call = crate::common::rawexpr::Call<$exprty>;
    pub type MemberAccess = crate::common::rawexpr::MemberAccess<$exprty>;
    pub type Ternary = crate::common::rawexpr::Ternary<$exprty>;
    pub type SizeOf = crate::common::rawexpr::SizeOf<$exprty, $typety>;
    pub type CStyleCast = crate::common::rawexpr::CStyleCast<$exprty>;
    pub type ArraySubscript = crate::common::rawexpr::ArraySubscript<$exprty>;
    pub type CompoundLiteral = crate::common::rawexpr::CompoundLiteral;

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
      ::lilac_utils::interconvert!(Constant, RawExpr);
      ::lilac_utils::interconvert!(Unary, RawExpr);
      ::lilac_utils::interconvert!(Binary, RawExpr);
      ::lilac_utils::interconvert!(Call, RawExpr);
      ::lilac_utils::interconvert!(MemberAccess, RawExpr);
      ::lilac_utils::interconvert!(Ternary, RawExpr);
      ::lilac_utils::interconvert!(SizeOf, RawExpr);
      ::lilac_utils::interconvert!(CStyleCast, RawExpr);
      ::lilac_utils::interconvert!(ArraySubscript, RawExpr);
      ::lilac_utils::interconvert!(CompoundLiteral, RawExpr);
      $(
        ::lilac_utils::interconvert!($extra, RawExpr);
      )*
    }
  };
}

#[derive(Debug)]
pub struct Unary<ExprTy> {
  pub operator: Operator,
  pub expression: Box<ExprTy>,
}
#[derive(Debug)]
pub struct Binary<ExprTy> {
  pub operator: Operator,
  pub left: Box<ExprTy>,
  pub right: Box<ExprTy>,
}
#[derive(Debug)]
pub struct Call<ExprTy> {
  pub callee: Box<ExprTy>,
  pub arguments: Vec<ExprTy>,
}
#[derive(Debug)]
pub struct MemberAccess<ExprTy> {
  pub object: Box<ExprTy>,
  pub member: String,
}
#[derive(Debug)]
pub struct Ternary<ExprTy> {
  pub condition: Box<ExprTy>,
  pub then_expr: Box<ExprTy>,
  pub else_expr: Box<ExprTy>,
}
#[derive(Debug)]
pub enum SizeOf<ExprTy, TypeTy> {
  Type(TypeTy), // ignore for now
  Expression(Box<ExprTy>),
}

#[derive(Debug)]
pub struct CStyleCast<ExprTy> {
  pub target_type: QualifiedType,
  pub expression: Box<ExprTy>,
}
#[derive(Debug)]
pub struct ArraySubscript<ExprTy> {
  pub array: Box<ExprTy>,
  pub index: Box<ExprTy>,
}
#[derive(Debug)]
pub struct CompoundLiteral {
  pub target_type: QualifiedType,
  // pub initializer: Initializer,
}

impl<ExprTy> Unary<ExprTy> {
  pub fn from_operator(operator: Operator, expression: ExprTy) -> Option<Self> {
    match operator.unary() {
      true => Some(Self {
        operator,
        expression: Box::new(expression),
      }),
      false => None,
    }
  }

  pub fn new(operator: Operator, expression: ExprTy) -> Self {
    Self::from_operator(operator, expression).unwrap()
  }
}

impl<ExprTy> Binary<ExprTy> {
  pub fn from_operator(
    operator: Operator,
    left: ExprTy,
    right: ExprTy,
  ) -> Option<Self> {
    match operator.binary() {
      true => Some(Self {
        operator,
        left: Box::new(left),
        right: Box::new(right),
      }),
      false => None,
    }
  }

  pub fn new(operator: Operator, left: ExprTy, right: ExprTy) -> Self {
    Self::from_operator(operator, left, right).unwrap()
  }
}
impl<ExprTy> Ternary<ExprTy> {
  pub fn new(condition: ExprTy, then_expr: ExprTy, else_expr: ExprTy) -> Self {
    Self {
      condition: Box::new(condition),
      then_expr: Box::new(then_expr),
      else_expr: Box::new(else_expr),
    }
  }
}

impl<ExprTy> Call<ExprTy> {
  pub fn new(callee: ExprTy, arguments: Vec<ExprTy>) -> Self {
    Self {
      callee: Box::new(callee),
      arguments,
    }
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::{Binary, Call, Ternary, Unary};

  impl<ExprTy: Display> Display for Call<ExprTy> {
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
  impl<ExprTy: Display> Display for Unary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {})", self.expression, self.operator)
    }
  }
  impl<ExprTy: Display> Display for Binary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }
  impl<ExprTy: Display> Display for Ternary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "({} ? {} : {})",
        self.condition, self.then_expr, self.else_expr
      )
    }
  }
}
