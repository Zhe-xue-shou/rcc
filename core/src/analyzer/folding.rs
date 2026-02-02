#![allow(unused)]

use super::expression::{
  Binary, Constant, ConstantLiteral, Expression, Paren, RawExpr, SizeOf,
  SizeOfKind, Ternary, Unary, Variable,
};
use crate::{
  analyzer::expression::ImplicitCast,
  common::{Operator, SourceSpan},
  diagnosis::{DiagData::*, Diagnosis},
  types::{CastType, Compatibility, Primitive, QualifiedType, Type, TypeInfo},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldingLevel {
  Failed,
  Partial,
  Success,
}
use ::rc_utils::{contract_assert, contract_violation};
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

  fn merge_n<I: IntoIterator<Item = FoldingLevel>>(levels: I) -> FoldingLevel {
    levels
      .into_iter()
      .fold(Success, |acc, level| acc.merge_with(level))
  }

  fn merge_n_with<I: IntoIterator<Item = FoldingLevel>>(
    self,
    levels: I,
  ) -> FoldingLevel {
    Self::merge_n(::std::iter::once(self).chain(levels))
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
  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded>;
}

impl Folding for Expression {
  type Folded = Self;

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    let (raw_expr, expr_type, _value_category) = self.destructure();
    raw_expr
      .fold(target_type, diag)
      .map(|folded_expr| Self::new_rvalue(folded_expr, expr_type))
  }
}

impl Expression {
  #[inline]
  pub(super) fn try_fold(self, diag: &impl Diagnosis) -> (Self, FoldingLevel) {
    let (raw_expr, expr_type, _value_category) = self.destructure();
    raw_expr
      .fold(&expr_type, diag)
      .map(|folded_expr| Self::new_rvalue(folded_expr, expr_type))
      .destructure()
  }
}

impl Folding for RawExpr {
  type Folded = Self;

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    match self {
      RawExpr::Empty(_) => FoldingResult::failed(self),
      RawExpr::Constant(constant) => constant.fold(target_type, diag),
      RawExpr::Unary(unary) => unary.fold(target_type, diag),
      RawExpr::Binary(binary) => binary.fold(target_type, diag),
      RawExpr::Call(_) => FoldingResult::failed(self),
      RawExpr::Paren(paren) => paren.fold(target_type, diag),
      RawExpr::MemberAccess(member_access) => todo!(),
      RawExpr::Ternary(ternary) => ternary.fold(target_type, diag),
      RawExpr::SizeOf(size_of) => size_of.fold(target_type, diag),
      RawExpr::CStyleCast(cstyle_cast) => todo!(),
      RawExpr::ArraySubscript(array_subscript) => todo!(),
      RawExpr::CompoundLiteral(compound_literal) => todo!(),
      RawExpr::Variable(variable) => variable.fold(target_type, diag),
      RawExpr::ImplicitCast(implicit_cast) =>
        implicit_cast.fold(target_type, diag),
      // assignment expr is not considered constant expr in C, but in C++ it is.
      RawExpr::Assignment(_) => FoldingResult::failed(self),
    }
  }
}

impl Folding for Constant {
  type Folded = RawExpr;

  #[inline]
  fn fold(
    self,
    _target_type: &QualifiedType,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    FoldingResult::success(self.into())
  }
}

impl Folding for Unary {
  type Folded = RawExpr;

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    debug_assert!(
      self.operator.unary(),
      "not an unary operator! should not happen!"
    );

