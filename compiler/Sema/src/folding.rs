use ::rcc_adt::{Floating, Integral};
use ::rcc_ast::types::{CastType, Compatibility, QualifiedType, TypeInfo};
use ::rcc_shared::{
  DiagData::*, Diagnosis, Operator, OperatorCategory, SourceSpan,
};
use ::rcc_utils::{IntoWith, RefEq, contract_assert, contract_violation};

use super::expression::{
  ArraySubscript, Binary, CStyleCast, Call, CompoundLiteral, Constant,
  ConstantLiteral as CL, Empty, Expression, ImplicitCast, MemberAccess, Paren,
  RawExpr, SizeOf, SizeOfKind, Ternary, Unary, ValueCategory, Variable,
};
use crate::expression::CompoundAssign;

#[derive(Debug)]
pub enum FoldingResult<T> {
  Success(T),
  Failure(T),
}

impl<T> FoldingResult<T> {
  #[inline]
  fn map<U>(self, f: impl FnOnce(T) -> U) -> FoldingResult<U> {
    match self {
      Self::Success(v) => FoldingResult::Success(f(v)),
      Self::Failure(v) => FoldingResult::Failure(f(v)),
    }
  }

  #[inline]
  pub fn inspect_error<F>(self, f: F) -> Self
  where
    F: FnOnce(&T),
  {
    if let Self::Failure(v) = &self {
      f(v)
    }
    self
  }

  /// This function **won't** panic, and always returns the inner value regardless of success or failure.
  #[inline]
  pub fn take(self) -> T {
    match self {
      Self::Failure(v) | Self::Success(v) => v,
    }
  }

  #[inline]
  pub fn transform<U>(self, f: impl FnOnce(T) -> U) -> U {
    match self {
      Self::Success(v) | Self::Failure(v) => f(v),
    }
  }
}

/// Folding trait for constant expression evaluation
pub trait Folding<'c> {
  /// This serves as a never-fail folding mechanism,
  /// all errors and warnings shall be handled via `diag` parameter.
  /// [`Operational`](crate::diagnosis::Operational) is recommended.
  ///
  /// If folding is not possible, return self unchanged.
  /// So it may end up being a no-op, partial-fold, or full-fold.
  ///
  /// If [`Diagnosis`] is not required, use [`NoOp`](crate::diagnosis::NoOp) as the dummy parameter.
  #[must_use]
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>>;
}

use FoldingResult::{Failure, Success};

impl<'c> Expression<'c> {
  #[inline(always)]
  pub fn fold(
    self,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    let (raw_expr, expr_type, value_category) = self.destructure();
    raw_expr.fold(expr_type, value_category, diag)
  }
}
impl<'c> Folding<'c> for RawExpr<'c> {
  #[inline]
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.fold(target_type, value_category, diag) =>
      Empty Constant Unary Binary Call Paren MemberAccess Ternary SizeOf
      CStyleCast ArraySubscript CompoundLiteral Variable ImplicitCast CompoundAssign
    )
  }
}
impl<'c> Folding<'c> for Empty {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}

impl<'c> Folding<'c> for Call<'c> {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}

impl<'c> Folding<'c> for MemberAccess<'c> {
  fn fold(
    self,
    _target_type: QualifiedType<'c>,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    todo!()
  }
}
impl<'c> Folding<'c> for CStyleCast<'c> {
  fn fold(
    self,
    _target_type: QualifiedType<'c>,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    todo!()
  }
}

impl<'c> Folding<'c> for ArraySubscript<'c> {
  /// always fails folding in C, unlike C++.
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}

impl<'c> Folding<'c> for CompoundLiteral {
  fn fold(
    self,
    _target_type: QualifiedType<'c>,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    todo!()
  }
}

impl<'c> Folding<'c> for Constant<'c> {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    Success(Expression::new(self.into(), target_type, value_category))
  }
}

