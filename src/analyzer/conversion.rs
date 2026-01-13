use crate::analyzer::expression::{Expression, ImplicitCast, RawExpr};
use crate::common::error::Error;
use crate::common::types::{
  CastType, Compatibility, Pointer, Primitive, Promotion, QualifiedType, Type,
};

impl Expression {
  /// 6.3.1.8 Usual arithmetic conversions, applied implicitly where arithmetic conversions are required
  ///
  /// unary/binary.
  #[must_use]
  pub fn usual_arithmetic_conversion(
    lhs: Expression,
    rhs: Expression,
  ) -> Result<(Expression, Expression, QualifiedType), Error> {
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
    let lhs = lhs.promote();
    let rhs = rhs.promote();

    let (common_type, lhs_cast, rhs_cast) = match (lhs.unqualified_type(), rhs.unqualified_type()) {
      (Type::Primitive(l), Type::Primitive(r)) => Primitive::common_type(l.clone(), r.clone()),
      _ => return Err(()),
    };

    let common_qtype = QualifiedType::new_unqualified(Type::Primitive(common_type));
    let lhs = Self::maybe_cast(lhs, lhs_cast, &common_qtype);
    let rhs = Self::maybe_cast(rhs, rhs_cast, &common_qtype);
    Ok((lhs, rhs, common_qtype))
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
  /// Returned expression is always rvalue of type bool.
  #[must_use]
  pub fn conditional_conversion(self) -> Result<Self, Error> {
    assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    match self.expr_type.unqualified_type {
      Type::Primitive(Primitive::Bool) => Ok(self),
      Type::Primitive(ref p) if p.is_integer() => Ok(Self::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::IntegralToBoolean)),
        QualifiedType::new_unqualified(Primitive::Bool.into()),
      )),
      Type::Primitive(ref p) if p.is_floating_point() => Ok(Self::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::FloatingToBoolean)),
        QualifiedType::new_unqualified(Primitive::Bool.into()),
      )),
      Type::Primitive(Primitive::Nullptr) => Ok(Self::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::NullptrToBoolean)),
        QualifiedType::new_unqualified(Primitive::Bool.into()),
      )),
      Type::Primitive(Primitive::Void) => {
        Err(()) // void cannot be converted to bool
      }
      Type::Primitive(_) => {
        Err(()) // other primitive types cannot be converted to bool
      }
      // compare with nullptr
      Type::Pointer(_) => Ok(Self::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::PointerToBoolean)),
        QualifiedType::new_unqualified(Primitive::Bool.into()),
      )),

      Type::Array(array) => panic!(
        "should be decayed before conditional conversion: {:?}",
        array
      ),

      Type::FunctionProto(function_proto) => panic!(
        "should be decayed before conditional conversion: {:?}",
        function_proto
      ),
      Type::Enum(_) | Type::Record(_) | Type::Union(_) => {
        todo!("conditional conversion for complex types")
      }
    }
  }
  /// If an expression of any other type is evaluated as a void expression, its value or designator is discarded.
  /// (A void expression is evaluated for its side effects.)
  ///
  /// unary.
  #[must_use]
  pub fn void_conversion(self) -> Self {
    Self::new_rvalue(
      RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::ToVoid)),
      QualifiedType::void(),
    )
  }
  /// 6.5.17.2 Simple assignment
  #[must_use]
  pub fn assignment_conversion(self, target_type: &QualifiedType) -> Result<Self, Error> {
    assert!(
      !matches!(
        self.unqualified_type(),
        Type::FunctionProto(_) | Type::Array(_)
      ),
      "perform decay first"
    );
    match (&target_type.unqualified_type, &self.unqualified_type()) {
      //  the left operand has [...] arithmetic type, and the right operand has arithmetic type;
      (Type::Primitive(left), Type::Primitive(right))
        if left.is_arithmetic() && right.is_arithmetic() =>
      {
        let cast_type = Self::get_cast_type(self.unqualified_type(), &target_type.unqualified_type);
        Ok(Self::maybe_cast(self, cast_type, target_type))
      }
      // the left operand has [...] of a structure or union type compatible with the type of the right operand;
      (Type::Record(_), Type::Record(_)) | (Type::Union(_), Type::Union(_)) => {
        todo!()
      }
      // the left operand has atomic, qualified, or unqualified pointer type,
      //    and (considering the type the left operand would have after lvalue conversion) one operand is a pointer to an object type,
      //    and the other is a pointer to a qualified or unqualified version of void,
      //    and the type pointed to by the left operand has all the qualifiers of the type pointed to by the right operand;
      (Type::Pointer(lhs), Type::Pointer(rhs)) => {
        // can add qualifiers, but cannot remove them
        // error if removing qualifiers (const, volatile, etc.)
        if !lhs.pointee.qualifiers.contains(rhs.pointee.qualifiers) {
          return Err(()); // discarding qualifiers
        }

        if lhs
          .pointee
          .unqualified_type
          .compatible_with(&rhs.pointee.unqualified_type)
          || lhs.pointee.unqualified_type.is_void()
          || rhs.pointee.unqualified_type.is_void()
        {
          // no need to create composite type -- pointer types are the same except for qualifiers
          Ok(Self::new_rvalue(
            RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::BitCast)),
            target_type.clone(),
          ))
        } else {
          Err(()) // incompatible pointer types
        }
      }
      // the left operand has an atomic, qualified, or unqualified version of the nullptr_t type and the right operand is a null pointer constant or its type is nullptr_t;
      (Type::Primitive(Primitive::Nullptr), Type::Primitive(Primitive::Nullptr)) => Ok(self),
      // the left operand is an atomic, qualified, or unqualified pointer, and the right operand is a null pointer constant or its type is nullptr_t;
      (Type::Pointer(_), Type::Primitive(Primitive::Nullptr)) => Ok(Self::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::NullptrToPointer)),
        target_type.clone(),
      )),

      // the left operand has atomic, qualified, or unqualified bool, and the right operand is a pointer or its type is nullptr_t.
      (Type::Primitive(Primitive::Bool), Type::Pointer(_)) => Ok(Self::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::PointerToBoolean)),
        target_type.clone(),
      )),
      (Type::Primitive(Primitive::Bool), Type::Primitive(Primitive::Nullptr)) => {
        Ok(Self::new_rvalue(
          RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::NullptrToBoolean)),
          target_type.clone(),
        ))
      }
      _ => Err(()), // other cases are invalid
    }
  }
  /// 6.3.2.1 Lvalues, arrays, and function designators
  ///
  /// Except when it is the operand of the sizeof operator, or the typeof operators, the unary `&` operator,
  ///     the `++` operator, the `--` operator, or the left operand of the `.` operator or an assignment operator, an
  ///     lvalue that does not have array type is converted to the value stored in the designated object (and is
  ///     no longer an lvalue); this is called lvalue conversion.
  #[must_use]
  pub fn lvalue_conversion(self) -> Self {
    if self.is_lvalue() {
      // If the lvalue has qualified type, the value has the unqualified version of the type of the lvalue. perform cast from lvalue to rvalue
      let old_unqual_type = self.expr_type.unqualified_type.clone();
      Self::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(self.into(), CastType::LValueToRValue)),
        QualifiedType::new_unqualified(old_unqual_type),
      )
    } else {
      self
    }
  }
  #[must_use]
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
      self.expr_type
    );
    let pointer_type = Type::from(Pointer::new(
      QualifiedType::new(
        self.qualifiers().clone(), // should equal to empty -- functionproto never has qualifiers
        Type::FunctionProto(function_type.clone()),
      )
      .into(),
    ));
    Self::new_rvalue(
      RawExpr::ImplicitCast(ImplicitCast::new(
        self.into(),
        CastType::FunctionToPointerDecay,
      )),
      // The pointer itself is never qualified
      QualifiedType::new_unqualified(pointer_type),
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
      self.expr_type
    );
    let pointer_type = Type::from(Pointer::new(
      // array itself should not have qualifiers, but the element qualifiers are preserved
      array_type.element_type.clone().into(),
    ));
    Self::new_rvalue(
      RawExpr::ImplicitCast(ImplicitCast::new(
        self.into(),
        CastType::ArrayToPointerDecay,
      )),
      // The pointer itself is never qualified
      QualifiedType::new_unqualified(pointer_type),
    )
  }

  fn get_cast_type(from: &Type, to: &Type) -> CastType {
    match (from, to) {
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
      }
      _ => panic!("Invalid cast: {:?} -> {:?}", from, to),
    }
  }
  /// if noop -> itself; else perform cast
  fn maybe_cast(expr: Expression, cast_type: CastType, target_type: &QualifiedType) -> Expression {
    match cast_type {
      CastType::Noop => expr,
      _ => Expression::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(expr.into(), cast_type)),
        target_type.clone(),
      ),
    }
  }
  pub fn promote(self) -> Self {
    let (promoted_type, cast_type) = self.expr_type.clone().promote();
    match cast_type {
      CastType::Noop => self,
      cast_type => Self::new_rvalue(
        RawExpr::ImplicitCast(ImplicitCast::new(self.into(), cast_type)),
        promoted_type,
      ),
    }
  }
}