    let (folded_operand, level) =
      self.operand.fold(target_type, diag).destructure();

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
      // unary `+` is no-op for arithmetic types
      Operator::Plus => FoldingResult::success(folded_operand.into_raw()),
      // this happens after promotion, so no need to worry about smaller types
      Operator::Minus =>
        match &folded_operand.raw_expr().as_constant_unchecked().constant {
          ConstantLiteral::UInt(u) => {
            let (res, overflow) = u.overflowing_neg();
            if overflow {
              diag.add_warning(
                ArithmeticUnaryOpOverflow(
                  ConstantLiteral::UInt(*u),
                  Operator::Minus,
                ),
                self.span,
              )
            }
            // unsigned integer overflow is well-defined in C, so still a constant expression
            FoldingResult::success(ConstantLiteral::UInt(res).into())
          },
          ConstantLiteral::Int(i) => FoldingResult::success(
            ConstantLiteral::Int(i.wrapping_neg()).into(),
          ),
          ConstantLiteral::ULongLong(ull) => {
            let (res, overflow) = ull.overflowing_neg();
            if overflow {
              diag.add_warning(
                ArithmeticUnaryOpOverflow(
                  ConstantLiteral::ULongLong(*ull),
                  Operator::Minus,
                ),
                self.span,
              )
            }
            // ditto
            FoldingResult::success(ConstantLiteral::ULongLong(res).into())
          },
          ConstantLiteral::LongLong(ll) => FoldingResult::success(
            ConstantLiteral::LongLong(ll.wrapping_neg()).into(),
          ),
          ConstantLiteral::Float(f) =>
            FoldingResult::success(ConstantLiteral::Float(-f).into()),
          ConstantLiteral::Double(d) =>
            FoldingResult::success(ConstantLiteral::Double(-d).into()),
          // as-is!
          _ => contract_violation!(
            "the unary '-' applied to non-numeric constant or types that should be promoted: {:?}",
            folded_operand.raw_expr().as_constant_unchecked().constant
          ),
        },
      Operator::Not =>
        if folded_operand
          .raw_expr()
          .as_constant_unchecked()
          .constant
          .is_zero()
        {
          FoldingResult::success(ConstantLiteral::Int(1).into())
        } else {
          FoldingResult::success(ConstantLiteral::Int(0).into())
        },
      _ => todo!(),
    }
  }
}

