/// IR Type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type<'ir> {
  Void(),
  Label(),
  Float(),
  Double(),

  Pointer(),
  Integer(u8),
  Array(Array<'ir>),
  Function(Function<'ir>),
  // TODO: complete it later, placeholder now vvv
  Struct(Struct<'ir>),
}

pub type TypeRef<'ir> = &'ir Type<'ir>;
pub type TypeRefMut<'ir> = &'ir mut Type<'ir>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]

pub struct Array<'ir> {
  element_type: TypeRef<'ir>,
  length: usize,
}

impl<'ir> Array<'ir> {
  pub fn new(element_type: TypeRef<'ir>, length: usize) -> Self {
    Self {
      element_type,
      length,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Function<'ir> {
  pub return_type: TypeRef<'ir>,
  pub params: &'ir [TypeRef<'ir>],
  pub is_variadic: bool,
}

impl<'ir> Function<'ir> {
  pub fn new(
    result_type: TypeRef<'ir>,
    params: &'ir [TypeRef<'ir>],
    is_variadic: bool,
  ) -> Self {
    Self {
      return_type: result_type,
      params,
      is_variadic,
    }
  }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Struct<'ir> {
  _placeholder: &'ir ::std::marker::PhantomData<i8>,
}
use ::rcc_utils::{interconvert, make_trio_for, make_trio_for_unit_tuple};

use crate::common::RefEq;

interconvert!(Array, Type, 'ir);
interconvert!(Function, Type, 'ir);
interconvert!(Struct, Type, 'ir);

make_trio_for_unit_tuple!(Void, Type<'ir>);
make_trio_for_unit_tuple!(Label, Type<'ir>);
make_trio_for_unit_tuple!(Float, Type<'ir>);
make_trio_for_unit_tuple!(Double, Type<'ir>);
make_trio_for_unit_tuple!(Pointer, Type<'ir>);

make_trio_for!(Array, Type, 'ir);
make_trio_for!(Function, Type, 'ir);
make_trio_for!(Struct, Type, 'ir);

impl RefEq for TypeRef<'_> {
  fn ref_eq(lhs: Self, rhs: Self) -> bool
  where
    Self: PartialEq + Sized,
  {
    let ref_eq = ::std::ptr::eq(lhs, rhs);
    if const { cfg!(debug_assertions) } {
      let actual_eq = lhs == rhs;
      if ref_eq != actual_eq {
        eprintln!(
          "INTERNAL ERROR: comparing by pointer address result did not match 
          the actual result: {:p}: {:?} and {:p}: {:?}
        ",
          lhs, lhs, rhs, rhs
        );
      }
      return actual_eq;
    }
    ref_eq
  }
}
