use ::bumpalo::Bump;
use ::rcc_utils::IntoWith;
use ::std::{cell::RefCell, collections::HashSet};

use super::{
  Array, ArraySize, Compatibility, FunctionProto, FunctionSpecifier, Pointer,
  Primitive, QualifiedType, Type, TypeRef,
};
use crate::common::StrRef;
#[derive(Debug)]
pub struct Context<'context> {
  arena: &'context Bump,
  type_interner: RefCell<HashSet<&'context Type<'context>>>,
  string_interner: RefCell<HashSet<StrRef<'context>>>,

  nullptr_type: TypeRef<'context>,
  void_type: TypeRef<'context>,
  bool_type: TypeRef<'context>,
  char_type: TypeRef<'context>,
  short_type: TypeRef<'context>,
  int_type: TypeRef<'context>,
  long_type: TypeRef<'context>,
  long_long_type: TypeRef<'context>,
  uchar_type: TypeRef<'context>,
  ushort_type: TypeRef<'context>,
  uint_type: TypeRef<'context>,
  ulong_type: TypeRef<'context>,
  ulong_long_type: TypeRef<'context>,
  ptrdiff_type: TypeRef<'context>,
  uintptr_type: TypeRef<'context>,
  float_type: TypeRef<'context>,
  double_type: TypeRef<'context>,
  voidptr_type: TypeRef<'context>,

  unnamed_str: StrRef<'context>,
}
impl<'context> Context<'context> {
  pub fn new(arena: &'context Bump) -> Self {
    let void_type = arena.alloc(Primitive::Void.into());
    Self {
      arena,
      type_interner: Default::default(),
      string_interner: Default::default(),
      int_type: arena.alloc(Primitive::Int.into()),
      float_type: arena.alloc(Primitive::Float.into()),
      short_type: arena.alloc(Primitive::Short.into()),
      ptrdiff_type: arena.alloc(Primitive::LongLong.into()),
      uintptr_type: arena.alloc(Primitive::ULongLong.into()),
      void_type,
      char_type: arena.alloc(Primitive::Char.into()),
      uchar_type: arena.alloc(Primitive::UChar.into()),
      ushort_type: arena.alloc(Primitive::UShort.into()),
      uint_type: arena.alloc(Primitive::UInt.into()),
      ulong_long_type: arena.alloc(Primitive::ULongLong.into()),
      long_type: arena.alloc(Primitive::Long.into()),
      ulong_type: arena.alloc(Primitive::ULong.into()),
      nullptr_type: arena.alloc(Primitive::Nullptr.into()),
      double_type: arena.alloc(Primitive::Double.into()),
      bool_type: arena.alloc(Primitive::Bool.into()),
      long_long_type: arena.alloc(Primitive::LongLong.into()),
      voidptr_type: arena
        .alloc(Pointer::new(QualifiedType::new_unqualified(void_type)).into()),
      unnamed_str: arena.alloc_str("<unnamed>"),
    }
  }

  pub fn arena(&self) -> &'context Bump {
    self.arena
  }
}

pub type ArenaVec<'a, T> = ::bumpalo::collections::Vec<'a, T>;

impl<'context> Context<'context> {
  fn do_intern(&self, value: Type<'context>) -> TypeRef<'context> {
    if let Some(&interned) = self.type_interner.borrow().get(&value) {
      interned
    } else {
      let interned = self.arena.alloc(value);
      self.type_interner.borrow_mut().insert(interned);
      interned
    }
  }

  pub fn intern_str(&self, value: &str) -> StrRef<'context> {
    if let Some(&interned) = self.string_interner.borrow().get(value) {
      interned
    } else {
      let interned = self.arena.alloc_str(value);
      self.string_interner.borrow_mut().insert(interned);
      // ... weird syntax to make &mut str into &str
      &*interned
    }
  }

  pub fn intern<T: Into<Type<'context>>>(&self, value: T) -> TypeRef<'context> {
    self.do_intern(value.into())
  }

