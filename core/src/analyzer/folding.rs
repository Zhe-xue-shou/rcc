use ::rc_utils::{IntoWith, contract_assert, contract_violation};

use super::expression::{
  ArraySubscript, Assignment, Binary, CStyleCast, Call, CompoundLiteral,
  Constant, ConstantLiteral as CL, Empty, Expression, ImplicitCast,
  MemberAccess, Paren, RawExpr, SizeOf, SizeOfKind, Ternary, Unary,
  ValueCategory, Variable,
};
use crate::{
  common::{Operator, SourceSpan},
  diagnosis::{DiagData::*, Diagnosis},
  types::{CastType, Compatibility, Primitive, QualifiedType, Type, TypeInfo},
};

#[derive(Debug)]
pub enum FoldingResult<T> {
  Success(T),
  Failure(T),
}
impl<T> ::std::ops::FromResidual for FoldingResult<T> {
  fn from_residual(residual: <Self as ::std::ops::Try>::Residual) -> Self {
    residual
  }
}

impl<T> ::std::ops::Try for FoldingResult<T> {
  type Output = T;
  type Residual = FoldingResult<T>;

  fn from_output(output: Self::Output) -> Self {
    Self::Success(output)
  }

  fn branch(self) -> ::std::ops::ControlFlow<Self::Residual, Self::Output> {
    match self {
      Self::Success(v) => ::std::ops::ControlFlow::Continue(v),
      _ => ::std::ops::ControlFlow::Break(self),
    }
  }
}

impl<T> FoldingResult<T> {
  fn map<U>(self, f: impl FnOnce(T) -> U) -> FoldingResult<U> {
    match self {
      Self::Success(v) => FoldingResult::Success(f(v)),
      Self::Failure(v) => FoldingResult::Failure(f(v)),
    }
  }

  pub fn unwrap(self) -> T {
    match self {
      Self::Failure(v) | Self::Success(v) => v,
    }
  }

  pub fn transform<U>(self, f: impl FnOnce(T) -> U) -> U {
    match self {
      Self::Success(v) | Self::Failure(v) => f(v),
    }
  }
}

/// Folding trait for constant expression evaluation
pub trait Folding {
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
    target_type: QualifiedType,
    value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression>;
}

use FoldingResult::{Failure, Success};

impl Expression {
  #[inline(always)]
  pub(super) fn fold(self, diag: &impl Diagnosis) -> FoldingResult<Expression> {
    let (raw_expr, expr_type, value_category) = self.destructure();
    raw_expr.fold(expr_type, value_category, diag)
  }
}
impl Folding for Empty {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}

impl Folding for Call {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}

impl Folding for MemberAccess {
  fn fold(
    self,
    _target_type: QualifiedType,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    todo!()
  }
}
impl Folding for CStyleCast {
  fn fold(
    self,
    _target_type: QualifiedType,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    todo!()
  }
}

impl Folding for ArraySubscript {
  fn fold(
    self,
    _target_type: QualifiedType,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    todo!()
  }
}

impl Folding for CompoundLiteral {
  fn fold(
    self,
    _target_type: QualifiedType,
    _value_category: ValueCategory,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    todo!()
  }
}

impl Folding for Assignment {
  /// assignment expr is not considered constant expr in C, but in C++ it is.
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    Failure(Expression::new(self.into(), target_type, value_category))
  }
}
macro_rules! static_dispatch {
  ($this:ident.$func:ident $args:tt, $($variant:ident)*) => {
    match $this {
      $(
        Self::$variant(v) => v.$func $args,
      )*
    }
  }
}
impl Folding for RawExpr {
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    static_dispatch!(
      self.fold(target_type, value_category, diag),
      Empty Constant Unary Binary Call Paren MemberAccess Ternary SizeOf CStyleCast ArraySubscript CompoundLiteral Variable ImplicitCast Assignment
    )
  }
}
impl Folding for Constant {
  #[inline(always)]
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    _diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    Success(Expression::new(self.into(), target_type, value_category))
  }
}

impl Folding for Unary {
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    debug_assert!(
      self.operator.unary(),
      "not an unary operator! should not happen!"
    );

    let folded_operand = self.operand.fold(diag)?;

    contract_assert!(
      folded_operand.raw_expr().is_constant(),
      "only implemented for constant var of constant eval"
    );

