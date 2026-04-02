use ::rcc_adt::{Floating, Integral};
use ::rcc_ast::{
  Session,
  types::{CastType, Compatibility, QualifiedType, TypeInfo},
};
use ::rcc_shared::{
  DiagData::*, Diagnosis, Operator, OperatorCategory, SourceSpan,
};
use ::rcc_utils::{RefEq, contract_assert, contract_violation};

use super::expression::{
  ArraySubscript, Binary, CStyleCast, Call, CompoundLiteral, Constant,
  Constant as CL, Empty, ExprRef, Expression, ImplicitCast, MemberAccess,
  Paren, RawExpr, SizeOf, SizeOfKind, Ternary, Unary, ValueCategory, Variable,
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
pub trait Folding<'c, VariantTy> {
  /// This serves as a never-fail folding mechanism,
  /// all errors and warnings shall be handled via `diag` parameter.
  /// [`Operational`](crate::diagnosis::Operational) is recommended.
  ///
  /// If folding is not possible, return self unchanged.
  /// So it may end up being a no-op, partial-fold, or full-fold.
  ///
  /// If [`Diagnosis`] is not required, use [`NoOp`](crate::diagnosis::NoOp) as the dummy parameter.
  #[must_use]
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    variant: &VariantTy,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>>;
}

use FoldingResult::{Failure, Success};

#[inline]
fn retype_or_reuse<'c, D: Diagnosis<'c>>(
  session: &Session<'c, D>,
  expression: ExprRef<'c>,
  target_type: QualifiedType<'c>,
  value_category: ValueCategory,
) -> ExprRef<'c> {
  if QualifiedType::ref_eq_same(expression.qualified_type(), &target_type)
    && expression.value_category() == value_category
  {
    expression
  } else {
    Expression::new(
      session.ast(),
      expression.raw_expr().clone(),
      target_type,
      value_category,
      expression.span(),
    )
  }
}

#[inline(always)]
fn fold_overload<'c, D, V>(
  expression: ExprRef<'c>,
  variant: &V,
  session: &Session<'c, D>,
) -> FoldingResult<ExprRef<'c>>
where
  D: Diagnosis<'c>,
  Expression<'c>: Folding<'c, V>,
{
  <Expression<'c> as Folding<'c, V>>::fold(expression, variant, session)
}

impl<'c> Expression<'c> {
  #[inline(always)]
  pub fn fold<D: Diagnosis<'c>>(
    self: ExprRef<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    ::rcc_utils::static_dispatch!(
      RawExpr: self.raw_expr(),
      |variant| fold_overload(self, variant, session) =>
      Empty Constant Unary Binary Call Paren MemberAccess Ternary SizeOf
      CStyleCast ArraySubscript CompoundLiteral Variable ImplicitCast CompoundAssign
    )
  }
}
impl<'c> Folding<'c, Empty> for Expression<'c> {
  #[inline(always)]
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    _variant: &Empty,
    _session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    Failure(expression)
  }
}

impl<'c> Folding<'c, Call<'c>> for Expression<'c> {
  #[inline(always)]
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    call: &Call<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    let callee = call.callee.fold(session).take();
    let mut changed = !::std::ptr::eq(callee, call.callee);
    let arguments = call
      .arguments
      .iter()
      .copied()
      .map(|arg| {
        let folded = arg.fold(session).take();
        changed |= !::std::ptr::eq(folded, arg);
        folded
      })
      .collect::<Vec<_>>();

    if !changed {
      Failure(expression)
    } else {
      Failure({
        let raw_expr = Call::new(session.ast(), callee, arguments);
        Expression::new(
          session.ast(),
          raw_expr,
          *expression.qualified_type(),
          expression.value_category(),
          expression.span(),
        )
      })
    }
  }
}

impl<'c> Folding<'c, MemberAccess<'c>> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    _expression: ExprRef<'c>,
    _variant: &MemberAccess<'c>,
    _session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    todo!()
  }
}
impl<'c> Folding<'c, CStyleCast<'c>> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    _expression: ExprRef<'c>,
    _variant: &CStyleCast<'c>,
    _session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    todo!()
  }
}

impl<'c> Folding<'c, ArraySubscript<'c>> for Expression<'c> {
  /// always fails folding in C, unlike C++.
  #[inline(always)]
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    array_subscript: &ArraySubscript<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    let array = array_subscript.array.fold(session).take();
    let index = array_subscript.index.fold(session).take();

