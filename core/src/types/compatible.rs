#![allow(unused_variables)]

use ::rcc_utils::breakpoint;

use super::{
  Array, ArraySize, Context, Enum, FunctionProto, Pointer, Primitive,
  QualifiedType, Qualifiers, Record, Type, Union,
};

/// rules about the `metadata`. used for declaration and definition.
pub trait Compatibility<'context> {
  fn compatible(lhs: &Self, rhs: &Self) -> bool
  where
    Self: Sized;
  #[inline(always)]
  fn compatible_with(&self, other: &Self) -> bool
  where
    Self: Sized,
  {
    Self::compatible(self, other)
  }
  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized;
  #[inline(always)]
  fn composite_with(
    &self,
    other: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    Self::composite(self, other, context)
  }
  /// used internally to avoid unnecessary compatibility checks
  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized;
  #[inline(always)]
  fn composite_unchecked_with(
    &self,
    other: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    Self::composite_unchecked(self, other, context)
  }
}
impl<'context> Compatibility<'context> for ArraySize {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(r)) => l == r,
      (Self::Incomplete, Self::Incomplete)
      | (Self::Constant(_), Self::Incomplete)
      | (Self::Incomplete, Self::Constant(_)) => true,
      _ => todo!("variable array size compatible"),
    }
  }

  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(r)) =>
        if l == r {
          Some(Self::Constant(*l))
        } else {
          None
        },
      (Self::Incomplete, Self::Incomplete) => Some(Self::Incomplete),
      (Self::Constant(l), Self::Incomplete)
      | (Self::Incomplete, Self::Constant(l)) => Some(Self::Constant(*l)),
      _ => todo!("variable array size composite"),
    }
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Self::Constant(l), Self::Constant(_)) => Self::Constant(*l),
      (Self::Incomplete, Self::Incomplete) => Self::Incomplete,
      (Self::Constant(l), Self::Incomplete)
      | (Self::Incomplete, Self::Constant(l)) => Self::Constant(*l),
      _ => todo!("variable array size composite_unchecked"),
    }
  }
}

impl<'context> Compatibility<'context> for Enum<'context> {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    todo!()
  }

  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    todo!()
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
}

impl<'context> Compatibility<'context> for Record<'context> {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    todo!()
  }

  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    todo!()
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
}

impl<'context> Compatibility<'context> for Pointer<'context> {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    QualifiedType::compatible(&lhs.pointee, &rhs.pointee)
  }

  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::new(QualifiedType::composite_unchecked(
        &lhs.pointee,
        &rhs.pointee,
        context,
      )))
    }
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    Self::new(QualifiedType::composite_unchecked(
      &lhs.pointee,
      &rhs.pointee,
      context,
    ))
  }
}

impl<'context> Compatibility<'context> for Primitive {
  #[inline]
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    lhs == rhs
  }

  #[inline]
  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs, context))
    }
  }

  #[inline]
  fn composite_unchecked(
    lhs: &Self,
    _rhs: &Self,
    _context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    *lhs
  }
}

impl<'context> Compatibility<'context> for FunctionProto<'context> {
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
    for (lparam, rparam) in
      lhs.parameter_types.iter().zip(rhs.parameter_types.iter())
    {
      if !Compatibility::compatible(
        lparam.unqualified_type,
        rparam.unqualified_type,
      ) {
        return false;
      }
    }

    true
  }

  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs, context))
    }
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    let return_type = QualifiedType::composite_unchecked(
      &lhs.return_type,
      &rhs.return_type,
      context,
    );
    let mut parameter_types = context.alloc_vec(lhs.parameter_types.len());
    for (lparam, rparam) in
      lhs.parameter_types.iter().zip(rhs.parameter_types.iter())
    {
      let param_type = QualifiedType::new(
        // this is actually not strictly correct -
        // e.g., const decl + non-const def -> var is const, non-const decl + const def -> var is non-const
        lparam.qualifiers | rparam.qualifiers,
        Compatibility::composite_unchecked(
          lparam.unqualified_type,
          rparam.unqualified_type,
          context,
        )
        .lookup(context),
      );
      parameter_types.push(param_type);
    }
    Self::new(
      return_type,
      parameter_types.into_bump_slice(),
      lhs.is_variadic,
    )
  }
}

