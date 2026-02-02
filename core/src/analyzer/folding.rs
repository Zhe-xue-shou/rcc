#![allow(unused)]

use super::expression::{
  Binary, Constant, ConstantLiteral, Expression, Paren, RawExpr, SizeOf,
  SizeOfKind, Ternary, Unary, Variable,
};
use crate::{
  common::Operator,
  diagnosis::{DiagData::*, Diagnosis},
  types::TypeInfo,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldingLevel {
  Failed,
  Partial,
  Success,
}
use ::rc_utils::contract_assert;
use FoldingLevel::*;

impl FoldingLevel {
  fn merge(lhs: FoldingLevel, rhs: FoldingLevel) -> FoldingLevel {
    match (lhs, rhs) {
      (Success, Success) => Success,
      (Failed, Failed) => Failed,
      _ => Partial,
    }
  }

  fn merge_with(self, other: FoldingLevel) -> FoldingLevel {
    Self::merge(self, other)
  }
}

#[derive(Debug)]
pub struct FoldingResult<T> {
  output: T,
  level: FoldingLevel,
}

impl<T> FoldingResult<T> {
  fn new(output: T, level: FoldingLevel) -> Self {
    Self { output, level }
  }

  fn success(output: T) -> Self {
    Self::new(output, Success)
  }

  fn partial(output: T) -> Self {
    Self::new(output, Partial)
  }

  fn failed(output: T) -> Self {
    Self::new(output, Failed)
  }

  pub fn destructure(self) -> (T, FoldingLevel) {
    (self.output, self.level)
  }

  fn map<U>(self, f: impl FnOnce(T) -> U) -> FoldingResult<U> {
    FoldingResult {
      output: f(self.output),
      level: self.level,
    }
  }

  fn is_success(&self) -> bool {
    matches!(self.level, Success)
  }

  fn is_partial(&self) -> bool {
    matches!(self.level, Partial)
  }

  fn is_failed(&self) -> bool {
    matches!(self.level, Failed)
  }
}

/// Folding trait for constant expression evaluation
pub trait Folding {
  /// The type after folding
  type Folded;

  /// This serves as a never-fail folding mechanism,
  /// all errors and warnings shall be handled via `diag` parameter.
  /// [`Operational`](crate::diagnosis::Operational) is recommended.
  /// If folding is not possible, return self unchanged.
  /// So it may end up being a no-op, partial-fold, or full-fold.
  ///
  /// If [`Diagnosis`] is not required, use [`NoOp`](crate::diagnosis::NoOp) as the dummy parameter.
  #[must_use]
  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded>;
}

impl Folding for Expression {
  type Folded = Self;

  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    let (raw_expr, expr_type, _value_category) = self.destructure();
    raw_expr
      .fold(diag)
      .map(|folded_expr| Self::new_rvalue(folded_expr, expr_type))
  }
}

impl Folding for RawExpr {
  type Folded = RawExpr;

  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    match self {
      RawExpr::Empty => FoldingResult::failed(self),
      RawExpr::Constant(constant) => constant.fold(diag),
      RawExpr::Unary(unary) => unary.fold(diag),
      RawExpr::Binary(binary) => binary.fold(diag),
      RawExpr::Call(_) => FoldingResult::failed(self),
      RawExpr::Paren(paren) => paren.fold(diag),
      RawExpr::MemberAccess(member_access) => todo!(),
      RawExpr::Ternary(ternary) => ternary.fold(diag),
      RawExpr::SizeOf(size_of) => size_of.fold(diag),
      RawExpr::CStyleCast(cstyle_cast) => todo!(),
      RawExpr::ArraySubscript(array_subscript) => todo!(),
      RawExpr::CompoundLiteral(compound_literal) => todo!(),
      RawExpr::Variable(variable) => variable.fold(diag),
      RawExpr::ImplicitCast(implicit_cast) => todo!(),
      // assignment expr is not considered constant expr in C, but in C++ it is.
      RawExpr::Assignment(_) => FoldingResult::failed(self),
    }
  }
}

impl Folding for Constant {
  type Folded = RawExpr;

