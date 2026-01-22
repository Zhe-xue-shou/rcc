use crate::{
  common::Operator,
  parser::expression::{Binary, Constant, Expression},
};

impl Expression {
  pub fn oneplusone() -> Self {
    Self::Binary(Binary {
      operator: Operator::Plus,
      left: Self::Constant(Constant::Int(1).into()).into(),
      right: Self::Constant(Constant::Int(1)).into(),
    })
  }
}