    let raw_constant = match self.operator {
      // unary `+` is no-op for arithmetic types
      Operator::Plus => return Success(folded_operand),
      // this happens after promotion, so no need to worry about smaller types
      Operator::Minus =>
        match &folded_operand.raw_expr().as_constant_unchecked().constant {
          CL::UInt(u) => {
            let (res, overflow) = u.overflowing_neg();
            if overflow {
              diag.add_warning(
                ArithmeticUnaryOpOverflow(CL::UInt(*u), Operator::Minus),
                self.span,
              )
            }
            // unsigned integer overflow is well-defined in C, so still a constant expression
            Success(CL::UInt(res))
          },
          CL::Int(i) => Success(CL::Int(i.wrapping_neg())),
          CL::ULongLong(ull) => {
            let (res, overflow) = ull.overflowing_neg();
            if overflow {
              diag.add_warning(
                ArithmeticUnaryOpOverflow(CL::ULongLong(*ull), Operator::Minus),
                self.span,
              )
            }
            // ditto
            Success(CL::ULongLong(res))
          },
          CL::LongLong(ll) => Success(CL::LongLong(ll.wrapping_neg())),
          CL::Float(f) => Success(CL::Float(-f)),
          CL::Double(d) => Success(CL::Double(-d)),
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
          Success(CL::Int(1))
        } else {
          Success(CL::Int(0))
        },
      _ => todo!(),
    };
    raw_constant.map(|constant| {
      Expression::new(
        constant.into_with(self.span),
        target_type,
        value_category,
      )
    })
  }
}

