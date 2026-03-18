use ::bumpalo::Bump;
use ::rcc_utils::IntoWith;

use super::{
  Array, ArraySize, Compatibility, FunctionProto, FunctionSpecifier, Pointer,
  Primitive, QualifiedType, Type, TypeRef,
};
use crate::{common::StrRef, storage::StorageRef};
#[derive(Debug)]
pub struct Context<'c> {
  storage: StorageRef<'c>,

  nullptr_type: TypeRef<'c>,
  void_type: TypeRef<'c>,
  bool_type: TypeRef<'c>,
  char_type: TypeRef<'c>,
  short_type: TypeRef<'c>,
  int_type: TypeRef<'c>,
  long_type: TypeRef<'c>,
  long_long_type: TypeRef<'c>,
  uchar_type: TypeRef<'c>,
  ushort_type: TypeRef<'c>,
  uint_type: TypeRef<'c>,
  ulong_type: TypeRef<'c>,
  ulong_long_type: TypeRef<'c>,
  ptrdiff_type: TypeRef<'c>,
  uintptr_type: TypeRef<'c>,
  float_type: TypeRef<'c>,
  double_type: TypeRef<'c>,
  voidptr_type: TypeRef<'c>,

  converted_bool: TypeRef<'c>, // shall be `int` according to C standard.

  unnamed_str: StrRef<'c>,
}
impl<'c> Context<'c> {
  pub fn new(storage: StorageRef<'c>) -> Self {
    let void_type = storage.ast_arena.alloc(Primitive::Void.into());
    let int_type = storage.ast_arena.alloc(Primitive::Int.into());
    let this = Self {
      storage,
      int_type,
      float_type: storage.ast_arena.alloc(Primitive::Float.into()),
      short_type: storage.ast_arena.alloc(Primitive::Short.into()),
      ptrdiff_type: storage.ast_arena.alloc(Primitive::LongLong.into()),
      uintptr_type: storage.ast_arena.alloc(Primitive::ULongLong.into()),
      void_type,
      char_type: storage.ast_arena.alloc(Primitive::Char.into()),
      uchar_type: storage.ast_arena.alloc(Primitive::UChar.into()),
      ushort_type: storage.ast_arena.alloc(Primitive::UShort.into()),
      uint_type: storage.ast_arena.alloc(Primitive::UInt.into()),
      ulong_long_type: storage.ast_arena.alloc(Primitive::ULongLong.into()),
      long_type: storage.ast_arena.alloc(Primitive::Long.into()),
      ulong_type: storage.ast_arena.alloc(Primitive::ULong.into()),
      nullptr_type: storage.ast_arena.alloc(Primitive::Nullptr.into()),
      double_type: storage.ast_arena.alloc(Primitive::Double.into()),
      bool_type: storage.ast_arena.alloc(Primitive::Bool.into()),
      long_long_type: storage.ast_arena.alloc(Primitive::LongLong.into()),
      voidptr_type: storage
        .ast_arena
        .alloc(Pointer::new(QualifiedType::new_unqualified(void_type)).into()),

      converted_bool: int_type,

      unnamed_str: storage.ast_arena.alloc_str("<unnamed>"),
    };
    {
      let mut refmut = this.storage.ast_type_interner.borrow_mut();
      refmut.insert(this.int_type);
      refmut.insert(this.float_type);
      refmut.insert(this.short_type);
      refmut.insert(this.ptrdiff_type);
      refmut.insert(this.uint_type);
      refmut.insert(this.ulong_type);
      refmut.insert(this.ulong_long_type);
      refmut.insert(this.char_type);
      refmut.insert(this.uchar_type);
      refmut.insert(this.ushort_type);
      refmut.insert(this.long_type);
      refmut.insert(this.long_long_type);
      refmut.insert(this.void_type);
      refmut.insert(this.nullptr_type);
      refmut.insert(this.double_type);
      refmut.insert(this.bool_type);
      refmut.insert(this.voidptr_type);

      refmut.insert(this.converted_bool); // not needed actually, anyways
    }
    this
      .storage
      .str_interner
      .borrow_mut()
      .insert(this.unnamed_str);
    this
  }

  pub fn arena(&self) -> &'c Bump {
    self.storage.ast_arena
  }
}

pub type ArenaVec<'a, T> = ::bumpalo::collections::Vec<'a, T>;