impl<'context> Compatibility<'context> for Type<'context> {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    match (lhs, rhs) {
      (Type::Primitive(l), Type::Primitive(r)) => Primitive::compatible(l, r),
      (Type::Pointer(l), Type::Pointer(r)) => Pointer::compatible(l, r),
      (Type::Array(l), Type::Array(r)) => Array::compatible(l, r),
      (Type::FunctionProto(l), Type::FunctionProto(r)) =>
        FunctionProto::compatible(l, r),
      (Type::Enum(l), Type::Enum(r)) => Enum::compatible(l, r),
      (Type::Record(l), Type::Record(r)) => Record::compatible(l, r),
      (Type::Union(l), Type::Union(r)) => Union::compatible(l, r),
      _ => false,
    }
  }

  #[inline]
  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs, context))
    }
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    match (lhs, rhs) {
      (Type::Primitive(l), Type::Primitive(r)) =>
        Type::Primitive(Primitive::composite_unchecked(l, r, context)),
      (Type::Pointer(l), Type::Pointer(r)) =>
        Type::Pointer(Pointer::composite_unchecked(l, r, context)),
      (Type::Array(l), Type::Array(r)) =>
        Type::Array(Array::composite_unchecked(l, r, context)),
      (Type::FunctionProto(l), Type::FunctionProto(r)) =>
        Type::FunctionProto(FunctionProto::composite_unchecked(l, r, context)),
      (Type::Enum(l), Type::Enum(r)) =>
        Type::Enum(Enum::composite_unchecked(l, r, context)),
      (Type::Record(l), Type::Record(r)) =>
        Type::Record(Record::composite_unchecked(l, r, context)),
      (Type::Union(l), Type::Union(r)) =>
        Type::Union(Union::composite_unchecked(l, r, context)),
      _ => {
        breakpoint!();
        unreachable!()
      },
    }
  }
}

impl<'context> Compatibility<'context> for QualifiedType<'context> {
  fn compatible(lhs: &QualifiedType, rhs: &QualifiedType) -> bool {
    // 6.2.7.1: Two types are compatible types if they are the same.
    if Type::ref_eq(lhs, rhs) {
      return true;
    }
    // 6.7.4.1.11: For two qualified types to be compatible, both shall have the identically qualified version of a compatible type.
    if lhs.qualifiers != rhs.qualifiers {
      return false;
    }
    Compatibility::compatible(lhs.unqualified_type, rhs.unqualified_type)
  }

  #[inline]
  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self> {
    if !QualifiedType::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs, context))
    }
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    debug_assert!(
      lhs.qualifiers == rhs.qualifiers
        && lhs.qualifiers | rhs.qualifiers == lhs.qualifiers,
      "idk, but they should be equal"
    );

    // function and array types cannot have qualifiers
    if lhs.unqualified_type.is_array()
      || lhs.unqualified_type.is_functionproto()
    {
      debug_assert!(
        lhs.qualifiers == Qualifiers::empty()
          && rhs.qualifiers == Qualifiers::empty(),
        "array and function types cannot have qualifiers"
      );
    }

    // struct, enum, union -- todo
    // alignment specifier -- won't care

    QualifiedType::new(
      lhs.qualifiers | rhs.qualifiers,
      Compatibility::composite_unchecked(
        lhs.unqualified_type,
        rhs.unqualified_type,
        context,
      )
      .lookup(context),
    )
  }
}

impl<'context> Compatibility<'context> for Array<'context> {
  #[inline]
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    if !QualifiedType::compatible(&lhs.element_type, &rhs.element_type) {
      false
    } else {
      ArraySize::compatible(&lhs.size, &rhs.size)
    }
  }

  #[inline]
  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    if !Self::compatible(lhs, rhs) {
      None
    } else {
      Some(Self::composite_unchecked(lhs, rhs, context))
    }
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    let element_type = Compatibility::composite_unchecked(
      &lhs.element_type,
      &rhs.element_type,
      context,
    );
    Self::new(
      element_type,
      ArraySize::composite_unchecked(&lhs.size, &rhs.size, context),
    )
  }
}

impl<'context> Compatibility<'context> for Union<'context> {
  fn compatible(lhs: &Self, rhs: &Self) -> bool {
    todo!()
  }

  fn composite(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Option<Self>
  where
    Self: Sized,
  {
    todo!()
  }

  fn composite_unchecked(
    lhs: &Self,
    rhs: &Self,
    context: &Context<'context>,
  ) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
}
