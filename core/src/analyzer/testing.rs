use crate::{
  analyzer::expression::{Binary, Constant, Expression},
  common::Operator,
  types::QualifiedType,
};

impl Expression {
  pub fn oneplusone() -> Self {
    Self::new_rvalue(
      Binary::new(
        Operator::Plus,
        Self::new_rvalue(Constant::Int(1).into(), QualifiedType::int()),
        Self::new_rvalue(Constant::Int(1).into(), QualifiedType::int()),
      )
      .into(),
      QualifiedType::int(),
    )
  }
}
