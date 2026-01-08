use ::strum_macros::Display;

use crate::common::types::{Primitive, QualifiedType, Type};
use crate::{
  common::{environment::SymbolRef, types::Qualifiers},
  type_alias_expr,
};

type_alias_expr! {Expression, QualifiedType, Variable, ImplicitCast}
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
  raw_expr: RawExpr,
  expr_type: QualifiedType,
  value_category: ValueCategory,
}
impl Expression {
  pub fn new(raw_expr: RawExpr, expr_type: QualifiedType, value_category: ValueCategory) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category,
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
      expr_type: QualifiedType::new(Qualifiers::empty(), Type::Primitive(Primitive::Int)),
      value_category: ValueCategory::RValue,
    }
  }
}
impl ::core::default::Default for Expression {
  fn default() -> Self {
    Self {
      raw_expr: RawExpr::Empty,
      expr_type: QualifiedType::new(Qualifiers::empty(), Type::Primitive(Primitive::Void)),
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
  pub target_type: QualifiedType,
  pub expr: Box<Expression>,
}
mod fmt {

  use super::{ImplicitCast, Variable};
  use ::std::fmt::Display;

  use super::Expression;

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
      write!(f, "({}){}", self.target_type, self.expr)
    }
  }
}