    if ::std::ptr::eq(array, array_subscript.array)
      && ::std::ptr::eq(index, array_subscript.index)
    {
      Failure(expression)
    } else {
      Failure({
        let raw_expr = ArraySubscript::new(array, index);
        Expression::new(
          session.ast(),
          raw_expr,
          *expression.qualified_type(),
          expression.value_category(),
          expression.span(),
        )
      })
    }
  }
}

impl<'c> Folding<'c, CompoundLiteral> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    _expression: ExprRef<'c>,
    _variant: &CompoundLiteral,
    _session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    todo!()
  }
}

impl<'c> Folding<'c, Constant<'c>> for Expression<'c> {
  #[inline(always)]
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    _constant: &Constant<'c>,
    _session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    Success(expression)
  }
}

impl<'c> Folding<'c, Unary<'c>> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    unary: &Unary<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    let target_type = *expression.qualified_type();
    let value_category = expression.value_category();

    debug_assert!(
      unary.operator.unary(),
      "not an unary operator! should not happen!"
    );

    let folded_operand = match unary.operand.fold(session) {
      Success(folded) => folded,
      Failure(original) => {
        if ::std::ptr::eq(original, unary.operand) {
          return Failure(expression);
        }
        return Failure({
          let raw_expr = Unary {
            operand: original,
            ..unary.clone()
          };
          Expression::new(
            session.ast(),
            raw_expr,
            *expression.qualified_type(),
            expression.value_category(),
            expression.span(),
          )
        });
      },
    };
    use OperatorCategory::*;

    contract_assert!(
      folded_operand.raw_expr().is_constant(),
      "only implemented for constant var of constant eval"
    );
    match folded_operand.raw_expr().as_constant_unchecked().clone() {
      ::rcc_ast::Constant::Integral(operand) => Integral::handle_unary_op(
        unary.operator,
        operand,
        expression.span(),
        session,
      )
      .map(Into::into)
      .map(|constant: CL| {
        Expression::new(
          session.ast(),
          constant,
          target_type,
          value_category,
          expression.span(),
        )
      }),
      ::rcc_ast::Constant::Floating(operand) => match unary.operator.category()
      {
        Arithmetic => Floating::handle_unary_arith_op(
          unary.operator,
          operand,
          expression.span(),
          session,
        )
        .map(Into::into),
        Logical => Floating::handle_unary_order_op(
          unary.operator,
          operand,
          expression.span(),
          session,
        )
        .map(Into::into),
        _ => contract_violation!(
          "not a unary operator or un-op but cannot be applied to floating!"
        ),
      }
      .map(|constant: CL| {
        Expression::new(
          session.ast(),
          constant,
          target_type,
          value_category,
          expression.span(),
        )
      }),
      _ => unimplemented!(),
    }
  }
}

