use ::rcc_utils::StrRef;
use ::std::{cell::RefCell, collections::HashSet};

use crate::types::{
  Array, ArraySize, Compatibility, FunctionProto, FunctionSpecifier, Pointer,
  Primitive, QualifiedType, Type, TypeRef,
};
type Interner<T> = RefCell<HashSet<T>>;
#[derive(Debug)]
pub struct Context<'c> {
  arena: &'c Arena,
  ast_type_interner: Interner<TypeRef<'c>>,
  // str_interner: Interner<StrRef<'c>>,
  nullptr_type: TypeRef<'c>,
  void_type: TypeRef<'c>,
  bool_type: TypeRef<'c>,
  char_type: TypeRef<'c>,
  schar_type: TypeRef<'c>,
  short_type: TypeRef<'c>,
  int_type: TypeRef<'c>,
  long_type: TypeRef<'c>,
  long_long_type: TypeRef<'c>,
  // i128_type: TypeRef<'c>,
  uchar_type: TypeRef<'c>,
  ushort_type: TypeRef<'c>,
  uint_type: TypeRef<'c>,
  ulong_type: TypeRef<'c>,
  ulong_long_type: TypeRef<'c>,
  // u128_type: TypeRef<'c>,
  ptrdiff_type: TypeRef<'c>,
  uintptr_type: TypeRef<'c>,
  float_type: TypeRef<'c>,
  double_type: TypeRef<'c>,
  voidptr_type: TypeRef<'c>,

  /// this field serve differenc purpose and represented as i1 NOT i8 in IR
  /// -- it's a loophole in my design --
  /// do NOT use this in AST level -- use `bool_type` during AST!
  ///
  /// it works because my [`TypeInfo::size_bits`] returns 1 instead of 8.
  fake_bool_type: TypeRef<'c>,

  converted_bool: TypeRef<'c>, //  `int` according to C standard.

  unnamed_str: StrRef<'c>,

  langopts: u8,
}

impl<'c> Context<'c> {
  fn do_intern(&self, value: Type<'c>) -> TypeRef<'c> {
    if let Some(&interned) = self.ast_type_interner.borrow().get(&value) {
      // println!("{} found", value);
      interned
    } else {
      // println!("{} not found", value);
      let interned = self.arena.alloc(value);
      self.ast_type_interner.borrow_mut().insert(interned);
      interned
    }
  }

  // #[must_use]
  // pub fn intern_str(&self, value: &str) -> StrRef<'c> {
  //   if let Some(&interned) = self.str_interner.borrow().get(value) {
  //     interned
  //   } else {
  //     let interned = self.arena.alloc_str(value);
  //     self.str_interner.borrow_mut().insert(interned);
  //     // ... weird syntax to make &mut str into &str
  //     &*interned
  //   }
  // }

  #[must_use]
  pub fn intern<T: Into<Type<'c>>>(&self, value: T) -> TypeRef<'c> {
    self.do_intern(value.into())
  }

  // #[must_use]
  // fn alloc_vec<T>(&self, capacity: usize) -> ArenaVec<'c, T> {
  //   ArenaVec::with_capacity_in(capacity, self.arena)
  // }

  /// Helper to allocate slices
  #[must_use]
  fn alloc_slice<T: Copy>(&self, values: &[T]) -> &'c [T] {
    self.arena.alloc_slice_copy(values)
  }

  #[must_use]
  pub fn make_function_proto(
    &self,
    return_type: QualifiedType<'c>,
    params: &'c [QualifiedType<'c>],
    is_variadic: bool,
  ) -> TypeRef<'c> {
    // FIXME: canonical typers, re-intern
    self.intern(FunctionProto::new(return_type, params, is_variadic))
  }

  #[must_use]
  pub fn make_pointer(&self, pointee: QualifiedType<'c>) -> TypeRef<'c> {
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
  pub fn float32_type(&self) -> TypeRef<'c> {
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
  pub fn schar_type(&self) -> TypeRef<'c> {
    self.schar_type
  }

  #[must_use]
  pub fn nullptr_type(&self) -> TypeRef<'c> {
    self.nullptr_type
  }

  #[must_use]
  pub fn float64_type(&self) -> TypeRef<'c> {
    self.double_type
  }

  /// Mostly this is not the correct choice for a converted bool: use [`Self::converted_bool`] instead.
  #[must_use]
  pub fn i1_bool_type(&self) -> TypeRef<'c> {
    self.fake_bool_type
  }

  #[must_use]
  pub fn i8_bool_type(&self) -> TypeRef<'c> {
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

  #[must_use]
  pub fn langopts(&self) -> u8 {
    self.langopts
  }
}
impl<'c> Context<'c> {
  pub fn new(arena: &'c Arena) -> Self {
    use Primitive::*;

    let void_type = arena.alloc(Void.into());
    let int_type = arena.alloc(Int.into());
    let this = Self {
      arena,
      ast_type_interner: Default::default(),
      // str_interner: Default::default(),
      int_type,
      float_type: arena.alloc(Float.into()),
      short_type: arena.alloc(Short.into()),
      ptrdiff_type: arena.alloc(LongLong.into()),
      uintptr_type: arena.alloc(ULongLong.into()),
      void_type,
      char_type: arena.alloc(Char.into()),
      schar_type: arena.alloc(SChar.into()),
      uchar_type: arena.alloc(UChar.into()),
      ushort_type: arena.alloc(UShort.into()),
      uint_type: arena.alloc(UInt.into()),
      ulong_long_type: arena.alloc(ULongLong.into()),
      long_type: arena.alloc(Long.into()),
      ulong_type: arena.alloc(ULong.into()),

      nullptr_type: arena.alloc(Nullptr.into()),
      double_type: arena.alloc(Double.into()),
      bool_type: arena.alloc(Bool.into()),
      long_long_type: arena.alloc(LongLong.into()),
      voidptr_type: arena
        .alloc(Pointer::new(QualifiedType::new_unqualified(void_type)).into()),

      converted_bool: int_type,
      fake_bool_type: arena.alloc(__IRBit.into()),

      unnamed_str: arena.alloc_str("<unnamed>"),
      langopts: 23,
    };
    {
      let mut refmut = this.ast_type_interner.borrow_mut();
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
    // this.str_interner.borrow_mut().insert(this.unnamed_str);
    this
  }

  pub fn arena(&self) -> &'c Arena {
    self.arena
  }
}
use ::rcc_shared::{Arena, DiagMeta, Severity};
impl<'c> Context<'c> {
  pub fn main_proto_validate(
    &self,
    proto: &FunctionProto<'c>,
    function_specifier: FunctionSpecifier,
  ) -> Result<(), DiagMeta<'c>> {
    use ::rcc_shared::DiagData::MainFunctionProtoMismatch;

    if proto.is_variadic {
      Err(
        MainFunctionProtoMismatch("main function cannot be variadic")
          + Severity::Error,
      )
    } else if function_specifier.contains(FunctionSpecifier::Inline) {
      Err(
        MainFunctionProtoMismatch("main function cannot be inline")
          + Severity::Error,
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
        ) + Severity::Error,
      )
    } else {
      Ok(())
    }
  }

  pub fn unnamed_str(&self) -> StrRef<'c> {
    self.unnamed_str
  }
}