impl Folding for Binary {
  type Folded = RawExpr;

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    debug_assert!(
      self.operator.binary(),
      "not a binary operator! should not happen!"
    );
    let (folded_lhs, lhs_level) =
      self.left.fold(target_type, diag).destructure();
    let (folded_rhs, rhs_level) =
      self.right.fold(target_type, diag).destructure();
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
      "type checker makes sure both sides have the same type via `ImplicitCast`!"
    );
    if folded_lhs.unqualified_type().is_integer() {
      Self::integral_folding(
        self.operator,
        folded_lhs,
        folded_rhs,
        self.span,
        diag,
      )
    } else if folded_lhs.unqualified_type().is_floating_point() {
      Self::floating_folding(
        self.operator,
        folded_lhs,
        folded_rhs,
        self.span,
        diag,
      )
    } else {
      todo!()
    }
  }
}
impl Binary {
  fn integral_folding(
    op: Operator,
    folded_lhs: Expression,
    folded_rhs: Expression,
    span: SourceSpan,
    diag: &impl Diagnosis,
  ) -> FoldingResult<<Self as Folding>::Folded> {
    use crate::underlying_type_of;

    let (lhs_expr, lhs_type, lhs_value_category) = folded_lhs.destructure();
    let (rhs_expr, rhs_type, rhs_value_category) = folded_rhs.destructure();
    let lhs = lhs_expr
      .into_constant()
      .expect("shall be constant")
      .constant;
    let rhs = rhs_expr
      .into_constant()
      .expect("shall be constant")
      .constant;

    macro_rules! arith {
      ($op:tt, $lhs:ident, $rhs:ident, $($variant:ident)*) => {
        match ($lhs, $rhs) {
          $( (ConstantLiteral::$variant(l), ConstantLiteral::$variant(r)) => {
            let (res, overflow) = <underlying_type_of!($variant)>::$op(l, r);
            if overflow {
              diag.add_warning(
                ArithmeticBinOpOverflow(
                  ConstantLiteral::$variant(l),
                  ConstantLiteral::$variant(r),
                  op,
                ),
                span,
              )
            }
            FoldingResult::success(ConstantLiteral::$variant(res).into())
          }, )*
          _ => {
            panic!("type checker ensures both sides have the same types! or unimplemented type");
          }
        }
      }
    }

    use ::std::ops::{BitAnd, BitOr, BitXor};
    macro_rules! bitwise {
      ($op:tt, $lhs:ident, $rhs:ident, $($variant:ident)*) => {
        match ($lhs, $rhs) {
          $( (ConstantLiteral::$variant(l), ConstantLiteral::$variant(r)) => {
            FoldingResult::success(ConstantLiteral::$variant(<underlying_type_of!($variant)>::$op(l, r)).into())
          }, )*
          _ => {
            panic!("type checker ensures both sides have the same types! or unimplemented type");
          }
        }
      }
    }

    use ::std::ops::{Shl, Shr};
    macro_rules! bitshift {
      ($op:tt, $lhs:ident, $rhs:ident, $($variant:ident)*) => {
        match ($lhs, $rhs) {
          $( (ConstantLiteral::$variant(l), ConstantLiteral::$variant(r)) => {
            FoldingResult::success(ConstantLiteral::$variant(<underlying_type_of!($variant)>::$op(l, r)).into())
          }, )*
          _ => {
            panic!("type checker ensures both sides have the same types! or unimplemented type");
          }
        }
      }
    }

    macro_rules! logical {
      ($op:tt, $lhs:ident, $rhs:ident, $($variant:ident)*) => {
        match ($lhs, $rhs) {
          $( (ConstantLiteral::$variant(l), ConstantLiteral::$variant(r)) => {
            FoldingResult::success(ConstantLiteral::Int(((l != 0) $op (r != 0)) as underlying_type_of!(Int)).into())
          }, )*
          _ => {
            panic!("type checker ensures both sides have the same types! or unimplemented type");
          }
        }
      }
    }

    macro_rules! rel {
      ($op:tt, $lhs:ident, $rhs:ident, $($variant:ident)*) => {
        match ($lhs, $rhs) {
          $( (ConstantLiteral::$variant(l), ConstantLiteral::$variant(r)) => {
            FoldingResult::success(ConstantLiteral::Int((l $op r) as underlying_type_of!(Int)).into())
          }, )*
          _ => {
            panic!("type checker ensures both sides have the same types! or unimplemented type");
          }
        }
      }
    }

    macro_rules! shorthand {
      ($select:ident, $op:tt) => {
        $select!(
          $op, lhs, rhs, Char UChar Short UShort Int UInt LongLong ULongLong
        )
      };
    }
    macro_rules! zerocheck {
      ($select:ident, $op:tt) => {
        if rhs.is_zero() {
          diag.add_warning(DivideByZero, span);
          // returns 0 as workaround
          FoldingResult::failed(ConstantLiteral::Int(0).into())
        } else {
          shorthand!($select, $op)
        }
      };
    }
    match op {
      Operator::Plus => shorthand!(arith, overflowing_add),
      Operator::Minus => shorthand!(arith, overflowing_sub),
      Operator::Star => shorthand!(arith, overflowing_mul),
      Operator::Slash => zerocheck!(arith, overflowing_div),
      Operator::Percent => zerocheck!(arith, overflowing_rem),

      Operator::Ampersand => shorthand!(bitwise, bitand),
      Operator::Pipe => shorthand!(bitwise, bitor),
      Operator::Caret => shorthand!(bitwise, bitxor),
      // had type checker ensures the rhs is non-negative?
      Operator::LeftShift => shorthand!(bitshift, shl),
      Operator::RightShift => shorthand!(bitshift, shr),

      Operator::And => shorthand!(logical, &&),
      Operator::Or => shorthand!(logical, ||),

      Operator::Less => shorthand!(rel, <),
      Operator::LessEqual => shorthand!(rel, <=),
      Operator::Greater => shorthand!(rel, >),
      Operator::GreaterEqual => shorthand!(rel, >=),
      Operator::EqualEqual => shorthand!(rel, ==),
      Operator::NotEqual => shorthand!(rel, !=),
      _ => contract_violation!(
        "not a binary operator! assignment op should be handled upstream, so does comma."
      ),
    }
  }