impl<'c> Folding<'c> for Unary<'c> {
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    debug_assert!(
      self.operator.unary(),
      "not an unary operator! should not happen!"
    );

    let folded_operand = match self.operand.fold(diag) {
      Success(folded) => folded,
      Failure(original) =>
        return Failure(Expression::new(
          Self {
            operand: original.into(),
            ..self
          }
          .into(),
          target_type,
          value_category,
        )),
    };
    use OperatorCategory::*;

    contract_assert!(
      folded_operand.raw_expr().is_constant(),
      "only implemented for constant var of constant eval"
    );
    match folded_operand.raw_expr().as_constant_unchecked().inner {
      ::rcc_shared::Constant::Integral(operand) =>
        Integral::handle_unary_op(self.operator, operand, self.span, diag)
          .map(Into::into)
          .map(|constant: CL| {
            Expression::new(
              constant.into_with(self.span),
              target_type,
              value_category,
            )
          }),
      ::rcc_shared::Constant::Floating(operand) =>
        match self.operator.category() {
          Arithmetic => Floating::handle_unary_arith_op(
            self.operator,
            operand,
            self.span,
            diag,
          )
          .map(Into::into),
          Logical => Floating::handle_unary_order_op(
            self.operator,
            operand,
            self.span,
            diag,
          )
          .map(Into::into),
          _ => contract_violation!(
            "not a unary operator or un-op but cannot be applied to floating!"
          ),
        }
        .map(|constant: CL| {
          Expression::new(
            constant.into_with(self.span),
            target_type,
            value_category,
          )
        }),
      _ => unimplemented!(),
    }
  }
}

impl<'c> Folding<'c> for Binary<'c> {
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    debug_assert!(
      self.operator.binary(),
      "not a binary operator! should not happen!"
    );
    // logical && and || are short-circuit evaluated,
    // so    `static const int j = 0 && x;` would pass constant folding,
    // while `static const int j = 0 || x;` would not.
    let fl = self.left.fold(diag);
    let fr = self.right.fold(diag);

    let f = |left, right| {
      Expression::new(
        Self {
          left,
          right,
          ..self
        }
        .into(),
        target_type,
        value_category,
      )
    };

    let (folded_lhs, folded_rhs) = match (fl, fr) {
      (Success(left), Success(right)) => (left, right),
      (Success(left), Failure(_))
        if self.operator == Operator::And
          && left.raw_expr().as_constant_unchecked().is_zero() =>
        return Success(left),

      (Success(left), Failure(right)) if self.operator == Operator::And =>
        return Failure(f(left.into(), right.into())),

      (Success(left), Failure(_))
        if self.operator == Operator::Or
          && left.raw_expr().as_constant_unchecked().is_not_zero() =>
        return Success(left),
      (Success(left), Failure(right)) if self.operator == Operator::Or =>
        return Failure(f(left.into(), right.into())),

      (fl, fr) => return Failure(f(fl.take().into(), fr.take().into())),
    };
    if self.operator == Operator::Comma {
      diag.add_warning(LeftCommaNoEffect, self.span);
      return Success(Expression::new(
        folded_rhs.destructure().0,
        target_type,
        value_category,
      ));
    }
    assert!(
      folded_lhs.raw_expr().is_constant()
        && folded_rhs.raw_expr().is_constant(),
      "only implemented for constant var of constant eval"
    );
    assert!(
      RefEq::ref_eq(
        folded_lhs.unqualified_type(),
        folded_rhs.unqualified_type()
      ),
      "type checker makes sure both sides have the same type via \
       `ImplicitCast`! {:#?} vs {:#?}, op {:#?}",
      folded_lhs.qualified_type(),
      folded_rhs.qualified_type(),
      self.operator
    );
    let (lhs_expr, lhs_type, lhs_value_category) = folded_lhs.destructure();
    let (rhs_expr, rhs_type, rhs_value_category) = folded_rhs.destructure();

    assert!(
      RefEq::ref_eq(lhs_type.unqualified_type, rhs_type.unqualified_type),
      "type checker ensures both sides have the same types!"
    );

    assert!(
      lhs_value_category == ValueCategory::RValue
        && rhs_value_category == ValueCategory::RValue,
      "type checker ensures both sides are rvalues! 
      Assignment is handled at start of this function."
    );

    let lhs = lhs_expr.into_constant_unchecked().inner;
    let rhs = rhs_expr.into_constant_unchecked().inner;

    use OperatorCategory::*;
    match (lhs, rhs) {
      (
        ::rcc_shared::Constant::Integral(lhs),
        ::rcc_shared::Constant::Integral(rhs),
      ) => Integral::handle_binary_op(self.operator, lhs, rhs, self.span, diag)
        .map(Into::into),
      (
        ::rcc_shared::Constant::Floating(lhs),
        ::rcc_shared::Constant::Floating(rhs),
      ) => match self.operator.category() {
        Logical | Relational => Floating::handle_binary_order_op(
          self.operator,
          lhs,
          rhs,
          self.span,
          diag,
        )
        .map(Into::into),
        Arithmetic => Floating::handle_binary_arith_op(
          self.operator,
          lhs,
          rhs,
          self.span,
          diag,
        )
        .map(Into::into),
        _ => contract_violation!(
          "not a binary operator or bin-op but cannot be applied to floating!"
        ),
      },
      (
        ::rcc_shared::Constant::String(_),
        ::rcc_shared::Constant::String(_),
      ) => contract_violation!("can we reach here?"),
      (
        ::rcc_shared::Constant::Nullptr(),
        ::rcc_shared::Constant::Nullptr(),
      ) => contract_violation!("can we reach here?"),
      _ => contract_violation!(
        "type checker ensures both sides have the same types! or \
         unimplemented type"
      ),
    }
    .map(|constant: CL| {
      Expression::new(
        constant.into_with(self.span),
        target_type,
        value_category,
      )
    })
  }
}
impl<'c> Folding<'c> for Ternary<'c> {
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    debug_assert!(
      self
        .then_expr
        .as_deref()
        .expect("?: unimplemented")
        .qualified_type()
        .compatible_with(self.else_expr.qualified_type()),
      "type checker ensures both branches have compatible types!"
    );
    let fc = self.condition.fold(diag);
    match fc {
      Success(folded_condition) => {
        match folded_condition
          .raw_expr()
          .as_constant_unchecked()
          .inner
          .is_zero()
        {
          true => self.else_expr.fold(diag),
          false => self.then_expr.expect("?: unimplemented").fold(diag),
        }
      },
      Failure(_) => fc.map(|folded_condition| {
        Expression::new(
          Self {
            condition: folded_condition.into(),
            then_expr: Some(
              self
                .then_expr
                .expect("?: unimplemented")
                .fold(diag)
                .take()
                .into(),
            ),
            else_expr: self.else_expr.fold(diag).take().into(),
            ..self
          }
          .into(),
          target_type,
          value_category,
        )
      }),
    }
  }
}

