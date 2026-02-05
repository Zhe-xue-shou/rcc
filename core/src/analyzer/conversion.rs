#![allow(clippy::double_must_use)]

use ::rcc_utils::IntoWith;

use crate::{
  analyzer::expression::{Expression, ImplicitCast},
  common::SourceSpan,
  diagnosis::{Diag, DiagData::*, Severity},
  types::{
    CastType, Compatibility, Pointer, Primitive, Promotion, QualifiedType, Type,
  },
};

impl Expression {
  /// 6.3.1.8 Usual arithmetic conversions, applied implicitly where arithmetic conversions are required
  #[must_use]
  #[inline]
  pub fn usual_arithmetic_conversion(
    lhs: Expression,
    rhs: Expression,
  ) -> Result<(Expression, Expression, QualifiedType), Diag> {
    assert!(
      !lhs.is_lvalue() && !rhs.is_lvalue(),
      "perform lvalue conversion first"
    );
    assert!(
      !matches!(
        lhs.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ) && !matches!(
        rhs.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    Self::usual_arithmetic_conversion_unchecked(lhs, rhs)
  }

  #[must_use]
  #[inline]
  pub fn usual_arithmetic_conversion_unary(self) -> Result<Self, Diag> {
    assert!(!self.is_lvalue(), "perform lvalue conversion first");
    assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    Self::usual_arithmetic_conversion_unary_unchecked(self)
  }

  /// 6.3.1.2.1: When any scalar value is converted to bool, the result is false if:
  ///   - the value is a zero (for arithmetic types)
  ///   - null (for pointer types),
  ///   - the scalar has type nullptr_t
  ///
  /// otherwise, the result is true.
  ///
  /// NO promotion is performed.
  ///
  /// unary.
  ///
  /// Returned expression is always rvalue of type `int` -- well, cannot be `bool` since `sizeof` expression will be wrong!
  #[must_use]
  #[inline]
  pub fn conditional_conversion(self) -> Result<Self, Diag> {
    assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    Self::conditional_conversion_unchecked(self)
  }

  /// If an expression of any other type is evaluated as a void expression, its value or designator is discarded.
  /// (A void expression is evaluated for its side effects -- of course we need to evaluate it!)
  ///
  /// unary.
  #[must_use]
  #[inline]
  pub fn void_conversion(self) -> Self {
    let span = self.span();
    Self::new_rvalue(
      ImplicitCast::new(self.into(), CastType::ToVoid, span).into(),
      QualifiedType::void(),
    )
  }

  /// 6.5.17.2 Simple assignment
  #[must_use]
  #[inline]
  pub fn assignment_conversion(
    self,
    target_type: &QualifiedType,
  ) -> Result<Self, Diag> {
    assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    Self::assignment_conversion_unchecked(self, target_type)
  }

  /// 6.3.2.1 Lvalues, arrays, and function designators
  ///
  /// Except when it is the operand of the sizeof operator, or the typeof operators, the unary `&` operator,
  ///     the `++` operator, the `--` operator, or the left operand of the `.` operator or an assignment operator, an
  ///     lvalue that does not have array type is converted to the value stored in the designated object (and is
  ///     no longer an lvalue); this is called lvalue conversion.
  #[must_use]
  pub fn lvalue_conversion(self) -> Self {
    if self.unqualified_type().is_array()
      || self.unqualified_type().is_functionproto()
    {
      // these types are not converted by lvalue conversion
      self
    } else if self.is_lvalue() {
      // If the lvalue has qualified type, the value has the unqualified version of the type of the lvalue. perform cast from lvalue to rvalue
      let old_unqual_type = self.unqualified_type().clone();
      let span = self.span();
      Self::new_rvalue(
        ImplicitCast::new(self.into(), CastType::LValueToRValue, span).into(),
        old_unqual_type.into(),
      )
    } else {
      self
    }
  }

  #[must_use]
  #[inline]
  pub fn decay(self) -> Self {
    match self.unqualified_type() {
      Type::Array(_) => self.array_to_pointer_decay(),
      Type::FunctionProto(_) => self.function_to_pointer_decay(),
      _ => self,
    }
  }

  /// A function designator is an expression that has function type. Except when it is the operand of the
  ///     sizeof operator, a typeof operator, or the unary & operator, a function designator with type
  ///     "function returning type" is converted to an expression that has type "pointer to function returning
  ///     type"
  #[must_use]
  pub fn function_to_pointer_decay(self) -> Self {
    let function_type = match self.unqualified_type() {
      Type::FunctionProto(f) => f,
      _ => unreachable!(),
    };
    assert!(
      self.qualifiers().is_empty(),
      "function type should not have qualifiers: {:?}",
      self.qualified_type()
    );
    let pointer_type = Pointer::new(QualifiedType::new_unqualified(
      Type::FunctionProto(function_type.clone()).into(),
    ));
    let span = self.span();
    Self::new_rvalue(
      ImplicitCast::new(self.into(), CastType::FunctionToPointerDecay, span)
        .into(),
      // The pointer itself is never qualified
      pointer_type.into(),
    )
  }

  /// Except when it is the operand of the sizeof operator, or typeof operators, or the unary & operator,
  ///       or is a string literal used to initialize an array, an expression that has type "array of type" is converted
  ///       to an expression with type "pointer to type" that points to the initial element of the array object and
  ///       is not an lvalue.
  #[must_use]
  pub fn array_to_pointer_decay(self) -> Self {
    let array_type = match self.unqualified_type() {
      Type::Array(a) => a,
      _ => unreachable!(),
    };
    assert!(
      self.qualifiers().is_empty(),
      "array type should not have qualifiers: {:?}",
      self.qualified_type()
    );
    let pointer_type = Type::from(Pointer::new(
      // array itself should not have qualifiers, but the element qualifiers are preserved
      array_type.element_type.clone(),
    ));
    let span = self.span();
    Self::new_rvalue(
      ImplicitCast::new(self.into(), CastType::ArrayToPointerDecay, span)
        .into(),
      // The pointer itself is never qualified
      pointer_type.into(),
    )
  }
}
// unchecked version is requireds to use w.r.t. `&`, `sizeof`, `alignof`, etc.
impl Expression {
  #[must_use]
  fn assignment_conversion_unchecked(
    self,
    target_type: &QualifiedType,
  ) -> Result<Self, Diag> {
    let span = self.span();

    match (target_type.unqualified_type(), self.unqualified_type()) {
      //  the left operand has [...] arithmetic type, and the right operand has arithmetic type;
      (Type::Primitive(left), Type::Primitive(right))
        if left.is_arithmetic() && right.is_arithmetic() =>
      {
        let cast_type = Self::get_cast_type(
          self.unqualified_type(),
          target_type.unqualified_type(),
        );
        Ok(Self::maybe_cast(self, cast_type, target_type))
      },
      // the left operand has [...] of a structure or union type compatible with the type of the right operand;
      (Type::Record(_), Type::Record(_)) | (Type::Union(_), Type::Union(_)) => {
        todo!()
      },
      // the left operand has atomic, qualified, or unqualified pointer type,
      //    and (considering the type the left operand would have after lvalue conversion) one operand is a pointer to an object type,
      //    and the other is a pointer to a qualified or unqualified version of void,
      //    and the type pointed to by the left operand has all the qualifiers of the type pointed to by the right operand;
      (Type::Pointer(lhs), Type::Pointer(rhs)) => {
        // can add qualifiers, but cannot remove them
        // error if removing qualifiers (const, volatile, etc.)
        if !lhs.pointee.qualifiers().contains(*rhs.pointee.qualifiers()) {
          do yeet DiscardingQualifiers(
            *rhs.pointee.qualifiers() - *lhs.pointee.qualifiers(),
          )
          .into_with(Severity::Error)
          .into_with(span)
        }

        if lhs
          .pointee
          .unqualified_type()
          .compatible_with(rhs.pointee.unqualified_type())
          || lhs.pointee.unqualified_type().is_void()
          || rhs.pointee.unqualified_type().is_void()
        {
          // no need to create composite type -- pointer types are the same except for qualifiers
          Ok(Self::new_rvalue(
            ImplicitCast::new(self.into(), CastType::BitCast, span).into(),
            target_type.clone(),
          ))
        } else {
          Err(
            IncompatiblePointerTypes(
              lhs.pointee.to_string(),
              rhs.pointee.to_string(),
            )
            .into_with(Severity::Error)
            .into_with(span),
          )
        }
      },
      // the left operand has an atomic, qualified, or unqualified version of the nullptr_t type and the right operand is a null pointer constant or its type is nullptr_t;
      (
        Type::Primitive(Primitive::Nullptr),
        Type::Primitive(Primitive::Nullptr),
      ) => Ok(self),
      // the left operand is an atomic, qualified, or unqualified pointer, and the right operand is a null pointer constant or its type is nullptr_t;
      (Type::Pointer(_), Type::Primitive(Primitive::Nullptr)) =>
        Ok(Self::new_rvalue(
          ImplicitCast::new(self.into(), CastType::NullptrToPointer, span)
            .into(),
          target_type.clone(),
        )),

      // the left operand has atomic, qualified, or unqualified bool, and the right operand is a pointer or its type is nullptr_t.
      (Type::Primitive(Primitive::Bool), Type::Pointer(_)) =>
        Ok(Self::new_rvalue(
          ImplicitCast::new(self.into(), CastType::PointerToBoolean, span)
            .into(),
          target_type.clone(),
        )),
      (
        Type::Primitive(Primitive::Bool),
        Type::Primitive(Primitive::Nullptr),
      ) => Ok(Self::new_rvalue(
        ImplicitCast::new(self.into(), CastType::NullptrToBoolean, span).into(),
        target_type.clone(),
      )),
      _ => Err(
        InvalidConversion(format!(
          "cannot convert from '{}' to '{}'",
          self.unqualified_type(),
          target_type.unqualified_type()
        ))
        .into_with(Severity::Error)
        .into_with(self.span()),
      ),
    }
  }

  #[must_use]
  fn conditional_conversion_unchecked(self) -> Result<Self, Diag> {
    let span = self.span();

    match self.unqualified_type() {
      Type::Primitive(Primitive::Bool) => Ok(self),
      Type::Primitive(p) if p.is_integer() =>
        Ok(self.cast_if_needed(&QualifiedType::int())),
      Type::Primitive(p) if p.is_floating_point() => Ok(Self::new_rvalue(
        ImplicitCast::new(self.into(), CastType::FloatingToIntegral, span)
          .into(),
        QualifiedType::int(),
      )),
      Type::Primitive(Primitive::Nullptr) => Ok(Self::new_rvalue(
        ImplicitCast::new(self.into(), CastType::NullptrToIntegral, span)
          .into(),
        QualifiedType::int(),
      )),
      Type::Primitive(Primitive::Void) => Err(
        InvalidConversion(
          "cannot convert void to int in conditional conversion".to_string(),
        )
        .into_with(Severity::Error)
        .into_with(self.span()),
      ),
      Type::Primitive(_) => Err(
        InvalidConversion(format!(
          "cannot convert '{}' to int in conditional conversion",
          self.unqualified_type()
        ))
        .into_with(Severity::Error)
        .into_with(self.span()),
      ),
      // compare with nullptr
      Type::Pointer(_) => Ok(Self::new_rvalue(
        ImplicitCast::new(self.into(), CastType::PointerToIntegral, span)
          .into(),
        QualifiedType::int(),
      )),

      Type::Array(array) => panic!(
        "should be decayed before conditional conversion: {:#?}",
        array
      ),

      Type::FunctionProto(function_proto) => panic!(
        "should be decayed before conditional conversion: {:#?}",
        function_proto
      ),
      Type::Enum(e) => {
        // let underlying_type = e.underlying_type.clone();
        todo!("conditional conversion for enum types {:#?}", e)
      },
      Type::Record(_) | Type::Union(_) => {
        todo!("conditional conversion for complex types")
      },
    }
  }

  #[must_use]
  fn usual_arithmetic_conversion_unchecked(
    lhs: Expression,
    rhs: Expression,
  ) -> Result<(Expression, Expression, QualifiedType), Diag> {
    let lhs = lhs.promote();
    let rhs = rhs.promote();

    let (common_type, lhs_cast, rhs_cast) =
      match (lhs.unqualified_type(), rhs.unqualified_type()) {
        (Type::Primitive(l), Type::Primitive(r)) =>
          Primitive::common_type(l, r),
        _ => {
          let lhs_span = lhs.span();
          do yeet InvalidConversion(
            "usual arithmetic conversion only applies to arithmetic types"
              .to_string(),
          )
          .into_with(Severity::Error)
          .into_with(SourceSpan {
            end: rhs.span().end,
            ..lhs_span
          })
        },
      };

    let common_qtype = common_type.into();
    let lhs = Self::maybe_cast(lhs, lhs_cast, &common_qtype);
    let rhs = Self::maybe_cast(rhs, rhs_cast, &common_qtype);
    Ok((lhs, rhs, common_qtype))
  }

  /// used for unary `~`, `+` and `-`
  #[must_use]
  pub(super) fn usual_arithmetic_conversion_unary_unchecked(
    self,
  ) -> Result<Self, Diag> {
    let promoted = self.promote();
    match promoted.unqualified_type() {
      Type::Primitive(p) if p.is_arithmetic() => Ok(promoted),
      _ => Err(
        InvalidConversion(
          "usual arithmetic conversion only applies to arithmetic types"
            .to_string(),
        )
        .into_with(Severity::Error)
        .into_with(promoted.span()),
      ),
    }
  }
}

impl Expression {
  #[must_use]
  fn get_cast_type(from: &Type, to: &Type) -> CastType {
    match (from, to) {
      (from, to) if from == to => CastType::Noop,
      (Type::Primitive(from_prim), Type::Primitive(to_prim)) => {
        if from_prim.is_integer() && to_prim.is_integer() {
          CastType::IntegralCast
        } else if from_prim.is_integer() && to_prim.is_floating_point() {
          CastType::IntegralToFloating
        } else if from_prim.is_floating_point() && to_prim.is_integer() {
          CastType::FloatingToIntegral
        } else if from_prim.is_floating_point() && to_prim.is_floating_point() {
          CastType::FloatingCast
        } else {
          panic!("Invalid cast: {:?} -> {:?}", from_prim, to_prim)
        }
      },
      _ => panic!("Invalid cast: {:?} -> {:?}", from, to),
    }
  }

  /// if noop -> itself; else perform cast
  #[must_use]
  fn maybe_cast(
    self,
    cast_type: CastType,
    target_type: &QualifiedType,
  ) -> Expression {
    match cast_type {
      CastType::Noop => self,
      _ => {
        let span = self.span();
        Expression::new_rvalue(
          ImplicitCast::new(self.into(), cast_type, span).into(),
          target_type.clone(),
        )
      },
    }
  }

  #[must_use]
  fn cast_if_needed(self, target_type: &QualifiedType) -> Self {
    let cast_type = Self::get_cast_type(
      self.unqualified_type(),
      target_type.unqualified_type(),
    );
    self.maybe_cast(cast_type, target_type)
  }

  #[must_use]
  pub fn promote(self) -> Self {
    let (promoted_type, cast_type) = self.qualified_type().clone().promote();
    match cast_type {
      CastType::Noop => self,
      cast_type => {
        let span = self.span();
        Self::new_rvalue(
          ImplicitCast::new(self.into(), cast_type, span).into(),
          promoted_type,
        )
      },
    }
  }
}