impl<'c> Folding<'c, Binary<'c>> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    binary: &Binary<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    let target_type = *expression.qualified_type();
    let value_category = expression.value_category();

    debug_assert!(
      binary.operator.binary(),
      "not a binary operator! should not happen!"
    );
    // logical && and || are short-circuit evaluated,
    // so    `static const int j = 0 && x;` would pass constant folding,
    // while `static const int j = 0 || x;` would not.
    let fl = binary.left.fold(session);
    let fr = binary.right.fold(session);

    let f = |left: ExprRef<'c>, right: ExprRef<'c>| {
      if ::std::ptr::eq(left, binary.left)
        && ::std::ptr::eq(right, binary.right)
      {
        expression
      } else {
        {
          let raw_expr = Binary {
            left,
            right,
            ..binary.clone()
          };
          Expression::new(
            session.ast(),
            raw_expr,
            *expression.qualified_type(),
            expression.value_category(),
            expression.span(),
          )
        }
      }
    };

    let (folded_lhs, folded_rhs) = match (fl, fr) {
      (Success(left), Success(right)) => (left, right),
      (Success(left), Failure(_))
        if binary.operator == Operator::LogicalAnd
          && left.raw_expr().as_constant_unchecked().clone().is_zero() =>
        return Success(left),

      (Success(left), Failure(right))
        if binary.operator == Operator::LogicalAnd =>
        return Failure(f(left, right)),

      (Success(left), Failure(_))
        if binary.operator == Operator::LogicalOr
          && left
            .raw_expr()
            .as_constant_unchecked()
            .clone()
            .is_not_zero() =>
        return Success(left),
      (Success(left), Failure(right))
        if binary.operator == Operator::LogicalOr =>
        return Failure(f(left, right)),

      (fl, fr) => return Failure(f(fl.take(), fr.take())),
    };
    if binary.operator == Operator::Comma {
      session
        .diag()
        .add_warning(LeftCommaNoEffect, expression.span());
      return Success(retype_or_reuse(
        session,
        expression,
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
      binary.operator
    );
    let lhs_expr = folded_lhs.raw_expr();
    let rhs_expr = folded_rhs.raw_expr();
    let lhs_type = folded_lhs.qualified_type();
    let rhs_type = folded_rhs.qualified_type();
    let lhs_value_category = folded_lhs.value_category();
    let rhs_value_category = folded_rhs.value_category();

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

    let lhs = lhs_expr.as_constant_unchecked().clone();
    let rhs = rhs_expr.as_constant_unchecked().clone();

    use OperatorCategory::*;
    match (lhs, rhs) {
      (
        ::rcc_ast::Constant::Integral(lhs),
        ::rcc_ast::Constant::Integral(rhs),
      ) => Integral::handle_binary_op(
        binary.operator,
        lhs,
        rhs,
        expression.span(),
        session,
      )
      .map(Into::into),
      (
        ::rcc_ast::Constant::Floating(lhs),
        ::rcc_ast::Constant::Floating(rhs),
      ) => match binary.operator.category() {
        Logical | Relational => Floating::handle_binary_order_op(
          binary.operator,
          lhs,
          rhs,
          expression.span(),
          session,
        )
        .map(Into::into),
        Arithmetic => Floating::handle_binary_arith_op(
          binary.operator,
          lhs,
          rhs,
          expression.span(),
          session,
        )
        .map(Into::into),
        _ => contract_violation!(
          "not a binary operator or bin-op but cannot be applied to floating!"
        ),
      },
      (::rcc_ast::Constant::String(_), ::rcc_ast::Constant::String(_)) =>
        contract_violation!("can we reach here?"),
      (::rcc_ast::Constant::Nullptr(), ::rcc_ast::Constant::Nullptr()) =>
        contract_violation!("can we reach here?"),
      _ => contract_violation!(
        "type checker ensures both sides have the same types! or \
         unimplemented type"
      ),
    }
    .map(|constant: CL| {
      Expression::new(
        session.ast(),
        constant,
        target_type,
        value_category,
        expression.span(),
      )
    })
  }
}
impl<'c> Folding<'c, Ternary<'c>> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    ternary: &Ternary<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    let target_type = *expression.qualified_type();
    let value_category = expression.value_category();

    debug_assert!(
      ternary
        .then_expr
        .expect("?: unimplemented")
        .qualified_type()
        .compatible_with(ternary.else_expr.qualified_type()),
      "type checker ensures both branches have compatible types!"
    );
    let fc = ternary.condition.fold(session);
    match fc {
      Success(folded_condition) => {
        match folded_condition
          .raw_expr()
          .as_constant_unchecked()
          .clone()
          .is_zero()
        {
          true => ternary.else_expr.fold(session),
          false => ternary.then_expr.expect("?: unimplemented").fold(session),
        }
      },
      Failure(folded_condition) => {
        let then_expr = ternary
          .then_expr
          .expect("?: unimplemented")
          .fold(session)
          .take();
        let else_expr = ternary.else_expr.fold(session).take();
        if ::std::ptr::eq(folded_condition, ternary.condition)
          && ::std::ptr::eq(
            then_expr,
            ternary.then_expr.expect("?: unimplemented"),
          )
          && ::std::ptr::eq(else_expr, ternary.else_expr)
        {
          Failure(expression)
        } else {
          Failure(Expression::new(
            session.ast(),
            Ternary {
              condition: folded_condition,
              then_expr: Some(then_expr),
              else_expr,
            },
            target_type,
            value_category,
            expression.span(),
          ))
        }
      },
    }
  }
}

impl<'c> Folding<'c, SizeOf<'c>> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    sizeof: &SizeOf<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    let target_type = *expression.qualified_type();
    let value_category = expression.value_category();
    match sizeof.sizeof {
      SizeOfKind::Type(qualified_type) => if qualified_type.size() > 0 {
        Success(Integral::from_uintptr(qualified_type.size()))
      } else {
        Failure(Integral::from_uintptr(0))
      }
      .map(Into::into)
      .map(|constant: CL| {
        Expression::new(
          session.ast(),
          constant,
          target_type,
          value_category,
          expression.span(),
        )
      }),
      SizeOfKind::Expression(expr) => expr.fold(session),
    }
  }
}

impl<'c> Folding<'c, Variable<'c>> for Expression<'c> {
  /// TODO: address constant shall be handled here also.
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    variable: &Variable<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    use ::rcc_ast::types::Qualifiers;
    use ::rcc_shared::Storage::*;