impl<'c> Folding<'c> for SizeOf<'c> {
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    match self.sizeof {
      SizeOfKind::Type(qualified_type) => if qualified_type.size() > 0 {
        Success(Integral::from_uintptr(qualified_type.size()))
      } else {
        Failure(Integral::from_uintptr(0))
      }
      .map(Into::into)
      .map(|constant: CL| {
        Expression::new(
          constant.into_with(self.span),
          target_type,
          value_category,
        )
      }),
      SizeOfKind::Expression(expr) => expr.fold(diag),
    }
  }
}

impl<'c> Folding<'c> for Variable<'c> {
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    if self.name.borrow().is_constexpr() {
      diag.add_error(
        UnsupportedFeature("constexpr variable not implemented".to_string()),
        self.span,
      );
      Failure(Expression::new(self.into(), target_type, value_category))
    } else {
      Failure(Expression::new(self.into(), target_type, value_category))
    }
  }
}

impl<'c> Folding<'c> for Paren<'c> {
  fn fold(
    self,
    _target_type: QualifiedType<'c>,
    _value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    self.expr.fold(diag)
  }
}

impl<'c> Folding<'c> for ImplicitCast<'c> {
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    let folded_expr = match self.expr.fold(diag) {
      Success(folded) => folded,
      Failure(original) =>
        return Failure(Expression::new(
          Self {
            expr: original.into(),
            ..self
          }
          .into(),
          target_type,
          value_category,
        )),
    };
    let (raw_expr, expr_type, _value_category) = folded_expr.destructure();

    use CastType::*;
    match self.cast_type {
      Noop | ToVoid | LValueToRValue | BitCast => Success(raw_expr),
      PointerToIntegral | PointerToBoolean => Failure(raw_expr),
      ArrayToPointerDecay => todo!("address constant"),
      FunctionToPointerDecay => todo!("address constant"),
      NullptrToPointer => Success(raw_expr),
      IntegralCast => match raw_expr {
        RawExpr::Constant(c) => {
          let target_primitive =
            target_type.unqualified_type.as_primitive_unchecked();
          let expr_primitive =
            expr_type.unqualified_type.as_primitive_unchecked();
          if target_primitive.size() < expr_primitive.size()
            && !target_primitive.is_void()
            && !target_primitive.is_bool()
          {
            diag.add_warning(
              CastDown(expr_type.to_string(), target_type.to_string()),
              self.span,
            )
          }
          Success(
            c.inner
              .as_integral_unchecked()
              .cast(
                target_primitive.size_bits() as u8,
                target_primitive.is_signed().into(),
              )
              .into(),
          )
        }
        .map(|c: CL| c.into_with(self.span)),
        _ => contract_violation!("unreachable: {:?}", raw_expr),
      },
      // integral are promoted previously.
      IntegralToFloating => Success(
        raw_expr.into_constant_unchecked().inner.to_floating(
          target_type
            .unqualified_type
            .as_primitive_unchecked()
            .floating_format(),
        ),
      )
      .map(|c: CL| c.into_with(self.span)),
      IntegralToBoolean | FloatingToBoolean => Success(
        raw_expr
          .into_constant_unchecked()
          .inner
          .to_boolean()
          .into_with(self.span),
      ),
      FloatingCast => Success(
        raw_expr
          .into_constant_unchecked()
          .inner
          .as_floating_unchecked()
          .cast(
            target_type
              .unqualified_type
              .as_primitive_unchecked()
              .floating_format(),
          ),
      )
      .map(Into::into)
      .map(|c: CL| c.into_with(self.span)),
      FloatingToIntegral => Success(
        raw_expr.into_constant_unchecked().inner.to_integral(
          target_type.as_primitive_unchecked().integer_width(),
          target_type
            .as_primitive_unchecked()
            .is_signed_integer()
            .into(),
        ),
      )
      .map(|c: CL| c.into_with(self.span)),
      IntegralToPointer => contract_violation!(
        "no such implicit cast in C -- only for explicit casts!"
      ),
    }
    .map(|raw_expr| Expression::new(raw_expr, target_type, value_category))
  }
}
impl<'c> Folding<'c> for CompoundAssign<'c> {
  /// should always fail, but anyways
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'c>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Expression<'c>> {
    // let (raw_binary, inner_ty, inner_vc) = self.intermediate.destructure();
    // let res = raw_binary
    //   .as_binary_unchecked()
    //   .fold(inner_ty, inner_vc, diag);

    Failure(Expression::new(self.into(), target_type, value_category))
  }
}
mod private {
  use super::{Floating, Integral};
  pub trait Sealed {}
  impl Sealed for Integral {}
  impl Sealed for Floating {}
}
trait IntegralExt: private::Sealed {
  fn handle_binary_op<'c>(
    op: Operator,
    lhs: Self,
    rhs: Self,
    span: SourceSpan,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Self>
  where
    Self: Sized;