impl<'c> Context<'c> {
  fn do_intern(&self, value: Type<'c>) -> TypeRef<'c> {
    if let Some(&interned) = self.storage.ast_type_interner.borrow().get(&value)
    {
      interned
    } else {
      let interned = self.storage.ast_arena.alloc(value);
      self.storage.ast_type_interner.borrow_mut().insert(interned);
      interned
    }
  }

  #[must_use]
  pub fn intern_str(&self, value: &str) -> StrRef<'c> {
    if let Some(&interned) = self.storage.str_interner.borrow().get(value) {
      interned
    } else {
      let interned = self.storage.ast_arena.alloc_str(value);
      self.storage.str_interner.borrow_mut().insert(interned);
      // ... weird syntax to make &mut str into &str
      &*interned
    }
  }

  #[must_use]
  pub fn intern<T: Into<Type<'c>>>(&self, value: T) -> TypeRef<'c> {
    self.do_intern(value.into())
  }

  #[must_use]
  pub fn alloc_vec<T>(&self, capacity: usize) -> ArenaVec<'c, T> {
    ArenaVec::with_capacity_in(capacity, self.storage.ast_arena)
  }

  /// Helper to allocate slices
  #[must_use]
  pub fn alloc_slice<T: Copy>(&self, values: &[T]) -> &'c [T] {
    self.storage.ast_arena.alloc_slice_copy(values)
  }

  #[must_use]
  pub fn make_function_proto(
    &self,
    return_type: QualifiedType<'c>,
    params: &[QualifiedType<'c>],
    is_variadic: bool,
  ) -> TypeRef<'c> {
    let params = self.alloc_slice(params);
    self.intern(FunctionProto::new(return_type, params, is_variadic))
  }

  #[must_use]
  pub fn make_pointer(
    &self,
    pointee: QualifiedType<'c>,
  ) -> TypeRef<'c> {
    self.intern(Pointer::new(pointee))
  }

  #[must_use]
  pub fn make_array(
    &self,
    element_type: QualifiedType<'c>,
    size: ArraySize,
  ) -> TypeRef<'c> {
    self.intern(Array::new(element_type, size))
  }

  #[must_use]
  pub fn int_type(&self) -> TypeRef<'c> {
    self.int_type
  }

  #[must_use]
  pub fn float_type(&self) -> TypeRef<'c> {
    self.float_type
  }

  #[must_use]
  pub fn ptrdiff_type(&self) -> TypeRef<'c> {
    self.ptrdiff_type
  }

  #[must_use]
  pub fn uintptr_type(&self) -> TypeRef<'c> {
    self.uintptr_type
  }

  #[must_use]
  pub fn void_type(&self) -> TypeRef<'c> {
    self.void_type
  }

  #[must_use]
  pub fn char_type(&self) -> TypeRef<'c> {
    self.char_type
  }

  #[must_use]
  pub fn nullptr_type(&self) -> TypeRef<'c> {
    self.nullptr_type
  }

  #[must_use]
  pub fn double_type(&self) -> TypeRef<'c> {
    self.double_type
  }

  /// Mostly this is not the correct choice for a converted bool: use [`Self::converted_bool`] instead.
  #[must_use]
  pub fn bool_type(&self) -> TypeRef<'c> {
    self.bool_type
  }

  #[must_use]
  pub fn voidptr_type(&self) -> TypeRef<'c> {
    self.voidptr_type
  }

  #[must_use]
  pub fn long_long_type(&self) -> TypeRef<'c> {
    self.long_long_type
  }

  #[must_use]
  pub fn short_type(&self) -> TypeRef<'c> {
    self.short_type
  }

  #[must_use]
  pub fn uchar_type(&self) -> TypeRef<'c> {
    self.uchar_type
  }

  #[must_use]
  pub fn ushort_type(&self) -> TypeRef<'c> {
    self.ushort_type
  }

  #[must_use]
  pub fn uint_type(&self) -> TypeRef<'c> {
    self.uint_type
  }

  #[must_use]
  pub fn long_type(&self) -> TypeRef<'c> {
    self.long_type
  }

  #[must_use]
  pub fn ulong_type(&self) -> TypeRef<'c> {
    self.ulong_type
  }

  #[must_use]
  pub fn ulong_long_type(&self) -> TypeRef<'c> {
    self.ulong_long_type
  }

  #[must_use]
  pub fn converted_bool(&self) -> TypeRef<'c> {
    self.converted_bool
  }
}
use crate::diagnosis::{DiagMeta, Severity};
impl<'c> Context<'c> {
  pub fn main_proto_validate(
    &self,
    proto: &FunctionProto<'c>,
    function_specifier: FunctionSpecifier,
  ) -> Result<(), DiagMeta<'c>> {
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

  pub fn unnamed_str(&self) -> StrRef<'c> {
    self.unnamed_str
  }
}
