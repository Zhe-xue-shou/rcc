use ::rcc_utils::{IntoWith, contract_assert, contract_violation};

use super::expression::{
  ArraySubscript, Assignment, Binary, CStyleCast, Call, CompoundLiteral,
  Constant, ConstantLiteral as CL, Empty, Expression, ImplicitCast,
  MemberAccess, Paren, RawExpr, SizeOf, SizeOfKind, Ternary, Unary,
  ValueCategory, Variable,
};
use crate::{
  common::{Floating, Integral, Operator, OperatorCategory, SourceSpan},
  diagnosis::{DiagData::*, Diagnosis},
  types::{CastType, Compatibility, QualifiedType, Type, TypeInfo},
};

#[derive(Debug)]
pub enum FoldingResult<T> {
  Success(T),
  Failure(T),
}
impl<T> ::std::ops::FromResidual for FoldingResult<T> {
  #[inline]
  fn from_residual(residual: <Self as ::std::ops::Try>::Residual) -> Self {
    residual
  }
}

impl<T> ::std::ops::Try for FoldingResult<T> {
  type Output = T;
  type Residual = FoldingResult<T>;

  #[inline]
  fn from_output(output: Self::Output) -> Self {
    Self::Success(output)
  }

  #[inline]
  fn branch(self) -> ::std::ops::ControlFlow<Self::Residual, Self::Output> {
    match self {
      Self::Success(v) => ::std::ops::ControlFlow::Continue(v),
      _ => ::std::ops::ControlFlow::Break(self),
    }
  }
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
  pub fn unwrap(self) -> T {
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
pub trait Folding<'context> {
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
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>>;
}

use FoldingResult::{Failure, Success};

impl<'context> Expression<'context> {
  #[inline(always)]
  pub(super) fn fold(
    self,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    let (raw_expr, expr_type, value_category) = self.destructure();
    raw_expr.fold(expr_type, value_category, diag)
  }
}
impl<'context> Folding<'context> for RawExpr<'context> {
  #[inline]
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    ::rcc_utils::static_dispatch!(
      self.fold(target_type, value_category, diag),
      Empty Constant Unary Binary Call Paren MemberAccess Ternary SizeOf CStyleCast ArraySubscript CompoundLiteral Variable ImplicitCast Assignment
    )
  }
}
impl<'context> Folding<'context> for Empty {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}

impl<'context> Folding<'context> for Call<'context> {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}

impl<'context> Folding<'context> for MemberAccess<'context> {
  fn fold(
    self,
    _target_type: QualifiedType<'context>,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    todo!()
  }
}
impl<'context> Folding<'context> for CStyleCast<'context> {
  fn fold(
    self,
    _target_type: QualifiedType<'context>,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    todo!()
  }
}

impl<'context> Folding<'context> for ArraySubscript<'context> {
  /// always fails folding in C, unlike C++.
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}

impl<'context> Folding<'context> for CompoundLiteral {
  fn fold(
    self,
    _target_type: QualifiedType<'context>,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    todo!()
  }
}

impl<'context> Folding<'context> for Assignment<'context> {
  /// assignment expr is not considered constant expr in C, but in C++ it is.
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}
impl<'context> Folding<'context> for Constant<'context> {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    _diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    Success(Expression::new(self.into(), target_type, value_category))
  }
}