  fn handle_unary_op<'c>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Self>
  where
    Self: Sized;
}

impl IntegralExt for Integral {
  fn handle_binary_op<'c>(
    op: Operator,
    lhs: Self,
    rhs: Self,
    span: SourceSpan,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Self> {
    debug_assert!(op.binary());
    debug_assert_eq!(lhs.width(), rhs.width());
    debug_assert_eq!(lhs.signedness(), rhs.signedness());

    macro_rules! arith {
      ($func:ident, $op:expr) => {{
        let (result, overflow) = lhs.$func(rhs);
        if overflow {
          diag.add_warning(
            ArithmeticBinOpOverflow((lhs.into(), rhs.into(), $op).into()),
            span,
          );
        }
        Success(result)
      }};
    }
    macro_rules! div_zero {
      ($func:ident) => {{
        match lhs.$func(rhs) {
          Some(result) => Success(result),
          None => {
            diag.add_error(DivideByZero, span);
            Failure(
              Integral::new(0, lhs.width() as u8, lhs.signedness()).into(),
            )
          },
        }
      }};
    }
    macro_rules! logical_misuse_warn {
    ($op_sym:tt, $op_variant:ident, $suggest:ident) => {{
        diag.add_warning(
            LogicalOpMisuse($op_variant, $suggest.into()),
            span,
        );
        Success(Self::from_bool(!lhs.is_zero() $op_sym !rhs.is_zero()))
    }}
    }
    use Operator::*;
    match op {
      Plus => arith!(overflowing_add, Plus),
      Minus => arith!(overflowing_sub, Minus),
      Star => arith!(overflowing_mul, Star),

      Slash => div_zero!(checked_div),
      Percent => div_zero!(checked_rem),

      And => logical_misuse_warn!(&&, And, Ampersand),
      Or => logical_misuse_warn!(||, Or, Pipe),

      Less => Success(Self::from_bool(lhs < rhs)),
      LessEqual => Success(Self::from_bool(lhs <= rhs)),
      Greater => Success(Self::from_bool(lhs > rhs)),
      GreaterEqual => Success(Self::from_bool(lhs >= rhs)),
      EqualEqual => Success(Self::from_bool(lhs == rhs)),
      NotEqual => Success(Self::from_bool(lhs != rhs)),

      Ampersand => Success(lhs & rhs),
      Pipe => Success(lhs | rhs),
      Caret => Success(lhs ^ rhs),
      _ => contract_violation!(
        "not a binary operator or bin-op but cannot be applied to integral! \
         assignment op should be handled upstream, so does comma."
      ),
    }
  }

