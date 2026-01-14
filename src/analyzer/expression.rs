use ::strum_macros::Display;

use crate::{
  common::{
    environment::SymbolRef,
    operator::{Category, Operator},
    types::{CastType, Primitive, QualifiedType, Qualifiers, Type, TypeInfo},
  },
  type_alias_expr,
};

type_alias_expr! {Expression, QualifiedType, Variable, ImplicitCast, Assignment}
#[derive(Debug, Clone, Copy, Display, PartialEq)]
pub enum ValueCategory {
  #[strum(serialize = "lvalue")]
  LValue,
  /// 6.3.2: "rvalue" is in this document described as the "value of an expression".
  ///        which, is different from the one defined in C++ standard.
  #[strum(serialize = "rvalue")]
  RValue,
}
#[derive(Debug)]
pub struct Expression {
  pub(super) raw_expr: RawExpr,
  pub(super) expr_type: QualifiedType,
  pub(super) value_category: ValueCategory,
}
impl Expression {
  pub fn new(
    raw_expr: RawExpr,
    expr_type: QualifiedType,
    value_category: ValueCategory,
  ) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category,
    }
  }

  pub fn new_rvalue(raw_expr: RawExpr, expr_type: QualifiedType) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category: ValueCategory::RValue,
    }
  }

  pub fn new_lvalue(raw_expr: RawExpr, expr_type: QualifiedType) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category: ValueCategory::LValue,
    }
  }

  pub fn unqualified_type(&self) -> &Type {
    &self.expr_type.unqualified_type
  }

  pub fn qualifiers(&self) -> &Qualifiers {
    &self.expr_type.qualifiers
  }

  pub fn qualified_type(&self) -> &QualifiedType {
    &self.expr_type
  }

  pub fn raw_expr(&self) -> &RawExpr {
    &self.raw_expr
  }

  pub fn value_category(&self) -> ValueCategory {
    self.value_category
  }
}
impl Primitive {
  #[must_use]
  pub fn common_type(lhs: &Self, rhs: &Self) -> (Self, CastType, CastType) {
    // If both operands have the same type, then no further conversion is needed.
    // first: _Decimal types ignored
    // also, complex types ignored
    if lhs == rhs {
      return (lhs.clone(), CastType::Noop, CastType::Noop);
    }
    if matches!(lhs, Self::Void | Self::Nullptr)
      || matches!(rhs, Self::Void | Self::Nullptr)
    {
      panic!("Invalid types for common type: {:?}, {:?}", lhs, rhs);
    }
    // otherwise, if either operand is of some floating type, the other operand is converted to it.
    // Otherwise, if any of the two types is an enumeration, it is converted to its underlying type. - handled upstream
    match (lhs.is_floating_point(), rhs.is_floating_point()) {
      (true, false) =>
        (lhs.clone(), CastType::Noop, CastType::IntegralToFloating),
      (false, true) =>
        (rhs.clone(), CastType::IntegralToFloating, CastType::Noop),
      (true, true) => Self::common_floating_rank(lhs.clone(), rhs.clone()),
      (false, false) => Self::common_integer_rank(lhs.clone(), rhs.clone()),
    }
  }

  #[must_use]
  fn common_floating_rank(lhs: Self, rhs: Self) -> (Self, CastType, CastType) {
    assert!(lhs.is_floating_point() && rhs.is_floating_point());
    if lhs.floating_rank() > rhs.floating_rank() {
      (lhs, CastType::Noop, CastType::FloatingCast)
    } else {
      (rhs, CastType::FloatingCast, CastType::Noop)
    }
  }

