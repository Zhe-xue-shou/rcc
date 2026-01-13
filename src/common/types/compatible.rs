#![allow(unused)]
use super::{
  Array, ArraySize, Compatibility, Enum, FunctionProto, Pointer, Primitive, QualifiedType, Record,
  Type, Union,
};
use crate::breakpoint;

impl Compatibility for ArraySize {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(r)) => l == r,
      (Self::Incomplete, Self::Incomplete)
      | (Self::Constant(_), Self::Incomplete)
      | (Self::Incomplete, Self::Constant(_)) => true,
    }
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(r)) => {
        if l == r {
          Some(Self::Constant(*l))
        } else {
          None
        }
      }
      (Self::Incomplete, Self::Incomplete) => Some(Self::Incomplete),
      (Self::Constant(l), Self::Incomplete) | (Self::Incomplete, Self::Constant(l)) => {
        Some(Self::Constant(*l))
      }
    }
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(_)) => Self::Constant(*l),
      (Self::Incomplete, Self::Incomplete) => Self::Incomplete,
      (Self::Constant(l), Self::Incomplete) | (Self::Incomplete, Self::Constant(l)) => {
        Self::Constant(*l)
      }
    }
  }
}

impl Compatibility for Enum {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    todo!()
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    todo!()
  }

  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
}

impl Compatibility for Record {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    todo!()
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    todo!()
  }

  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
}

impl Compatibility for Pointer {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    QualifiedType::compatible(&lhs.pointee, &rhs.pointee)
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      return None;
    }
    let pointee = QualifiedType::composite_unchecked(&lhs.pointee, &rhs.pointee);
    Some(Self::new(Box::new(pointee)))
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    let pointee = QualifiedType::composite_unchecked(&lhs.pointee, &rhs.pointee);
    Self::new(Box::new(pointee))
  }
}

impl Compatibility for Primitive {
  #[inline]
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    lhs == rhs
  }

  #[inline]
  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs))
    }
  }
  #[inline]
  fn composite_unchecked(lhs: &Self, _rhs: &Self) -> Self
  where
    Self: Sized,
  {
    lhs.clone()
  }
}

impl Compatibility for FunctionProto {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    if lhs.is_variadic != rhs.is_variadic {
      return false;
    }
    // 6.7.7.4.13: For two function types to be compatible, both shall specify compatible return types.
    if !QualifiedType::compatible(&lhs.return_type, &rhs.return_type) {
      return false;
    }
    if lhs.parameter_types.len() != rhs.parameter_types.len() {
      return false;
    }
    // THIS IS A NASTY EXCEPTION:
    //  In the determination of type compatibility and of a composite type,
    //     each parameter declared with function or array type is taken as having the
    //     adjusted type and each parameter declared with qualified type is taken as having the unqualified
    //     version of its declared type.
    for (lparam, rparam) in lhs.parameter_types.iter().zip(rhs.parameter_types.iter()) {
      if !Type::compatible(&lparam.unqualified_type, &rparam.unqualified_type) {
        return false;
      }
    }

    true
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      return None;
    } else {
      Some(Self::composite_unchecked(lhs, rhs))
    }
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    let return_type = QualifiedType::composite_unchecked(&lhs.return_type, &rhs.return_type);
    let mut parameter_types = Vec::new();
    for (lparam, rparam) in lhs.parameter_types.iter().zip(rhs.parameter_types.iter()) {
      let param_type = QualifiedType::new(
        // this is actually not strictly correct -
        // e.g., const decl + non-const def -> var is const, non-const decl + const def -> var is non-const
        lparam.qualifiers | rparam.qualifiers,
        Type::composite_unchecked(&lparam.unqualified_type, &rparam.unqualified_type),
      );
      parameter_types.push(param_type);
    }
    Self::new(Box::new(return_type), parameter_types, lhs.is_variadic)
  }
}

impl Compatibility for Type {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    match (lhs, rhs) {
      (Type::Primitive(l), Type::Primitive(r)) => Primitive::compatible(l, r),
      (Type::Pointer(l), Type::Pointer(r)) => Pointer::compatible(l, r),
      (Type::Array(l), Type::Array(r)) => Array::compatible(l, r),
      (Type::FunctionProto(l), Type::FunctionProto(r)) => FunctionProto::compatible(l, r),
      (Type::Enum(l), Type::Enum(r)) => Enum::compatible(l, r),
      (Type::Record(l), Type::Record(r)) => Record::compatible(l, r),
      (Type::Union(l), Type::Union(r)) => Union::compatible(l, r),
      _ => false,
    }
  }

  #[inline]
  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs))
    }
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Type::Primitive(l), Type::Primitive(r)) => {
        Type::Primitive(Primitive::composite_unchecked(l, r))
      }
      (Type::Pointer(l), Type::Pointer(r)) => Type::Pointer(Pointer::composite_unchecked(l, r)),
      (Type::Array(l), Type::Array(r)) => Type::Array(Array::composite_unchecked(l, r)),
      (Type::FunctionProto(l), Type::FunctionProto(r)) => {
        Type::FunctionProto(FunctionProto::composite_unchecked(l, r))
      }
      (Type::Enum(l), Type::Enum(r)) => Type::Enum(Enum::composite_unchecked(l, r)),
      (Type::Record(l), Type::Record(r)) => Type::Record(Record::composite_unchecked(l, r)),
      (Type::Union(l), Type::Union(r)) => Type::Union(Union::composite_unchecked(l, r)),
      _ => {
        breakpoint!();
        unreachable!()
      }
    }
  }
}

impl Compatibility for QualifiedType {
  fn compatible(lhs: &QualifiedType, rhs: &QualifiedType) -> bool {
    // 6.2.7.1: Two types are compatible types if they are the same.
    if lhs == rhs {
      return true;
    }
    // 6.7.4.1.11: For two qualified types to be compatible, both shall have the identically qualified version of a compatible type.
    if lhs.qualifiers != rhs.qualifiers {
      return false;
    }
    <Type as Compatibility>::compatible(&lhs.unqualified_type, &rhs.unqualified_type)
  }
  #[inline]
  fn composite(lhs: &QualifiedType, rhs: &QualifiedType) -> Option<QualifiedType> {
    if !QualifiedType::compatible(lhs, rhs) {
      return None;
    }
    Some(Self::composite_unchecked(lhs, rhs))
  }
  fn composite_unchecked(lhs: &QualifiedType, rhs: &QualifiedType) -> QualifiedType
  where
    Self: Sized,
  {
    // there's some nasty rules about merging qualifiers for function types -- handled in analyzer
    // also nasty rules about arrays -- todo
    // struct, enum, union -- todo
    // alignment specifier -- don't care
    // QualifiedType::new(qualifiers, lhs.unqualified_type.clone())
    todo!()
  }
}

impl Compatibility for Array {
  #[inline]
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    if !QualifiedType::compatible(&lhs.element_type, &rhs.element_type) {
      false
    } else {
      ArraySize::compatible(&lhs.size, &rhs.size)
    }
  }

  #[inline]
  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs))
    }
  }
  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    let element_type =
      <QualifiedType as Compatibility>::composite_unchecked(&lhs.element_type, &rhs.element_type);
    let size = ArraySize::composite_unchecked(&lhs.size, &rhs.size);
    Self::new(Box::new(element_type), size)
  }
}

impl Compatibility for Union {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    todo!()
  }

  fn composite(lhs: &Self, rhs: &Self) -> Option<Self>
  where
    Self: Sized,
  {
    todo!()
  }

  fn composite_unchecked(lhs: &Self, rhs: &Self) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
}