  fn floating_folding(
    op: Operator,
    folded_lhs: Expression,
    folded_rhs: Expression,
    span: SourceSpan,
    diag: &impl Diagnosis,
  ) -> FoldingResult<<Self as Folding>::Folded> {
    let (lhs_expr, lhs_type, lhs_value_category) = folded_lhs.destructure();
    let (rhs_expr, rhs_type, rhs_value_category) = folded_rhs.destructure();
    let lhs = lhs_expr
      .into_constant()
      .expect("shall be constant")
      .constant;
    let rhs = rhs_expr
      .into_constant()
      .expect("shall be constant")
      .constant;

    macro_rules! arith {
      ($op:tt, $lhs:ident, $rhs:ident, $($variant:ident)*) => {
        match ($lhs, $rhs) {
          $( (ConstantLiteral::$variant(l), ConstantLiteral::$variant(r)) => {
            let res = l $op r;

            // if inputs were finite but result is infinite  => overflow.
            if res.is_infinite() && l.is_finite() && r.is_finite() {
              diag.add_warning(
                ArithmeticBinOpOverflow(
                  ConstantLiteral::$variant(l),
                  ConstantLiteral::$variant(r),
                  op,
                ),
                span,
              )
            }
            // inf with inf
            if res.is_nan() {
              diag.add_warning(
                NotANumber(
                  ConstantLiteral::$variant(l),
                  ConstantLiteral::$variant(r),
                  op,
                ),
                span,
              )
            }

            FoldingResult::success(ConstantLiteral::$variant(res).into())
          }, )*
          _ => {
            panic!("type checker ensures both sides have the same types! or unimplemented type");
          }
        }
      }
    }
    use crate::underlying_type_of;
    macro_rules! logical {
      ($op:tt, $lhs:ident, $rhs:ident, $($variant:ident)*) => {
        match ($lhs, $rhs) {
          $( (ConstantLiteral::$variant(l), ConstantLiteral::$variant(r)) => {
            FoldingResult::success(ConstantLiteral::Int(((l != 0.0) $op (r != 0.0)) as underlying_type_of!(Int)).into())
          }, )*
          _ => {
            panic!("type checker ensures both sides have the same types! or unimplemented type");
          }
        }
      }
    }

    macro_rules! rel {
      ($op:tt, $lhs:ident, $rhs:ident, $($variant:ident)*) => {
        match ($lhs, $rhs) {
          $( (ConstantLiteral::$variant(l), ConstantLiteral::$variant(r)) => {
            FoldingResult::success(ConstantLiteral::Int((l $op r) as underlying_type_of!(Int)).into())
          }, )*
          _ => {
            panic!("type checker ensures both sides have the same types! or unimplemented type");
          }
        }
      }
    }

    macro_rules! shorthand {
      ($select:ident, $op:tt) => {
        $select!(
          $op, lhs, rhs, Float Double
        )
      };
    }
    match op {
      Operator::Plus => shorthand!(arith, +),
      Operator::Minus => shorthand!(arith, -),
      Operator::Star => shorthand!(arith, *),
      // Division by zero for floating-point yields inf or nan, so no need to warn.
      Operator::Slash => shorthand!(arith, /),
      _ => contract_violation!(
        "not a binary operator or bin-op but cannot be applied to floating-point! assignment op should be handled upstream, so does comma."
      ),

      Operator::And => shorthand!(logical, &&),
      Operator::Or => shorthand!(logical, ||),

      Operator::Less => shorthand!(rel, <),
      Operator::LessEqual => shorthand!(rel, <=),
      Operator::Greater => shorthand!(rel, >),
      Operator::GreaterEqual => shorthand!(rel, >=),
      Operator::EqualEqual => shorthand!(rel, ==),
      Operator::NotEqual => shorthand!(rel, !=),
    }
  }
}

impl Folding for Ternary {
  type Folded = RawExpr;

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    debug_assert!(
      self
        .then_expr
        .qualified_type()
        .compatible_with(self.else_expr.qualified_type()),
      "type checker ensures both branches have compatible types!"
    );
    let (folded_cond, cond_level) =
      self.condition.fold(target_type, diag).destructure();
    let (folded_then, then_level) =
      self.then_expr.fold(target_type, diag).destructure();
    let (folded_else, else_level) =
      self.else_expr.fold(target_type, diag).destructure();

    if cond_level != Success {
      return FoldingResult::new(
        Self {
          condition: folded_cond.into(),
          then_expr: folded_then.into(),
          else_expr: folded_else.into(),
          ..self
        }
        .into(),
        FoldingLevel::merge_n([cond_level, then_level, else_level]),
      );
    }

    if folded_cond.raw_expr().as_constant_unchecked().is_zero() {
      FoldingResult::new(
        folded_else.into_raw(),
        FoldingLevel::merge(cond_level, else_level),
      )
    } else {
      FoldingResult::new(
        folded_then.into_raw(),
        FoldingLevel::merge(cond_level, then_level),
      )
    }
  }
}