  #[must_use]
  fn common_integer_rank(lhs: Self, rhs: Self) -> (Self, CastType, CastType) {
    assert!(lhs.is_integer() && rhs.is_integer());

    let (lhs, _) = lhs.integer_promotion();
    let (rhs, _) = rhs.integer_promotion();
    if lhs == rhs {
      // done
      return (lhs, CastType::Noop, CastType::Noop);
    }
    if lhs.is_unsigned() == rhs.is_unsigned() {
      return if lhs.integer_rank() > rhs.integer_rank() {
        (lhs, CastType::Noop, CastType::IntegralCast)
      } else {
        (rhs, CastType::IntegralCast, CastType::Noop)
      };
    }
    if lhs.is_unsigned() {
      assert!(!rhs.is_unsigned());
      if lhs.integer_rank() >= rhs.integer_rank() {
        (lhs, CastType::Noop, CastType::IntegralCast)
      } else if rhs.size() > lhs.size() {
        (rhs, CastType::IntegralCast, CastType::Noop)
      } else {
        // if the signed type cannot represent all values of the unsigned type, return the unsigned version of the signed type
        // the signed type is always larger than the corresponding unsigned type on my x86_64 architecture
        // so this branch is unlikely to be taken
        let promoted_rhs = rhs.into_unsigned();
        (promoted_rhs, CastType::IntegralCast, CastType::IntegralCast)
      }
    } else {
      assert!(rhs.is_unsigned());
      // symmetric to above
      if rhs.integer_rank() >= lhs.integer_rank() {
        (rhs, CastType::Noop, CastType::IntegralCast)
      } else if lhs.size() > rhs.size() {
        (lhs, CastType::IntegralCast, CastType::Noop)
      } else {
        let promoted_lhs = lhs.into_unsigned();
        (promoted_lhs, CastType::IntegralCast, CastType::IntegralCast)
      }
    }
  }
}
impl Expression {
  pub fn is_lvalue(&self) -> bool {
    matches!(self.value_category, ValueCategory::LValue)
  }

  /// 6.3.2.1:  A modifiable lvalue is an lvalue that does not have array type, does not have an incomplete
  ///           type, does not have a const-qualified type, and if it is a structure or union, does not have any
  ///           member (including, recursively, any member or element of all contained aggregates or unions) with
  ///           a const-qualified type.
  pub fn is_modifiable_lvalue(&self) -> bool {
    self.is_lvalue() && self.qualified_type().is_modifiable()
  }

  pub fn to_rvalue(self) -> Self {
    Self {
      value_category: ValueCategory::RValue,
      ..self
    }
  }

  pub fn default_int() -> Self {
    Self {
      raw_expr: RawExpr::Constant(Constant::Int(0)),
      expr_type: QualifiedType::new(
        Qualifiers::empty(),
        Type::Primitive(Primitive::Int),
      ),
      value_category: ValueCategory::RValue,
    }
  }
}

impl ::core::default::Default for Expression {
  fn default() -> Self {
    Self {
      raw_expr: RawExpr::Empty,
      expr_type: QualifiedType::new(
        Qualifiers::empty(),
        Type::Primitive(Primitive::Void),
      ),
      value_category: ValueCategory::RValue,
    }
  }
}
#[derive(Debug)]
pub struct Variable {
  pub name: SymbolRef,
}
impl Variable {
  pub fn new(name: SymbolRef) -> Self {
    Self { name }
  }
}
#[derive(Debug)]
pub struct ImplicitCast {
  pub expr: Box<Expression>,
  pub cast_type: CastType,
}
impl ImplicitCast {
  pub fn new(expr: Box<Expression>, cast_type: CastType) -> Self {
    Self { expr, cast_type }
  }
}
/// assignment-expression:
///    - conditional-expression
///    - unary-expression assignment-operator assignment-expression
#[derive(Debug)]
pub struct Assignment {
  pub operator: Operator,
  pub left: Box<Expression>,
  pub right: Box<Expression>,
}
impl Assignment {
  pub fn from_operator(
    operator: Operator,
    left: Expression,
    right: Expression,
  ) -> Option<Self> {
    match operator.category() {
      Category::Assignment => Some(Self {
        operator,
        left: Box::new(left),
        right: Box::new(right),
      }),
      _ => None,
    }
  }

  pub fn new(operator: Operator, left: Expression, right: Expression) -> Self {
    Self::from_operator(operator, left, right).unwrap()
  }
}
mod fmt {

  use ::std::fmt::Display;

  use super::{Assignment, Expression, ImplicitCast, Variable};

  impl Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }

  impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.raw_expr)
    }
  }
  // the "specialization" for the smart pointer case
  impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name.borrow())
    }
  }
  impl Display for ImplicitCast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.expr)
    }
  }
}

mod test {

  #[test]
  fn int_float() {
    use super::*;

    let int_expr = Expression::new(
      RawExpr::Constant(Constant::Int(42)),
      QualifiedType::new_unqualified(Type::from(Primitive::Int)),
      ValueCategory::RValue,
    );
    let float_expr = Expression::new(
      RawExpr::Constant(Constant::Float(3.14)),
      QualifiedType::new_unqualified(Type::from(Primitive::Float)),
      ValueCategory::RValue,
    );
    let promoted_expr =
      Expression::usual_arithmetic_conversion(int_expr, float_expr)
        .unwrap()
        .2;
    // type shall be
    println!("Promoted expression: {:#?}", promoted_expr);
  }
}