    // if target_type.is_functionproto()
    //   && self.declaration.is_eligible_of_address_constant()
    // {
    //   return Success(Expression::new(self, target_type, value_category));
    // }

    let declaration = variable.declaration;
    let storage = declaration.storage_class();
    match storage {
      Static
        if declaration
          .qualified_type()
          .qualifiers
          .contains(Qualifiers::Const) =>
      {
        session.diag().add_error(
          UnsupportedFeature(
            "static variable const initialization from const variable \
             unimplemented."
              .to_string(),
          ),
          expression.span(),
        );
        Failure(expression)
      },
      Automatic | Register | Extern | Static => Failure(expression),
      ThreadLocal | Constexpr => {
        session.diag().add_error(
          UnsupportedFeature("constexpr variable not implemented".to_string()),
          expression.span(),
        );
        Failure(expression)
      },
      Typedef => unreachable!(),
    }
  }
}

impl<'c> Folding<'c, Paren<'c>> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    _expression: ExprRef<'c>,
    paren: &Paren<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    paren.expr.fold(session)
  }
}

impl<'c> Folding<'c, ImplicitCast<'c>> for Expression<'c> {
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    implicit_cast: &ImplicitCast<'c>,
    session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    let target_type = *expression.qualified_type();
    let value_category = expression.value_category();

    let folded_expr = match implicit_cast.expr.fold(session) {
      Success(folded) => folded,
      Failure(original) => {
        if ::std::ptr::eq(original, implicit_cast.expr) {
          return Failure(expression);
        }
        return Failure({
          let raw_expr = ImplicitCast {
            expr: original,
            ..implicit_cast.clone()
          };
          Expression::new(
            session.ast(),
            raw_expr,
            *expression.qualified_type(),
            expression.value_category(),
            expression.span(),
          )
        });
      },
    };
    let raw_expr = folded_expr.raw_expr();
    let expr_type = *folded_expr.qualified_type();

    let alloc_constant = |constant: ::rcc_ast::Constant<'c>| -> ExprRef<'c> {
      Expression::new(
        session.ast(),
        constant,
        target_type,
        value_category,
        expression.span(),
      )
    };

