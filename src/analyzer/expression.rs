use ::strum_macros::Display;

use crate::{
  common::{
    environment::SymbolRef,
    types::{Primitive, QualifiedType, Qualifiers, Type},
  },
  type_alias_expr,
};

type_alias_expr! {Expression,SymbolRef, QualifiedType}
#[derive(Debug, Clone, Copy, Display, PartialEq)]
pub enum ValueCategory {
  LValue,
  RValue,
}
#[derive(Debug)]
pub struct Expression {
  pub raw_expr: RawExpr,
  expr_type: QualifiedType,
  pub value_category: ValueCategory,
}
impl Expression {
  pub fn new(raw_expr: RawExpr, expr_type: QualifiedType, value_category: ValueCategory) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category,
    }
  }
  pub fn qualified_type(&self) -> &QualifiedType {
    &self.expr_type
  }
  pub fn raw_type(&self) -> &Type {
    &self.expr_type.unqualified_type
  }
  pub fn qualifiers(&self) -> &Qualifiers {
    &self.expr_type.qualifiers
  }
}
impl Expression {
  pub fn is_lvalue(&self) -> bool {
    matches!(self.value_category, ValueCategory::LValue)
  }

  pub fn is_modifiable_lvalue(&self) -> bool {
    self.is_lvalue() && !self.expr_type.qualifiers.contains(Qualifiers::Const)
  }
  pub fn is_constant(&self) -> bool {
    matches!(self.raw_expr, RawExpr::Constant(_))
  }
  pub fn to_rvalue(self) -> Self {
    Self {
      value_category: ValueCategory::RValue,
      ..self
    }
  }
}
impl Expression {
  // pub fn decay(self) -> Self {}
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
impl Variable {
  pub fn new(name: SymbolRef) -> Self {
    Self { name }
  }
}
mod fmt {
  use super::Variable;
  use ::std::fmt::Display;

  use super::Expression;

  impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}: {}", self.raw_expr, self.expr_type)
    }
  }
  // the "specialization" for the smart pointer case
  impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name.borrow())
    }
  }
}