  fn handle_unary_op<'c>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Self> {
    debug_assert!(operator.unary());
    use Operator::*;
    match operator {
      Plus => Success(operand),
      Minus => Success(-operand),
      Not => Success(Self::from_bool(operand.is_zero())),
      Tilde => Success(!operand), // `!` is bitwise NOT here; rust does not have `~` operator
      Star | Ampersand | PlusPlus | MinusMinus =>
        contract_violation!("unary operator not applicable to integral!"),
      _ => unreachable!(),
    }
  }
}

trait FloatingExt: private::Sealed {
  fn handle_binary_arith_op<'c>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Self>
  where
    Self: Sized;

  fn handle_binary_order_op<'c>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Integral>;

  fn handle_unary_arith_op<'c>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Self>
  where
    Self: Sized;

  fn handle_unary_order_op<'c>(
    operator: Operator,
    operand: Self,
    span: SourceSpan,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Integral>;
}

impl FloatingExt for Floating {
  fn handle_binary_arith_op<'c>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Self> {
    use Operator::*;
    debug_assert!(
      op.binary() && op != Percent,
      "Tried to perform unary operation in binary op handler"
    );
    macro_rules! arith {
      ($op:tt) => {{
        let res = lhs $op rhs;
        if lhs.is_finite() && rhs.is_finite() && res.is_infinite() {
          diag.add_warning(
            ArithmeticBinOpOverflow((lhs.into(), rhs.into(), op).into()),
            span,
          );
        }
        Success(res)
      }};
    }

    match op {
      Plus => arith!(+),
      Minus => arith!(-),
      Star => arith!(*),
      Slash => arith!(/),
      _ => contract_violation!(
        "not a binary arithmetic operator but cannot be applied to floating! \
         assignment op should be handled upstream, so does comma."
      ),
    }
  }

  fn handle_binary_order_op<'c>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Integral> {
    use OperatorCategory::*;
    debug_assert!(
      matches!(op.category(), Logical | Relational),
      "Tried to perform unary operation in binary op handler"
    );
    macro_rules! logical_misuse_warn {
    ($op_sym:tt, $op_variant:ident, $suggest:ident) => {{
        diag.add_warning(
            LogicalOpMisuse($op_variant, $suggest.into()),
            span,
        );
        Success(Integral::from_bool(!lhs.is_zero() $op_sym !rhs.is_zero()))
    }}
    }
    use Operator::*;
    match op {
      And => logical_misuse_warn!(&&, And, Ampersand),
      Or => logical_misuse_warn!(||, Or, Pipe),

      Less => Success(Integral::from_bool(lhs < rhs)),
      LessEqual => Success(Integral::from_bool(lhs <= rhs)),
      Greater => Success(Integral::from_bool(lhs > rhs)),
      GreaterEqual => Success(Integral::from_bool(lhs >= rhs)),
      EqualEqual => Success(Integral::from_bool(lhs == rhs)),
      NotEqual => Success(Integral::from_bool(lhs != rhs)),
      _ => contract_violation!(
        "not a binary operator or bin-op but cannot be applied to floating! \
         {op}"
      ),
    }
  }

  fn handle_unary_arith_op<'c>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    _diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Self> {
    debug_assert!(operator.unary());
    use Operator::*;
    match operator {
      Plus => Success(operand),
      Minus => Success(-operand),
      Not | Tilde | Star | Ampersand | PlusPlus | MinusMinus =>
        contract_violation!(
          "unary operator not applicable to floating! should be handled \
           upstream"
        ),
      _ => unreachable!(),
    }
  }

  fn handle_unary_order_op<'c>(
    operator: Operator,
    operand: Self,
    span: SourceSpan,
    diag: &impl Diagnosis<'c>,
  ) -> FoldingResult<Integral> {
    debug_assert!(operator.unary());
    use Operator::*;
    match operator {
      Not => {
        diag.add_warning(LogicalOpMisuse(Not, None), span);
        Success(Integral::from_bool(operand.is_zero()))
      },
      And | Or | Less | LessEqual | Greater | GreaterEqual | EqualEqual
      | NotEqual | Star | Ampersand | PlusPlus | MinusMinus =>
        contract_violation!(
          "unary operator not applicable to floating! should be handled \
           upstream"
        ),
      _ => unreachable!(),
    }
  }
}