  #[inline]
  fn fold(self, _diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    FoldingResult::success(self.into())
  }
}

impl Folding for Unary {
  type Folded = RawExpr;

  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    debug_assert!(
      self.operator.unary(),
      "not an unary operator! should not happen!"
    );

    let (folded_operand, level) = self.operand.fold(diag).destructure();

    if level != Success {
      return FoldingResult {
        output: Self {
          operand: folded_operand.into(),
          ..self
        }
        .into(),
        level,
      };
    }

    contract_assert!(
      folded_operand.raw_expr().is_constant(),
      "only implemented for constant var of constant eval"
    );

    match self.operator {
      Operator::Plus => FoldingResult::success(folded_operand.destructure().0),
      _ => todo!(),
    }
  }
}

impl Folding for Binary {
  type Folded = RawExpr;

  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    debug_assert!(
      self.operator.binary(),
      "not a binary operator! should not happen!"
    );
    let (folded_lhs, lhs_level) = self.left.fold(diag).destructure();
    let (folded_rhs, rhs_level) = self.right.fold(diag).destructure();
    let level = FoldingLevel::merge(lhs_level, rhs_level);
    if level != Success {
      return FoldingResult::new(
        Self {
          left: folded_lhs.into(),
          right: folded_rhs.into(),
          ..self
        }
        .into(),
        level,
      );
    }
    assert!(
      folded_lhs.raw_expr().is_constant()
        && folded_rhs.raw_expr().is_constant(),
      "only implemented for constant var of constant eval"
    );
    assert!(
      folded_lhs.qualified_type() == folded_rhs.qualified_type(),
      "type checker makes sure both sides have the same type via `ImpliedCast`!"
    );
    match self.operator {
      Operator::Plus => {
        let lhs_const = folded_lhs
          .raw_expr()
          .as_constant()
          .expect("lhs is constant");
        let rhs_const = folded_rhs
          .raw_expr()
          .as_constant()
          .expect("rhs is constant");
        match (&lhs_const.constant, &rhs_const.constant) {
          (ConstantLiteral::Int(l), ConstantLiteral::Int(r)) => {
            let (res, of) = l.overflowing_add(*r);
            if of {
              diag.add_warning(
                ArithmeticOpOverflow(
                  ConstantLiteral::Int(*l),
                  ConstantLiteral::Int(*r),
                  self.operator,
                ),
                self.span,
              )
            }
            FoldingResult::success(ConstantLiteral::Int(res).into())
          },
          _ => todo!(),
        }
      },
      _ => todo!(),
    }
  }
}

impl Folding for Ternary {
  type Folded = RawExpr;

  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    todo!()
  }
}

impl Folding for SizeOf {
  type Folded = RawExpr;

  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    match self.sizeof {
      SizeOfKind::Type(qualified_type) => FoldingResult::new(
        ConstantLiteral::ULongLong(qualified_type.size() as u64).into(),
        if qualified_type.size() > 0 {
          Success
        } else {
          // incomplete type, VLA todo!!!
          Failed
        },
      ),
      SizeOfKind::Expression(expr) => expr.fold(diag).map(|expr|{
        match expr.raw_expr().as_constant() {
          None => panic!("impossible; type checker ensures sizeof has an expr which produces ull"),
          Some(constant) => ConstantLiteral::ULongLong(expr.unqualified_type().size() as u64).into(),
        }
      }),
    }
  }
}

impl Folding for Variable {
  type Folded = RawExpr;

  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    if self.name.borrow().is_constexpr() {
      diag.add_error(
        UnsupportedFeature("constexpr variable not implemented".to_string()),
        self.span,
      );
      FoldingResult::failed(self.into())
    } else {
      diag.add_error(
        ExprNotConstant(format!(
          "variable {} is not a constexpr variable; only constexpr variables are allowed",
          self.name.borrow().name
        )),
        self.span,
      );
      FoldingResult::failed(self.into())
    }
  }
}

impl Folding for Paren {
  type Folded = RawExpr;

  fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Self::Folded> {
    self.expr.fold(diag).map(|expr| expr.destructure().0)
  }
}