impl Folding for Binary {
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    debug_assert!(
      self.operator.binary(),
      "not a binary operator! should not happen!"
    );
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
    .map(|constant| {
      Expression::new(
        constant.into_with(self.span),
        target_type,
        value_category,
      )
    })
  }
}
impl Binary {
  fn integral_folding(
    op: Operator,
    folded_lhs: Expression,
    folded_rhs: Expression,
    span: SourceSpan,
    diag: &impl Diagnosis,
  ) -> FoldingResult<CL> {
    use crate::underlying_type_of;

    let (lhs_expr, lhs_type, lhs_value_category) = folded_lhs.destructure();
    let (rhs_expr, rhs_type, rhs_value_category) = folded_rhs.destructure();

    assert!(
      lhs_type == rhs_type,
      "type checker ensures both sides have the same types!"
    );

    assert!(
      lhs_value_category == ValueCategory::RValue
        && rhs_value_category == ValueCategory::RValue,
      "type checker ensures both sides are rvalues!"
    );

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
          $( (CL::$variant(l), CL::$variant(r)) => {
            let (res, overflow) = <underlying_type_of!($variant)>::$op(l, r);
            if overflow {
              diag.add_warning(
                ArithmeticBinOpOverflow(
                  CL::$variant(l),
                  CL::$variant(r),
                  op,
                ),
                span,
              )
            }
            Success(CL::$variant(res))
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
          $( (CL::$variant(l), CL::$variant(r)) => {
            Success(CL::$variant(<underlying_type_of!($variant)>::$op(l, r)))
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
          $( (CL::$variant(l), CL::$variant(r)) => {
            Success(CL::$variant(<underlying_type_of!($variant)>::$op(l, r)))
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
          $( (CL::$variant(l), CL::$variant(r)) => {
            Success(CL::Int(((l != 0) $op (r != 0)) as underlying_type_of!(Int)))
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
          $( (CL::$variant(l), CL::$variant(r)) => {
            Success(CL::Int((l $op r) as underlying_type_of!(Int)))
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
          Failure(CL::Int(0))
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
  ) -> FoldingResult<CL> {
    let (lhs_expr, lhs_type, lhs_value_category) = folded_lhs.destructure();
    let (rhs_expr, rhs_type, rhs_value_category) = folded_rhs.destructure();

    assert!(
      lhs_type == rhs_type,
      "type checker ensures both sides have the same types!"
    );

    assert!(
      lhs_value_category == ValueCategory::RValue
        && rhs_value_category == ValueCategory::RValue,
      "type checker ensures both sides are rvalues!"
    );
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
          $( (CL::$variant(l), CL::$variant(r)) => {
            let res = l $op r;

            // if inputs were finite but result is infinite  => overflow.
            if res.is_infinite() && l.is_finite() && r.is_finite() {
              diag.add_warning(
                ArithmeticBinOpOverflow(
                  CL::$variant(l),
                  CL::$variant(r),
                  op,
                ),
                span,
              )
            }
            // inf with inf
            if res.is_nan() {
              diag.add_warning(
                NotANumber(
                  CL::$variant(l),
                  CL::$variant(r),
                  op,
                ),
                span,
              )
            }

            Success(CL::$variant(res).into())
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
          $( (CL::$variant(l), CL::$variant(r)) => {
            Success(CL::Int(((l != 0.0) $op (r != 0.0)) as underlying_type_of!(Int)).into())
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
          $( (CL::$variant(l), CL::$variant(r)) => {
            Success(CL::Int((l $op r) as underlying_type_of!(Int)).into())
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

      Operator::And => shorthand!(logical, &&),
      Operator::Or => shorthand!(logical, ||),

      Operator::Less => shorthand!(rel, <),
      Operator::LessEqual => shorthand!(rel, <=),
      Operator::Greater => shorthand!(rel, >),
      Operator::GreaterEqual => shorthand!(rel, >=),
      Operator::EqualEqual => shorthand!(rel, ==),
      Operator::NotEqual => shorthand!(rel, !=),

      _ => contract_violation!(
        "not a binary operator or bin-op but cannot be applied to floating-point! assignment op should be handled upstream, so does comma."
      ),
    }
  }
}

impl Folding for Ternary {
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    debug_assert!(
      self
        .then_expr
        .qualified_type()
        .compatible_with(self.else_expr.qualified_type()),
      "type checker ensures both branches have compatible types!"
    );
    let fc = self.condition.fold(diag);
    let ft = self.then_expr.fold(diag);
    let fe = self.else_expr.fold(diag);

    let is_success =
      matches!((&fc, &ft, &fe), (Success(_), Success(_), Success(_)));

    let expr = Expression::new(
      Self {
        condition: fc.unwrap().into(),
        then_expr: ft.unwrap().into(),
        else_expr: fe.unwrap().into(),
        ..self
      }
      .into(),
      target_type,
      value_category,
    );

    match is_success {
      true => Success(expr),
      false => Failure(expr),
    }
  }
}

impl Folding for SizeOf {
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    match self.sizeof {
      SizeOfKind::Type(qualified_type) => if qualified_type.size() > 0 {
        Success(CL::ULongLong(qualified_type.size() as u64))
      } else {
        Failure(CL::ULongLong(qualified_type.size() as u64))
      }
      .map(|constant| {
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

impl Folding for Variable {
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
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

impl Folding for Paren {
  fn fold(
    self,
    _target_type: QualifiedType,
    _value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    self.expr.fold(diag).map(|expr| expr)
  }
}

impl Folding for ImplicitCast {
  fn fold(
    self,
    target_type: QualifiedType,
    value_category: ValueCategory,
    diag: &impl Diagnosis,
  ) -> FoldingResult<Expression> {
    let folded_expr = self.expr.fold(diag)?;
    let (raw_expr, expr_type, _value_category) = folded_expr.destructure();

    use CastType::*;
    match self.cast_type {
      Noop | ToVoid | LValueToRValue | BitCast => Success(raw_expr),
      ArrayToPointerDecay => todo!("address constant"),
      FunctionToPointerDecay => todo!("address constant"),
      NullptrToPointer => match target_type.unqualified_type() {
        // FIXME: i dont think this is correct
        Type::Pointer(_) =>
          Success(RawExpr::Constant(CL::ULongLong(0).into_with(self.span))),
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
          todo!("{c}")
        },
        _ => contract_violation!("unreachable"),
      },
      // integral are promoted previously.
      IntegralToFloating => {
        // let new_constant = match (
        //   target_type.unqualified_type(),
        //   expr_type.unqualified_type(),
        // ) {
        //   (Type::Primitive(target), Type::Primitive(current)) =>
        //     match (target, current) {
        //       (&Primitive::Float, &Primitive::Int) =>
        //         raw_expr.into_constant_unchecked(). as underlying_type_of!(Float),
        //       _ => todo!(),
        //     },
        //   _ => contract_violation!("unreachable"),
        // };
        todo!()
      },
      IntegralToBoolean | FloatingToBoolean => match raw_expr {
        RawExpr::Constant(c) =>
          Success(CL::Int(if c.constant.is_zero() { 0 } else { 1 }))
            .map(|c| c.into_with(self.span)),
        _ => contract_violation!("unreachable"),
      },
      FloatingCast => todo!(),
      FloatingToIntegral => todo!(),
      IntegralToPointer => contract_violation!(
        "no such implicit cast in C -- only for explicit casts!"
      ),
      PointerToIntegral => Failure(raw_expr),
      PointerToBoolean => Failure(raw_expr),
      NullptrToIntegral => match target_type.unqualified_type() {
        Type::Primitive(p) => match p {
          Primitive::Int => Success(CL::Int(0)),
          Primitive::Long => Success(CL::LongLong(0)),
          Primitive::LongLong => Success(CL::LongLong(0)),
          Primitive::UInt => Success(CL::UInt(0)),
          Primitive::ULong => Success(CL::ULongLong(0)),
          Primitive::ULongLong => Success(CL::ULongLong(0)),
          _ => contract_violation!("unreachable"),
        },
        _ => contract_violation!("unreachable"),
      }
      .map(|c| c.into_with(self.span)),
      NullptrToBoolean =>
        Success(CL::Bool(false)).map(|c| c.into_with(self.span)),
    }
    .map(|raw_expr| Expression::new(raw_expr, target_type, value_category))
  }
}
