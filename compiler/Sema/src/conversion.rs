#![allow(clippy::double_must_use)]

use ::rcc_ast::{
  Context,
  types::{
    CastType, Compatibility, Pointer, Primitive, Promotion, QualifiedType,
    Type, TypeInfo,
  },
};
use ::rcc_shared::{Diag, DiagData::*, DiagMeta, Severity, SourceSpan};
use ::rcc_utils::{IntoWith, RefEq};

use super::expression::{ExprRef, Expression, ImplicitCast};

impl<'c> Expression<'c> {
  /// 6.3.1.8 Usual arithmetic conversions, applied implicitly where arithmetic conversions are required
  #[must_use]
  #[inline]
  pub fn usual_arithmetic_conversion(
    lhs: ExprRef<'c>,
    rhs: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> Result<(ExprRef<'c>, ExprRef<'c>, QualifiedType<'c>), Diag<'c>> {
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
    Self::usual_arithmetic_conversion_unchecked(lhs, rhs, context)
  }

  #[must_use]
  #[inline]
  pub fn usual_arithmetic_conversion_unary(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    assert!(!self.is_lvalue(), "perform lvalue conversion first");
    assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    Self::usual_arithmetic_conversion_unary_unchecked(self, context)
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
  /// # Important note
  /// boolean conversion did not occur of the condition check in `if`, `while`, `for` and `do-while` statements,
  /// and `!`, `&&` and `||` operators;
  /// instead, at AST level, the condition is required to be contextually convertible to bool,
  /// and the actual boolean conversion is performed at IR level.
  ///
  #[must_use]
  #[inline]
  pub fn boolean_conversion(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    Self::boolean_conversion_unchecked(self, context)
  }

  #[must_use]
  #[inline]
  pub fn is_contextually_convertible_to_bool(
    self: ExprRef<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    assert!(
      !self.is_lvalue(),
      "perform lvalue_conversion() first; an lvalue is not contextually \
       convertible to bool"
    );
    match self.qualified_type().is_scalar() {
      true => Ok(self),
      false => Err(
        InvalidConversion(format!(
          "type {} is not contexaully convertible to int",
          self.qualified_type()
        ))
        .into_with(Severity::Error)
        .into_with(self.span()),
      ),
    }
  }

  /// If an expression of any other type is evaluated as a void expression, its value or designator is discarded.
  /// (A void expression is evaluated for its side effects -- of course we need to evaluate it!)
  ///
  /// unary.
  #[must_use]
  #[inline]
  pub fn void_conversion(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> ExprRef<'c> {
    match self.unqualified_type().is_void() {
      true => self,
      false => {
        let span = self.span();

        Self::new_rvalue(
          context,
          ImplicitCast::new(self, CastType::ToVoid),
          context.void_type().into(),
          span,
        )
      },
    }
  }

  /// 6.5.17.2 Simple assignment
  #[must_use]
  #[inline]
  pub fn assignment_conversion(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
    target_type: &QualifiedType<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    Self::assignment_conversion_unchecked(self, target_type, context)
  }

  /// 6.3.2.1 Lvalues, arrays, and function designators
  ///
  /// Except when it is the operand of the sizeof operator, or the typeof operators, the unary `&` operator,
  ///     the `++` operator, the `--` operator, or the left operand of the `.` operator or an assignment operator, an
  ///     lvalue that does not have array type is converted to the value stored in the designated object (and is
  ///     no longer an lvalue); this is called lvalue conversion.
  #[must_use]
  pub fn lvalue_conversion(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> ExprRef<'c> {
    if self.unqualified_type().is_array()
      || self.unqualified_type().is_functionproto()
    {
      // these types are not converted by lvalue conversion
      self
    } else if self.is_lvalue() {
      // If the lvalue has qualified type, the value has the unqualified version of the type of the lvalue. perform cast from lvalue to rvalue
      let old_unqual_type = self.unqualified_type();
      let span = self.span();

      Self::new_rvalue(
        context,
        ImplicitCast::new(self, CastType::LValueToRValue),
        old_unqual_type.into(),
        span,
      )
    } else {
      self
    }
  }

  #[must_use]
  #[inline]
  pub fn decay(self: ExprRef<'c>, context: &'c Context<'c>) -> ExprRef<'c> {
    match self.unqualified_type() {
      Type::Array(_) => self.array_to_pointer_decay(context),
      Type::FunctionProto(_) => self.function_to_pointer_decay(context),
      _ => self,
    }
  }

  /// A function designator is an expression that has function type. Except when it is the operand of the
  ///     sizeof operator, a typeof operator, or the unary & operator, a function designator with type
  ///     "function returning type" is converted to an expression that has type "pointer to function returning
  ///     type"
  #[must_use]
  pub fn function_to_pointer_decay(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> ExprRef<'c> {
    assert!(
      self.qualifiers().is_empty(),
      "function type should not have qualifiers: {:?}",
      self.qualified_type()
    );
    let pointer_type =
      Type::Pointer(Pointer::new(*self.qualified_type())).lookup(context);
    let span = self.span();

    Self::new_rvalue(
      context,
      ImplicitCast::new(self, CastType::FunctionToPointerDecay),
      // The pointer itself is never qualified
      pointer_type.into(),
      span,
    )
  }

  /// Except when it is the operand of the `sizeof` operator, or `typeof` operators, or the unary `&` operator,
  ///       or is a string literal used to initialize an array, an expression that has type `array of type` is converted
  ///       to an expression with type `pointer to type` that points to the initial element of the array object and
  ///       is not an lvalue.
  #[must_use]
  pub fn array_to_pointer_decay(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> ExprRef<'c> {
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
      array_type.element_type,
    ));
    let span = self.span();
    Self::new_rvalue(
      context,
      ImplicitCast::new(self, CastType::ArrayToPointerDecay),
      // The pointer itself is never qualified
      pointer_type.lookup(context).into(),
      span,
    )
  }

  /// I didnt find this in C standard, but it's a handy conversion for pointer arithmetic with numeric types (e.g. `ptr + 1`).
  #[must_use]
  #[inline]
  pub fn uintptr_conversion(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    debug_assert!(!self.is_lvalue(), "perform lvalue conversion first");
    debug_assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    if !self.unqualified_type().is_integer() {
      Err(
        InvalidConversion(
          "pointer conversion only applies to integer types".to_string(),
        )
        .into_with(Severity::Error)
        .into_with(self.span()),
      )
    } else {
      Ok(Self::uintptr_conversion_unchecked(self, context))
    }
  }

  #[must_use]
  #[inline]
  pub fn ptrdiff_conversion(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    debug_assert!(!self.is_lvalue(), "perform lvalue conversion first");
    debug_assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    if !self.unqualified_type().is_integer() {
      Err(
        InvalidConversion(
          "pointer conversion only applies to integer types".to_string(),
        )
        .into_with(Severity::Error)
        .into_with(self.span()),
      )
    } else {
      Ok(Self::ptrdiff_conversion_unchecked(self, context))
    }
  }
}
// unchecked version is requireds to use w.r.t. `&`, `sizeof`, `alignof`, etc.
impl<'c> Expression<'c> {
  #[must_use]
  fn assignment_conversion_unchecked(
    self: ExprRef<'c>,
    target_type: &QualifiedType<'c>,
    context: &'c Context<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    let span = self.span();

    match (target_type.unqualified_type, self.unqualified_type()) {
      //  the left operand has [...] arithmetic type, and the right operand has arithmetic type;
      (Type::Primitive(left), Type::Primitive(right))
        if left.is_arithmetic() && right.is_arithmetic() =>
      {
        let cast_type = Self::get_cast_type(
          self.unqualified_type(),
          target_type.unqualified_type,
        );
        Ok(Self::maybe_cast(self, cast_type, target_type, context))
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
        if !lhs.pointee.qualifiers.contains(rhs.pointee.qualifiers) {
          Err(
            DiscardingQualifiers(
              (rhs.pointee.qualifiers - lhs.pointee.qualifiers).to_string(),
            )
            .into_with(Severity::Error)
            .into_with(span),
          )?
        }

        if lhs
          .pointee
          .unqualified_type
          .compatible_with(rhs.pointee.unqualified_type)
          || lhs.pointee.unqualified_type.is_void()
          || rhs.pointee.unqualified_type.is_void()
        {
          // no need to create composite type -- pointer types are the same except for qualifiers
          // Ok(Self::new_rvalue(
          //   ImplicitCast::new(self.into(), CastType::BitCast, span).into(),
          //   *target_type,
          // ))
          Ok(self) // Noop? not sure
        } else {
          Err(
            IncompatiblePointerTypes(
              target_type.to_string(),
              self.qualified_type().to_string(),
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
          context,
          ImplicitCast::new(self, CastType::NullptrToPointer),
          *target_type,
          span,
        )),

      // the left operand has atomic, qualified, or unqualified bool, and the right operand is a pointer or its type is nullptr_t.
      (Type::Primitive(Primitive::Bool), Type::Pointer(_)) =>
        Ok(Self::new_rvalue(
          context,
          ImplicitCast::new(self, CastType::PointerToBoolean),
          *target_type,
          span,
        )),
      (
        Type::Primitive(Primitive::Bool),
        Type::Primitive(Primitive::Nullptr),
      ) => Ok(Self::new_rvalue(
        context,
        ImplicitCast::new(self, CastType::BitCast),
        *target_type,
        span,
      )),
      _ => Err(
        InvalidConversion(format!(
          "cannot convert from '{}' to '{}'",
          self.unqualified_type(),
          target_type.unqualified_type
        ))
        .into_with(Severity::Error)
        .into_with(self.span()),
      ),
    }
  }

  #[must_use]
  fn boolean_conversion_unchecked(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    let span = self.span();

    match self.unqualified_type() {
      Type::Primitive(Primitive::Bool) => Ok(Self::cast_if_needed(
        self,
        &context.i8_bool_type().into(),
        context,
      )),
      Type::Primitive(p) if p.is_integer() => Ok(Self::cast_if_needed(
        self,
        &context.i8_bool_type().into(),
        context,
      )),
      Type::Primitive(p) if p.is_floating_point() => Ok(Self::new_rvalue(
        context,
        ImplicitCast::new(self, CastType::FloatingToIntegral),
        context.i8_bool_type().into(),
        span,
      )),
      Type::Primitive(Primitive::Nullptr) => Err(
        InvalidConversion(
          "cannot cast an object of type nullptr to integral.".to_string(),
        )
        .into_with(Severity::Error)
        .into_with(self.span()),
      ),
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
        context,
        ImplicitCast::new(self, CastType::PointerToIntegral),
        context.i8_bool_type().into(),
        span,
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
        todo!("conditional conversion for enum types {:#?}", e)
      },
      Type::Record(_) | Type::Union(_) => {
        todo!("conditional conversion for complex types")
      },
    }
  }

  #[must_use]
  fn usual_arithmetic_conversion_unchecked(
    lhs: ExprRef<'c>,
    rhs: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> Result<(ExprRef<'c>, ExprRef<'c>, QualifiedType<'c>), Diag<'c>> {
    let lhs = lhs.promote(context);
    let rhs = rhs.promote(context);

    let (common_type, lhs_cast, rhs_cast) =
      match (lhs.unqualified_type(), rhs.unqualified_type()) {
        (Type::Primitive(l), Type::Primitive(r)) =>
          Primitive::common_type(l, r),
        _ => {
          let lhs_span = lhs.span();
          Err(
            InvalidConversion(
              "usual arithmetic conversion only applies to arithmetic types"
                .to_string(),
            )
            .into_with(Severity::Error)
            .into_with(SourceSpan {
              end: rhs.span().end,
              ..lhs_span
            }),
          )?
        },
      };

    let common_qtype = Type::Primitive(common_type).lookup(context).into();
    let lhs = Self::maybe_cast(lhs, lhs_cast, &common_qtype, context);
    let rhs = Self::maybe_cast(rhs, rhs_cast, &common_qtype, context);
    Ok((lhs, rhs, common_qtype))
  }

  /// used for unary `~`, `+` and `-`
  #[must_use]
  pub(super) fn usual_arithmetic_conversion_unary_unchecked(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> Result<ExprRef<'c>, Diag<'c>> {
    let promoted = self.promote(context);
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

  #[must_use]
  pub(super) fn uintptr_conversion_unchecked(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> ExprRef<'c> {
    let cast_type =
      Self::get_cast_type(self.unqualified_type(), context.uintptr_type());
    Self::maybe_cast(self, cast_type, &context.uintptr_type().into(), context)
  }

  #[must_use]
  pub(super) fn ptrdiff_conversion_unchecked(
    self: ExprRef<'c>,
    context: &'c Context<'c>,
  ) -> ExprRef<'c> {
    debug_assert!(self.unqualified_type().is_integer());
    let cast_type = Self::get_cast_type(
      self.unqualified_type(),
      Context::ptrdiff_type(context),
    );
    Self::maybe_cast(self, cast_type, &context.ptrdiff_type().into(), context)
  }
}

/// Heplers.
impl<'c> Expression<'c> {
  #[must_use]
  pub fn get_cast_type(from: &Type, to: &Type) -> CastType {
    Self::try_get_cast_type(from, to).unwrap()
  }

  /// i forgot the logic of the orig cast_type, so i create new one here for compound assignment op/assignment? im going nuts!
  pub fn try_get_cast_type(
    from: &Type,
    to: &Type,
  ) -> Result<CastType, DiagMeta<'c>> {
    use CastType::*;
    use Type::Primitive as P;
    // use Type::Pointer as Ptr;
    match (from, to) {
      (from, to) if RefEq::ref_eq(from, to) => Ok(Noop),
      (P(f), P(t)) if f.is_integer() && t.is_integer() => Ok(IntegralCast),
      (P(f), P(t)) if f.is_integer() && t.is_floating_point() =>
        Ok(IntegralToFloating),
      (P(f), P(t)) if f.is_floating_point() && t.is_integer() =>
        Ok(FloatingToIntegral),
      (P(f), P(t)) if f.is_floating_point() && t.is_floating_point() =>
        Ok(FloatingCast),
      // (P(i), Ptr(_)) if i.is_integer() => Ok(Noop), // attn
      _ => Err(
        InvalidConversion(format!(
          "cannot convert from '{}' to '{}'",
          from, to
        ))
        .into_with(Severity::Error),
      ),
    }
  }

  /// if noop -> itself; else perform cast
  #[must_use]
  fn maybe_cast(
    self: ExprRef<'c>,
    cast_type: CastType,
    target_type: &QualifiedType<'c>,
    context: &'c Context<'c>,
  ) -> ExprRef<'c> {
    match cast_type {
      CastType::Noop => self,
      _ => {
        let span = self.span();
        Expression::new_rvalue(
          context,
          ImplicitCast::new(self, cast_type),
          *target_type,
          span,
        )
      },
    }
  }

  #[must_use]
  fn cast_if_needed(
    self: ExprRef<'c>,
    target_type: &QualifiedType<'c>,
    context: &'c Context<'c>,
  ) -> ExprRef<'c> {
    let cast_type = Self::get_cast_type(
      self.unqualified_type(),
      target_type.unqualified_type,
    );
    self.maybe_cast(cast_type, target_type, context)
  }

  #[must_use]
  pub fn promote(self: ExprRef<'c>, context: &'c Context<'c>) -> ExprRef<'c> {
    let (promoted_type, cast_type) = self.unqualified_type().clone().promote();
    match cast_type {
      CastType::Noop => self,
      cast_type => {
        let span = self.span();

        Self::new_rvalue(
          context,
          ImplicitCast::new(self, cast_type),
          promoted_type.lookup(context).into(),
          span,
        )
      },
    }
  }
}