    use CastType::*;
    match implicit_cast.cast_type {
      Noop | ToVoid | LValueToRValue | BitCast => Success(retype_or_reuse(
        session,
        folded_expr,
        target_type,
        value_category,
      )),
      PointerToIntegral | PointerToBoolean => Failure(retype_or_reuse(
        session,
        folded_expr,
        target_type,
        value_category,
      )),
      ArrayToPointerDecay => todo!("address constant"),
      FunctionToPointerDecay => todo!("address constant"),
      NullptrToPointer => Success(retype_or_reuse(
        session,
        folded_expr,
        target_type,
        value_category,
      )),
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
            session.diag().add_warning(
              CastDown(expr_type.to_string(), target_type.to_string()),
              expression.span(),
            )
          }
          Success(alloc_constant(
            c.as_integral_unchecked()
              .cast(
                target_primitive.size_bits() as u8,
                target_primitive.is_signed().into(),
              )
              .into(),
          ))
        },
        _ => contract_violation!("unreachable: {:?}", raw_expr),
      },
      // integral are promoted previously.
      IntegralToFloating => Success(alloc_constant(
        raw_expr.as_constant_unchecked().clone().to_floating(
          target_type
            .unqualified_type
            .as_primitive_unchecked()
            .floating_format(),
        ),
      )),
      IntegralToBoolean | FloatingToBoolean => Success(alloc_constant(
        raw_expr.as_constant_unchecked().clone().to_boolean(),
      )),
      FloatingCast => {
        let floating = raw_expr
          .as_constant_unchecked()
          .clone()
          .as_floating_unchecked()
          .cast(
            target_type
              .unqualified_type
              .as_primitive_unchecked()
              .floating_format(),
          );
        Success(alloc_constant(floating.into()))
      },
      FloatingToIntegral => {
        let integral = raw_expr.as_constant_unchecked().clone().to_integral(
          target_type.as_primitive_unchecked().integer_width(),
          target_type
            .as_primitive_unchecked()
            .is_signed_integer()
            .into(),
        );
        Success(alloc_constant(integral))
      },
      IntegralToPointer => contract_violation!(
        "no such implicit cast in C -- only for explicit casts!"
      ),
    }
  }
}
impl<'c> Folding<'c, CompoundAssign<'c>> for Expression<'c> {
  /// should always fail, but anyways
  #[inline(always)]
  fn fold<D: Diagnosis<'c>>(
    expression: ExprRef<'c>,
    _compound_assign: &CompoundAssign<'c>,
    _session: &Session<'c, D>,
  ) -> FoldingResult<ExprRef<'c>> {
    Failure(expression)
  }
}
mod private {
  use super::{Floating, Integral};
  pub trait Sealed {}
  impl Sealed for Integral {}
  impl Sealed for Floating {}
}
trait IntegralExt: private::Sealed {
  fn handle_binary_op<'c, D: Diagnosis<'c>>(
    op: Operator,
    lhs: Self,
    rhs: Self,
    span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Self>
  where
    Self: Sized;

  fn handle_unary_op<'c, D: Diagnosis<'c>>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Self>
  where
    Self: Sized;
}

impl IntegralExt for Integral {
  fn handle_binary_op<'c, D: Diagnosis<'c>>(
    op: Operator,
    lhs: Self,
    rhs: Self,
    span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Self> {
    debug_assert!(op.binary());
    debug_assert_eq!(lhs.width(), rhs.width());
    debug_assert_eq!(lhs.signedness(), rhs.signedness());

    macro_rules! arith {
      ($func:ident, $op:expr) => {{
        let (result, overflow) = lhs.$func(rhs);
        if overflow {
          session.diag().add_warning(
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
            session.diag().add_error(DivideByZero, span);
            Failure(
              Integral::new(0, lhs.width() as u8, lhs.signedness()).into(),
            )
          },
        }
      }};
    }
    macro_rules! logical_misuse_warn {
    ($op_sym:tt, $op_variant:ident, $suggest:ident) => {{
        session.diag().add_warning(
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

      LogicalAnd => logical_misuse_warn!(&&, LogicalAnd, Ampersand),
      LogicalOr => logical_misuse_warn!(||, LogicalOr, Pipe),

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

  fn handle_unary_op<'c, D: Diagnosis<'c>>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    _session: &Session<'c, D>,
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
  fn handle_binary_arith_op<'c, D: Diagnosis<'c>>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Self>
  where
    Self: Sized;

  fn handle_binary_order_op<'c, D: Diagnosis<'c>>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Integral>;

  fn handle_unary_arith_op<'c, D: Diagnosis<'c>>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Self>
  where
    Self: Sized;

  fn handle_unary_order_op<'c, D: Diagnosis<'c>>(
    operator: Operator,
    operand: Self,
    span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Integral>;
}

impl FloatingExt for Floating {
  fn handle_binary_arith_op<'c, D: Diagnosis<'c>>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    session: &Session<'c, D>,
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
          session.diag().add_warning(
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

  fn handle_binary_order_op<'c, D: Diagnosis<'c>>(
    op: Operator,
    lhs: Floating,
    rhs: Floating,
    span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Integral> {
    use OperatorCategory::*;
    debug_assert!(
      matches!(op.category(), Logical | Relational),
      "Tried to perform unary operation in binary op handler"
    );
    macro_rules! logical_misuse_warn {
    ($op_sym:tt, $op_variant:ident, $suggest:ident) => {{
        session.diag().add_warning(
            LogicalOpMisuse($op_variant, $suggest.into()),
            span,
        );
        Success(Integral::from_bool(!lhs.is_zero() $op_sym !rhs.is_zero()))
    }}
    }
    use Operator::*;
    match op {
      LogicalAnd => logical_misuse_warn!(&&, LogicalAnd, Ampersand),
      LogicalOr => logical_misuse_warn!(||, LogicalOr, Pipe),

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

  fn handle_unary_arith_op<'c, D: Diagnosis<'c>>(
    operator: Operator,
    operand: Self,
    _span: SourceSpan,
    _session: &Session<'c, D>,
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

  fn handle_unary_order_op<'c, D: Diagnosis<'c>>(
    operator: Operator,
    operand: Self,
    span: SourceSpan,
    session: &Session<'c, D>,
  ) -> FoldingResult<Integral> {
    debug_assert!(operator.unary());
    use Operator::*;
    match operator {
      Not => {
        session.diag().add_warning(LogicalOpMisuse(Not, None), span);
        Success(Integral::from_bool(operand.is_zero()))
      },
      LogicalAnd | LogicalOr | Less | LessEqual | Greater | GreaterEqual
      | EqualEqual | NotEqual | Star | Ampersand | PlusPlus | MinusMinus =>
        contract_violation!(
          "unary operator not applicable to floating! should be handled \
           upstream"
        ),
      _ => unreachable!(),
    }
  }
}
