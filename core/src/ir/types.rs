/// IR Type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type<'ir> {
  Void,
  Label,
  IEEE32Float,
  IEEE64Float,

  Pointer,
  Integer(u8),
  Array(Array<'ir>),
  Function(Function<'ir>),
  // TODO: complete it later, placeholder now vvv
  Struct,
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
  result_type: TypeRef<'ir>,
  params: &'ir [TypeRef<'ir>],
  is_variadic: bool,
}

impl<'ir> Function<'ir> {
  pub fn new(
    result_type: TypeRef<'ir>,
    params: &'ir [TypeRef<'ir>],
    is_variadic: bool,
  ) -> Self {
    Self {
      result_type,
      params,
      is_variadic,
    }
  }
}

::rcc_utils::interconvert!(Array, Type, 'ir);
::rcc_utils::interconvert!(Function, Type, 'ir);

impl<'ir> Type<'ir> {
  #[inline]
  pub fn ref_eq(lhs: TypeRef<'ir>, rhs: TypeRef<'ir>) -> bool {
    if cfg!(debug_assertions) && !::std::ptr::eq(lhs, rhs) && lhs == rhs {
      eprintln!(
        "INTERNAL INVARIANT: comparing types by pointer but they are actually \
         the same: {:p}: {:?} and {:p}: {:?}.",
        lhs, lhs, rhs, rhs
      );
      return true;
    }
    ::std::ptr::eq(lhs, rhs)
  }
}