impl Folding for SizeOf {
  type Folded = RawExpr;

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
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
      SizeOfKind::Expression(expr) => expr.fold(target_type,diag).map(|expr|{
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

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    if self.name.borrow().is_constexpr() {
      diag.add_error(
        UnsupportedFeature("constexpr variable not implemented".to_string()),
        self.span,
      );
      FoldingResult::failed(self.into())
    } else {
      FoldingResult::failed(self.into())
    }
  }
}

impl Folding for Paren {
  type Folded = RawExpr;

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    self
      .expr
      .fold(target_type, diag)
      .map(|expr| expr.into_raw())
  }
}

impl Folding for ImplicitCast {
  type Folded = RawExpr;

  fn fold(
    self,
    target_type: &QualifiedType,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Self::Folded> {
    let (folded_expr, level) = self.expr.fold(target_type, diag).destructure();
    if level != Success {
      return FoldingResult::failed(folded_expr.into_raw());
    }
    let (raw_expr, expr_type, _value_category) = folded_expr.destructure();

    use CastType::*;
    match self.cast_type {
      Noop | ToVoid | LValueToRValue | BitCast =>
        FoldingResult::success(raw_expr),
      ArrayToPointerDecay => todo!("address constant"),
      FunctionToPointerDecay => todo!("address constant"),
      NullptrToPointer => match target_type.unqualified_type() {
        // FIXME: i dont think this is correct
        Type::Pointer(p) => FoldingResult::success(RawExpr::Constant(
          ConstantLiteral::ULongLong(0).into(),
        )),
        _ => contract_violation!("unreachable"),
      },
      IntegralCast => match raw_expr {
        RawExpr::Constant(c) => {
          let target_primitive =
            target_type.unqualified_type().as_primitive_unchecked();
          let expr_primitive =
            expr_type.unqualified_type().as_primitive_unchecked();
          if target_primitive.integer_rank() < expr_primitive.integer_rank() {
            diag.add_warning(
              CastDown(expr_type.clone().into(), target_type.clone().into()),
              self.span,
            )
          }
          todo!()
        },
        _ => contract_violation!("unreachable"),
      },
      IntegralToFloating => todo!(),
      IntegralToBoolean | FloatingToBoolean => match raw_expr {
        RawExpr::Constant(c) =>
          if c.constant.is_zero() {
            FoldingResult::success(RawExpr::Constant(
              ConstantLiteral::Int(0).into(),
            ))
          } else {
            FoldingResult::success(RawExpr::Constant(
              ConstantLiteral::Int(1).into(),
            ))
          },
        _ => contract_violation!("unreachable"),
      },
      FloatingCast => todo!(),
      FloatingToIntegral => todo!(),
      IntegralToPointer => contract_violation!(
        "no such implicit cast in C -- only for explicit casts!"
      ),
      PointerToIntegral => todo!(),
      PointerToBoolean => FoldingResult::failed(raw_expr),
      NullptrToIntegral => match target_type.unqualified_type() {
        Type::Primitive(p) => match p {
          Primitive::Int => FoldingResult::success(RawExpr::Constant(
            ConstantLiteral::Int(0).into(),
          )),
          Primitive::Long => FoldingResult::success(RawExpr::Constant(
            ConstantLiteral::LongLong(0).into(),
          )),
          Primitive::LongLong => FoldingResult::success(RawExpr::Constant(
            ConstantLiteral::LongLong(0).into(),
          )),
          Primitive::UInt => FoldingResult::success(RawExpr::Constant(
            ConstantLiteral::UInt(0).into(),
          )),
          Primitive::ULong => FoldingResult::success(RawExpr::Constant(
            ConstantLiteral::ULongLong(0).into(),
          )),
          Primitive::ULongLong => FoldingResult::success(RawExpr::Constant(
            ConstantLiteral::ULongLong(0).into(),
          )),
          _ => contract_violation!("unreachable"),
        },
        _ => contract_violation!("unreachable"),
      },
      NullptrToBoolean => FoldingResult::success(RawExpr::Constant(
        ConstantLiteral::Bool(false).into(),
      )),
    }
  }
}