impl<'context> Folding<'context> for Unary<'context> {
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    debug_assert!(
      self.operator.unary(),
      "not an unary operator! should not happen!"
    );

    let folded_operand = self.operand.fold(diag)?;
    use OperatorCategory::*;

    contract_assert!(
      folded_operand.raw_expr().is_constant(),
      "only implemented for constant var of constant eval"
    );
    match folded_operand.raw_expr().as_constant_unchecked().value {
      crate::types::Constant::Integral(operand) =>
        Integral::handle_unary_op(self.operator, operand, self.span, diag)
          .map(Into::into)
          .map(|constant: CL| {
            Expression::new(
              constant.into_with(self.span),
              target_type,
              value_category,
            )
          }),
      crate::types::Constant::Floating(operand) =>
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

impl<'context> Folding<'context> for Binary<'context> {
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    debug_assert!(
      self.operator.binary(),
      "not a binary operator! should not happen!"
    );
    // TODO: logical && and || should be short-circuit evaluated,
    // so that `static const int j = 0 && x;` would pass constant folding,
    // while `static const int j = 0 || x;` would not.
    let fl = self.left.fold(diag);
    let fr = self.right.fold(diag);
    let (folded_lhs, folded_rhs) =
      if matches!(fl, Success(_)) && matches!(fr, Success(_)) {
        (fl.unwrap(), fr.unwrap())
      } else {
        return Failure(Expression::new(
          Self {
            left: fl.unwrap().into(),
            right: fr.unwrap().into(),
            ..self
          }
          .into(),
          target_type,
          value_category,
        ));
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
      ::std::ptr::eq(
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
      ::std::ptr::eq(lhs_type.unqualified_type, rhs_type.unqualified_type),
      "type checker ensures both sides have the same types!"
    );

    assert!(
      lhs_value_category == ValueCategory::RValue
        && rhs_value_category == ValueCategory::RValue,
      "type checker ensures both sides are rvalues!"
    );

    let lhs = lhs_expr.into_constant().expect("shall be constant").value;
    let rhs = rhs_expr.into_constant().expect("shall be constant").value;

    use OperatorCategory::*;
    match (lhs, rhs) {
      (
        crate::types::Constant::Integral(lhs),
        crate::types::Constant::Integral(rhs),
      ) => Integral::handle_binary_op(self.operator, lhs, rhs, self.span, diag)
        .map(Into::into),
      (
        crate::types::Constant::Floating(lhs),
        crate::types::Constant::Floating(rhs),
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
        crate::types::Constant::String(_),
        crate::types::Constant::String(_),
      ) => contract_violation!("can we reach here?"),
      (
        crate::types::Constant::Nullptr(_),
        crate::types::Constant::Nullptr(_),
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
impl<'context> Folding<'context> for Ternary<'context> {
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    debug_assert!(
      self
        .then_expr
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
          .value
          .is_zero()
        {
          true => self.else_expr.fold(diag),
          false => self.then_expr.fold(diag),
        }
      },
      Failure(_) => fc.map(|folded_condition| {
        Expression::new(
          Self {
            condition: folded_condition.into(),
            then_expr: self.then_expr.fold(diag).unwrap().into(),
            else_expr: self.else_expr.fold(diag).unwrap().into(),
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

impl<'context> Folding<'context> for SizeOf<'context> {
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
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

impl<'context> Folding<'context> for Variable<'context> {
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
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

impl<'context> Folding<'context> for Paren<'context> {
  fn fold(
    self,
    _target_type: QualifiedType<'context>,
    _value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    self.expr.fold(diag)
  }
}

impl<'context> Folding<'context> for ImplicitCast<'context> {
  fn fold(
    self,
    target_type: QualifiedType<'context>,
    value_category: ValueCategory,
    diag: &impl Diagnosis<'context>,
  ) -> FoldingResult<Expression<'context>> {
    let folded_expr = self.expr.fold(diag)?;
    let (raw_expr, expr_type, _value_category) = folded_expr.destructure();

    use CastType::*;
    match self.cast_type {
      Noop | ToVoid | LValueToRValue | BitCast => Success(raw_expr),
      PointerToIntegral | PointerToBoolean => Failure(raw_expr),
      ArrayToPointerDecay => todo!("address constant"),
      FunctionToPointerDecay => todo!("address constant"),
      NullptrToPointer => match *target_type.unqualified_type {
        Type::Pointer(_) => todo!(),
        _ => contract_violation!("unreachable"),
      },
      IntegralCast => match raw_expr {
        RawExpr::Constant(c) => {
          let target_primitive =
            target_type.unqualified_type.as_primitive_unchecked();
          let expr_primitive =
            expr_type.unqualified_type.as_primitive_unchecked();
          if target_primitive.size() < expr_primitive.size() {
            diag.add_warning(CastDown(expr_type, target_type), self.span)
          }
          Success(
            c.value
              .as_integral_unchecked()
              .cast(
                target_primitive.size() as u8,
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
        raw_expr.into_constant_unchecked().value.to_floating(
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
          .value
          .to_boolean()
          .into_with(self.span),
      ),
      FloatingCast => Success(
        raw_expr
          .into_constant_unchecked()
          .value
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
        raw_expr.into_constant_unchecked().value.to_integral(
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
      NullptrToIntegral =>
        match *target_type.unqualified_type {
          Type::Primitive(p) => Success(
            Integral::new(0, p.size() as u8, p.is_signed().into()).into(),
          ),
          _ => contract_violation!("unreachable"),
        }
        .map(|c: CL| c.into_with(self.span)),
      NullptrToBoolean => Success(Integral::from_bool(false).into())
        .map(|c: CL| c.into_with(self.span)),
    }
    .map(|raw_expr| Expression::new(raw_expr, target_type, value_category))
  }
}
impl Integral {
  pub fn handle_binary_op<'context>(
    op: Operator,
    lhs: Self,
    rhs: Self,
    span: SourceSpan,
    diag: &impl Diagnosis<'context>,
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

  pub fn handle_unary_op<'context>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    _diag: &impl Diagnosis<'context>,
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

impl Floating {
  pub fn handle_binary_arith_op<'context>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    diag: &impl Diagnosis<'context>,
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

  pub fn handle_binary_order_op<'context>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    diag: &impl Diagnosis<'context>,
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

  pub fn handle_unary_arith_op<'context>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    _diag: &impl Diagnosis<'context>,
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

  pub fn handle_unary_order_op<'context>(
    operator: Operator,
    operand: Self,
    span: SourceSpan,
    diag: &impl Diagnosis<'context>,
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
