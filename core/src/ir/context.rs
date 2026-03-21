use super::{
  Type, TypeRef, Value, ValueID,
  types::{Array, Function},
  value::{WithAction, WithActionMut},
};
use crate::{common::FloatFormat, types::Constant};
#[derive(Debug)]
pub struct Context<'c> {
  void_type: TypeRef<'c>,
  label_type: TypeRef<'c>,
  float32_type: TypeRef<'c>,
  float64_type: TypeRef<'c>,
  pointer_type: TypeRef<'c>,
  common_integer_types: [TypeRef<'c>; 6],

  storage: StorageRef<'c>,
}

impl<'c> Context<'c> {
  pub fn new(storage: StorageRef<'c>) -> Self {
    let this = Self {
      void_type: storage.ast_arena.alloc(Type::Void()),
      label_type: storage.ast_arena.alloc(Type::Label()),
      float32_type: storage
        .ast_arena
        .alloc(Type::Floating(FloatFormat::IEEE32)),
      float64_type: storage
        .ast_arena
        .alloc(Type::Floating(FloatFormat::IEEE64)),
      pointer_type: storage.ast_arena.alloc(Type::Pointer()),
      common_integer_types: [
        storage.ast_arena.alloc(1.into()),
        storage.ast_arena.alloc(8.into()),
        storage.ast_arena.alloc(16.into()),
        storage.ast_arena.alloc(32.into()),
        storage.ast_arena.alloc(64.into()),
        storage.ast_arena.alloc(128.into()),
      ],

      storage,
    };
    {
      let mut refmut = this.storage.ir_type_interner.borrow_mut();
      refmut.insert(this.void_type);
      refmut.insert(this.label_type);
      refmut.insert(this.float32_type);
      refmut.insert(this.float64_type);
      refmut.insert(this.pointer_type);
      this.common_integer_types.iter().for_each(|&t| {
        refmut.insert(t);
      });
    }
    this
  }
}
impl<'c> Context<'c> {
  pub fn void_type(&self) -> TypeRef<'c> {
    self.void_type
  }

  pub fn label_type(&self) -> TypeRef<'c> {
    self.label_type
  }

  pub fn float32_type(&self) -> TypeRef<'c> {
    self.float32_type
  }

  pub fn float64_type(&self) -> TypeRef<'c> {
    self.float64_type
  }

  pub fn pointer_type(&self) -> TypeRef<'c> {
    self.pointer_type
  }

  fn do_intern(&self, value: Type<'c>) -> TypeRef<'c> {
    if let Some(existing) = self.storage.ir_type_interner.borrow().get(&value) {
      existing
    } else {
      let interned = self.storage.ast_arena.alloc(value);
      self.storage.ir_type_interner.borrow_mut().insert(interned);
      interned
    }
  }

  pub fn intern<T: Into<Type<'c>>>(&self, value: T) -> TypeRef<'c> {
    self.do_intern(value.into())
  }

  pub fn intern_constant<T: Into<Constant<'c>>>(
    &self,
    value: T,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let value = value.into();
    if let Some(existing) =
      self.storage.constant_interner.borrow().get_by_right(&value)
    {
      *existing
    } else {
      let value_id = self.storage.ir_arena.borrow_mut().insert(Value::new(
        qualified_type,
        self.ir_type(&qualified_type),
        value.clone().into(),
      ));
      self
        .storage
        .constant_interner
        .borrow_mut()
        .insert(value_id, value);
      value_id
    }
  }

  pub fn get_by_constant_id(
    &self,
    id: &ValueID,
  ) -> Option<Ref<'_, Constant<'c>>> {
    Ref::filter_map(self.storage.constant_interner.borrow(), |interner| {
      interner.get_by_left(id)
    })
    .ok()
  }

  pub fn make_integer(&self, bits: u8) -> TypeRef<'c> {
    match bits {
      1 => self.common_integer_types[0],
      8 => self.common_integer_types[1],
      16 => self.common_integer_types[2],
      32 => self.common_integer_types[3],
      64 => self.common_integer_types[4],
      128 => self.common_integer_types[5],
      _ => self.intern(Type::Integer(bits)),
    }
  }

  pub fn make_array(
    &self,
    element_type: TypeRef<'c>,
    length: usize,
  ) -> TypeRef<'c> {
    self.intern(Array::new(element_type, length))
  }

  pub fn make_function(
    &self,
    result_type: TypeRef<'c>,
    params: &'c [TypeRef<'c>],
    is_variadic: bool,
  ) -> TypeRef<'c> {
    self.intern(Function::new(result_type, params, is_variadic))
  }
}
use ::std::cell::{Ref, RefMut};
impl<'c> Context<'c> {
  pub fn insert(&self, value: Value<'c>) -> ValueID {
    self.storage.ir_arena.borrow_mut().insert(value)
  }

  pub fn get(&self, id: ValueID) -> Ref<'_, Value<'c>> {
    Ref::map(self.storage.ir_arena.borrow(), |slotmap| &slotmap[id])
  }

  pub fn get_mut(&self, id: ValueID) -> RefMut<'_, Value<'c>> {
    RefMut::map(self.storage.ir_arena.borrow_mut(), |slotmap| {
      &mut slotmap[id]
    })
  }

  pub fn apply<R, F: FnOnce(&Value<'c>) -> R>(
    &self,
    id: ValueID,
    action: F,
  ) -> R {
    self.get(id).with_action(action)
  }

  pub fn apply_mut<R, F: FnOnce(&mut Value<'c>) -> R>(
    &self,
    id: ValueID,
    action: F,
  ) -> R {
    self.get_mut(id).with_action_mut(action)
  }
}
use crate::{
  storage::StorageRef,
  types::{self, QualifiedType},
};
impl<'c> Context<'c> {
  pub fn ir_type(
    &self,
    qualified_type: &types::QualifiedType<'c>,
  ) -> TypeRef<'c> {
    use Primitive::*;
    use types::{Primitive, TypeInfo};
    match qualified_type.unqualified_type {
      types::Type::Primitive(primitive) => match primitive {
        Float => self.float32_type,
        Double => self.float64_type,
        Void => self.void_type,
        Nullptr => self.pointer_type,
        integer @ (Bool | Char | SChar | Short | Int | Long | LongLong
        | UChar | UShort | UInt | ULong | ULongLong) =>
          self.make_integer(integer.size_bits() as u8),
        placeholder @ (LongDouble | ComplexFloat | ComplexDouble
        | ComplexLongDouble) => todo!("{placeholder:#?} not implemented"),
      },
      types::Type::Pointer(_) => self.pointer_type,
      types::Type::Array(array) => self.make_array(
        self.ir_type(&array.element_type),
        match array.size {
          types::ArraySize::Constant(c) => c,
          types::ArraySize::Incomplete | types::ArraySize::Variable(_) => 0,
        },
      ),
      types::Type::FunctionProto(function_proto) => self.make_function(
        self.ir_type(&function_proto.return_type),
        self.storage.ast_arena.alloc_slice_fill_iter(
          function_proto
            .parameter_types
            .iter()
            .map(|t| self.ir_type(t)),
        ),
        function_proto.is_variadic,
      ),
      types::Type::Enum(_) => todo!(),
      types::Type::Record(_) => todo!(),
      types::Type::Union(_) => todo!(),
    }
  }
}
