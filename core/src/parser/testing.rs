use ::rc_utils::Dummy;

use crate::{
  common::{Operator, SourceSpan},
  parser::expression::{Binary, ConstantLiteral, Expression},
};

impl Expression {
  pub fn oneplusone() -> Self {
    Self::Binary(Binary {
      operator: Operator::Plus,
      left: Self::Constant(ConstantLiteral::Int(1).into()).into(),
      right: Self::Constant(ConstantLiteral::Int(1).into()).into(),
      span: SourceSpan::dummy(),
    })
  }
}
