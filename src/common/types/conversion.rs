//! applied during binary operations

use crate::common::{
  error::Error,
  types::{Primitive, Type, TypeInfo},
};

impl Primitive {
  /// 6.3.1.8 Usual arithmetic conversions
  pub fn usual_arithmetic_conversion(lhs: Primitive, rhs: Primitive) -> Result<Primitive, Error> {
    // first: _Decimal types ignored
    // also, complex types ignored
    // If both operands have the same type, then no further conversion is needed.
    if lhs == rhs {
      return Ok(lhs);
    }
    if matches!(lhs, Primitive::Void | Primitive::Nullptr)
      || matches!(rhs, Primitive::Void | Primitive::Nullptr)
    {
      return Err(()); // invalid
    }
    // otherwise, if either operand is of some floating type, the other operand is converted to it.
    // Otherwise, if any of the two types is an enumeration, it is converted to its underlying type. - handled upstream
    match (lhs.is_floating_point(), rhs.is_floating_point()) {
      (true, false) => Ok(lhs),
      (false, true) => Ok(rhs),
      (true, true) => Ok(Primitive::common_floating_rank(lhs, rhs)),
      (false, false) => Ok(Primitive::common_integer_rank(lhs, rhs)),
    }
  }
  fn common_floating_rank(lhs: Primitive, rhs: Primitive) -> Primitive {
    assert!(lhs.is_floating_point() && rhs.is_floating_point());
    if lhs.floating_rank() > rhs.floating_rank() {
      lhs
    } else {
      rhs
    }
  }
  fn common_integer_rank(lhs: Primitive, rhs: Primitive) -> Primitive {
    assert!(lhs.is_integer() && rhs.is_integer());

    let lhs = lhs.integer_promotion();
    let rhs = rhs.integer_promotion();
    if lhs == rhs {
      // done
      return lhs;
    }
    if lhs.is_unsigned() == rhs.is_unsigned() {
      return if lhs.integer_rank() > rhs.integer_rank() {
        lhs
      } else {
        rhs
      };
    }
    let (unsigned_oprand, signed_oprand) = if lhs.is_unsigned() {
      (lhs, rhs)
    } else {
      (rhs, lhs)
    };

    if unsigned_oprand.integer_rank() >= signed_oprand.integer_rank() {
      unsigned_oprand
    } else if signed_oprand.size() > unsigned_oprand.size() {
      signed_oprand
    } else {
      // if the signed type cannot represent all values of the unsigned type, return the unsigned version of the signed type
      // the signed type is always larger than the corresponding unsigned type on my x86_64 architecture
      // so this branch is unlikely to be taken
      signed_oprand.into_unsigned()
    }
  }
}

impl Type {
  #[must_use]
  pub fn usual_arithmetic_conversion(lhs: Type, rhs: Type) -> Result<Type, Error> {
    // upstream
    let lhs = if lhs.is_enum() {
      Type::Primitive(lhs.into_enum().unwrap().into_underlying_type())
    } else {
      lhs
    }
    .promote();
    let rhs = if rhs.is_enum() {
      Type::Primitive(rhs.into_enum().unwrap().into_underlying_type())
    } else {
      rhs
    }
    .promote();

    match (lhs, rhs) {
      (Type::Primitive(lhs), Type::Primitive(rhs)) => Ok(Type::Primitive(
        Primitive::usual_arithmetic_conversion(lhs, rhs)?,
      )),
      _ => panic!(),
    }
  }
}