  pub fn alloc_vec<T>(&self, capacity: usize) -> ArenaVec<'context, T> {
    ArenaVec::with_capacity_in(capacity, self.arena)
  }

  // Helper to allocate slices
  pub fn alloc_slice<T: Copy>(&self, values: &[T]) -> &'context [T] {
    self.arena.alloc_slice_copy(values)
  }

  pub fn make_function_proto(
    &self,
    return_type: QualifiedType<'context>,
    params: &[QualifiedType<'context>],
    is_variadic: bool,
  ) -> TypeRef<'context> {
    let params = self.alloc_slice(params);
    self.intern(FunctionProto::new(return_type, params, is_variadic))
  }

  pub fn make_pointer(
    &self,
    pointee: QualifiedType<'context>,
  ) -> TypeRef<'context> {
    self.intern(Pointer::new(pointee))
  }

  pub fn make_array(
    &self,
    element_type: QualifiedType<'context>,
    size: ArraySize,
  ) -> TypeRef<'context> {
    self.intern(Array::new(element_type, size))
  }

  pub fn int_type(&self) -> TypeRef<'context> {
    self.int_type
  }

  pub fn float_type(&self) -> TypeRef<'context> {
    self.float_type
  }

  pub fn ptrdiff_type(&self) -> TypeRef<'context> {
    self.ptrdiff_type
  }

  pub fn uintptr_type(&self) -> TypeRef<'context> {
    self.uintptr_type
  }

  pub fn void_type(&self) -> TypeRef<'context> {
    self.void_type
  }

  pub fn char_type(&self) -> TypeRef<'context> {
    self.char_type
  }

  pub fn nullptr_type(&self) -> TypeRef<'context> {
    self.nullptr_type
  }

  pub fn double_type(&self) -> TypeRef<'context> {
    self.double_type
  }

  pub fn bool_type(&self) -> TypeRef<'context> {
    self.bool_type
  }

  pub fn voidptr_type(&self) -> TypeRef<'context> {
    self.voidptr_type
  }

  pub fn long_long_type(&self) -> TypeRef<'context> {
    self.long_long_type
  }

  pub fn short_type(&self) -> TypeRef<'context> {
    self.short_type
  }

  pub fn uchar_type(&self) -> TypeRef<'context> {
    self.uchar_type
  }

  pub fn ushort_type(&self) -> TypeRef<'context> {
    self.ushort_type
  }

  pub fn uint_type(&self) -> TypeRef<'context> {
    self.uint_type
  }

  pub fn long_type(&self) -> TypeRef<'context> {
    self.long_type
  }

  pub fn ulong_type(&self) -> TypeRef<'context> {
    self.ulong_type
  }

  pub fn ulong_long_type(&self) -> TypeRef<'context> {
    self.ulong_long_type
  }
}
use crate::diagnosis::{DiagMeta, Severity};
impl<'context> Context<'context> {
  pub fn main_proto_validate(
    &self,
    proto: &FunctionProto<'context>,
    function_specifier: FunctionSpecifier,
  ) -> Result<(), DiagMeta<'context>> {
    use crate::diagnosis::DiagData::MainFunctionProtoMismatch;

    if proto.is_variadic {
      Err(
        MainFunctionProtoMismatch("main function cannot be variadic")
          .into_with(Severity::Error),
      )
    } else if function_specifier.contains(FunctionSpecifier::Inline) {
      Err(
        MainFunctionProtoMismatch("main function cannot be inline")
          .into_with(Severity::Error),
      )
    } else if !proto.compatible_with(
      self
        .intern(FunctionProto::new(
          self.int_type.into(),
          self.alloc_slice(&[]),
          false,
        ))
        .as_functionproto_unchecked(),
    ) && !proto.compatible_with(
      self
        .intern(FunctionProto::new(
          self.int_type.into(),
          self.alloc_slice(&[
            self.int_type.into(),
            self
              .intern(Pointer::new(
                self.intern(Pointer::new(self.char_type.into())).into(),
              ))
              .into(),
          ]),
          false,
        ))
        .as_functionproto_unchecked(),
    ) {
      Err(
        MainFunctionProtoMismatch(
          "main function must have either no parameters or two parameters \
           (int argc, char** argv)",
        )
        .into_with(Severity::Error),
      )
    } else {
      Ok(())
    }
  }

  pub fn unnamed_str(&self) -> StrRef<'context> {
    self.unnamed_str
  }
}
