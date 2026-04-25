use ::rcc_adt::{FloatFormat, Floating, Integral};
use ::rcc_ast::{Context as ASTContext, types as ast};
use ::rcc_shared::{Arena, Diagnosis, SourceManager};

use super::{
  ConstantData, Type, TypeRef, Value, ValueID,
  instruction::User,
  types::{Array, Function},
};

#[derive(Debug)]
pub struct Context<'c> {
  void_type: TypeRef<'c>,
  label_type: TypeRef<'c>,
  float32_type: TypeRef<'c>,
  float64_type: TypeRef<'c>,
  pointer_type: TypeRef<'c>,
  common_integer_types: [TypeRef<'c>; 6],

  ir_arena: RefCell<SlotMap<ValueID, Value<'c>>>,
  ir_def_use: RefCell<SecondaryMap<ValueID, Vec<ValueID>>>,

  ir_type_interner: Interner<TypeRef<'c>>,

  nullptr: ValueID,
  common_integer_one: [ValueID; 5],
  common_integer_zero: [ValueID; 5],
  common_floating_one: [ValueID; 2],
  common_floating_zero: [ValueID; 2],
  /// currently only for ir stage. use it in previous stage could cause unprecedented catastrophe. see the git stash.
  constant_interner: RefCell<BiHashMap<ValueID, ConstantData<'c>>>,

  ast_arena: &'c Arena,
}
#[derive(Debug)]
pub struct Session<'c, D: Diagnosis<'c>> {
  ir_context: &'c Context<'c>,
  ast_context: &'c ASTContext<'c>,
  diagnosis: &'c D,
  manager: &'c SourceManager,
}
pub type SessionRef<'c, D> = &'c Session<'c, D>;

impl<'c, D: Diagnosis<'c>> Session<'c, D> {
  pub fn new(
    diagnosis: &'c D,
    manager: &'c SourceManager,
    ast_context: &'c ASTContext<'c>,
    ir_context: &'c Context<'c>,
  ) -> Self {
    Self {
      diagnosis,
      manager,
      ast_context,
      ir_context,
    }
  }

  pub fn as_ast_session(&self) -> ::rcc_ast::Session<'c, D> {
    ::rcc_ast::Session::new(self.diagnosis, self.manager, self.ast_context)
  }
}
impl<'c, D: Diagnosis<'c>> Session<'c, D> {
  pub fn ast(&self) -> &'c ASTContext<'c> {
    self.ast_context
  }

  pub fn diag(&self) -> &'c D {
    self.diagnosis
  }

  pub fn src(&self) -> &'c SourceManager {
    self.manager
  }

  pub fn ir(&self) -> &'c Context<'c> {
    self.ir_context
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

  pub fn nullptr(&self) -> ValueID {
    self.nullptr
  }

  pub fn i1_true(&self) -> ValueID {
    self.common_integer_one[0]
  }

  pub fn i1_false(&self) -> ValueID {
    self.common_integer_zero[0]
  }

  pub fn floating_zero(&self, format: FloatFormat) -> ValueID {
    match format {
      FloatFormat::IEEE32 => self.common_floating_zero[0],
      FloatFormat::IEEE64 => self.common_floating_zero[1],
    }
  }

  pub fn floating_one(&self, format: FloatFormat) -> ValueID {
    match format {
      FloatFormat::IEEE32 => self.common_floating_one[0],
      FloatFormat::IEEE64 => self.common_floating_one[1],
    }
  }

  pub fn integer_zero(&self, width: u8) -> ValueID {
    let index = match width {
      1 => 0,
      8 => 1,
      16 => 2,
      32 => 3,
      64 => 4,
      // 128 => 5,
      _ => panic!("intern other integer constant on the fly"),
    };
    self.common_integer_zero[index]
  }

  pub fn integer_one(&self, width: u8) -> ValueID {
    let index = match width {
      1 => 0,
      8 => 1,
      16 => 2,
      32 => 3,
      64 => 4,
      // 128 => 5,
      _ => panic!("intern other integer constant on the fly"),
    };
    self.common_integer_one[index]
  }
}
impl<'c> Context<'c> {
  fn do_intern(&self, value: Type<'c>) -> TypeRef<'c> {
    if let Some(existing) = self.ir_type_interner.borrow().get(&value) {
      existing
    } else {
      let interned = self.ast_arena.alloc(value);
      self.ir_type_interner.borrow_mut().insert(interned);
      interned
    }
  }

  pub fn intern<T: Into<Type<'c>>>(&self, value: T) -> TypeRef<'c> {
    self.do_intern(value.into())
  }

  pub fn intern_constant<T: Into<ConstantData<'c>>>(
    &self,
    value: T,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let value = value.into();
    if let Some(existing) = self.constant_interner.borrow().get_by_right(&value)
    {
      *existing
    } else {
      let value_id = self.ir_arena.borrow_mut().insert(Value::new(
        ast_type,
        self.ir_type(ast_type),
        value.clone(),
        Default::default(),
      ));
      self.constant_interner.borrow_mut().insert(value_id, value);
      value_id
    }
  }

  pub fn get_by_constantdata_id(
    &self,
    id: &ValueID,
  ) -> Option<Ref<'_, ConstantData<'c>>> {
    Ref::filter_map(self.constant_interner.borrow(), |interner| {
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
      // 128 => self.common_integer_types[5],
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
use ::std::{
  cell::{Ref, RefMut},
  mem::MaybeUninit,
};
impl<'c> Context<'c> {
  pub fn insert(&self, value: Value<'c>) -> ValueID {
    let user = self.ir_arena.borrow_mut().insert(value);
    self.new_use_def_chain(user);
    self.apply(user, |value| {
      value
        .use_list()
        .iter()
        .filter(|&usee| !usee.is_null())
        .for_each(|usee| self.add_user_for(user, *usee));
    });
    user
  }

  pub fn add_user_for(&self, user: ValueID, usee: ValueID) {
    self
      .ir_def_use
      .borrow_mut()
      .entry(usee)
      .expect("not inserted, or key is null")
      .and_modify(|users| users.push(user));
  }

  pub fn new_use_def_chain(&self, user: ValueID) {
    assert!(!user.is_null());
    let _ = self
      .ir_def_use
      .borrow_mut()
      .insert(user, Default::default())
      .is_none_or(|_| panic!("{user:#?} has already inserted..."));
  }

  pub fn get(&self, id: ValueID) -> Ref<'_, Value<'c>> {
    Ref::map(self.ir_arena.borrow(), |slotmap| {
      slotmap.get(id).expect("invalid id used!")
    })
  }

  pub fn get_mut(&self, id: ValueID) -> RefMut<'_, Value<'c>> {
    RefMut::map(self.ir_arena.borrow_mut(), |slotmap| {
      slotmap.get_mut(id).expect("invalid id used!")
    })
  }

  pub fn get_use_list(&self, usee: ValueID) -> Ref<'_, Vec<ValueID>> {
    Ref::map(self.ir_def_use.borrow(), |def_use| {
      def_use
        .get(usee)
        .unwrap_or_else(|| panic!("usee {usee:#?} not found in def-use chain"))
    })
  }

  pub fn visit<R, F: FnOnce(&Value<'c>) -> R>(
    &self,
    id: ValueID,
    action: F,
  ) -> R {
    action(&self.get(id))
  }

  pub fn inspect<R, F: FnOnce(&Value<'c>, &Value<'c>) -> R>(
    &self,
    left: ValueID,
    right: ValueID,
    action: F,
  ) -> R {
    action(&self.get(left), &self.get(right))
  }

  pub fn apply<R, F: FnOnce(&mut Value<'c>) -> R>(
    &self,
    id: ValueID,
    action: F,
  ) -> R {
    action(&mut self.get_mut(id))
  }
}

impl<'c> Context<'c> {
  pub fn ir_type(&self, ast_type: ast::TypeRef<'c>) -> TypeRef<'c> {
    use ::rcc_ast::types::{Primitive, TypeInfo};
    use Primitive::*;
    match ast_type {
      ast::Type::Primitive(primitive) => match primitive {
        Float => self.float32_type,
        Double => self.float64_type,
        Void => self.void_type,
        Nullptr => self.pointer_type,
        integer @ (__IRBit | Bool | Char | SChar | Short | Int | Long
        | LongLong | UChar | UShort | UInt | ULong | ULongLong) =>
          self.make_integer(integer.size_bits() as u8),
        placeholder @ (LongDouble | ComplexFloat | ComplexDouble
        | ComplexLongDouble) => todo!("{placeholder:#?} not implemented"),
      },
      ast::Type::Pointer(_) => self.pointer_type,
      ast::Type::Array(array) => self.make_array(
        self.ir_type(&array.element_type),
        match array.size {
          ast::ArraySize::Constant(c) => c,
          ast::ArraySize::Incomplete | ast::ArraySize::Variable(_) => 0,
        },
      ),
      ast::Type::FunctionProto(function_proto) => self.make_function(
        self.ir_type(&function_proto.return_type),
        self.ast_arena.alloc_slice_fill_iter(
          function_proto
            .parameter_types
            .iter()
            .map(|t| self.ir_type(t)),
        ),
        function_proto.is_variadic,
      ),
      ast::Type::Enum(_) => todo!(),
      ast::Type::Record(_) => todo!(),
      ast::Type::Union(_) => todo!(),
    }
  }
}
use ::bimap::BiHashMap;
use ::slotmap::{SecondaryMap, SlotMap};
use ::std::{cell::RefCell, collections::HashSet};
type Interner<T> = RefCell<HashSet<T>>;

impl<'c> Context<'c> {
  #[allow(clippy::uninit_assumed_init)]
  #[allow(invalid_value)]
  pub fn new(ast_arena: &'c Arena, ast_context: &'c ASTContext) -> Self {
    let mut this = Self {
      void_type: ast_arena.alloc(Type::Void()),
      label_type: ast_arena.alloc(Type::Label()),
      float32_type: ast_arena.alloc(Type::Floating(FloatFormat::IEEE32)),
      float64_type: ast_arena.alloc(Type::Floating(FloatFormat::IEEE64)),
      pointer_type: ast_arena.alloc(Type::Pointer()),
      common_integer_types: [
        ast_arena.alloc(1.into()),
        ast_arena.alloc(8.into()),
        ast_arena.alloc(16.into()),
        ast_arena.alloc(32.into()),
        ast_arena.alloc(64.into()),
        ast_arena.alloc(128.into()),
      ],
      ast_arena,
      constant_interner: Default::default(),
      ir_arena: Default::default(),
      ir_def_use: Default::default(),
      ir_type_interner: Default::default(),
      nullptr: unsafe { MaybeUninit::uninit().assume_init() },
      common_integer_one: unsafe { MaybeUninit::uninit().assume_init() },
      common_integer_zero: unsafe { MaybeUninit::uninit().assume_init() },
      common_floating_one: unsafe { MaybeUninit::uninit().assume_init() },
      common_floating_zero: unsafe { MaybeUninit::uninit().assume_init() },
    };
    {
      let mut refmut = this.ir_type_interner.borrow_mut();
      refmut.insert(this.void_type);
      refmut.insert(this.label_type);
      refmut.insert(this.float32_type);
      refmut.insert(this.float64_type);
      refmut.insert(this.pointer_type);
      this.common_integer_types.iter().for_each(|&t| {
        refmut.insert(t);
      });
    }
    {
      let mut refmut = this.constant_interner.borrow_mut();
      let mut ir_arena_ref = this.ir_arena.borrow_mut();

      this.nullptr = ir_arena_ref.insert(Value::new(
        ast_context.nullptr_type(),
        this.pointer_type,
        ConstantData::Nullptr(),
        Default::default(),
      ));
      refmut.insert(this.nullptr, ConstantData::Nullptr());

      this.common_floating_zero[0] = ir_arena_ref.insert(Value::new(
        ast_context.float32_type(),
        this.float32_type(),
        ConstantData::Floating(Floating::zero(FloatFormat::IEEE32)),
        Default::default(),
      ));
      refmut.insert(
        this.common_floating_zero[0],
        ConstantData::Floating(Floating::zero(FloatFormat::IEEE32)),
      );

      this.common_floating_zero[1] = ir_arena_ref.insert(Value::new(
        ast_context.float64_type(),
        this.float64_type(),
        ConstantData::Floating(Floating::zero(FloatFormat::IEEE64)),
        Default::default(),
      ));
      refmut.insert(
        this.common_floating_zero[1],
        ConstantData::Floating(Floating::zero(FloatFormat::IEEE64)),
      );
      this.common_floating_one[0] = ir_arena_ref.insert(Value::new(
        ast_context.float32_type(),
        this.float32_type,
        ConstantData::Floating(Floating::one(FloatFormat::IEEE32)),
        Default::default(),
      ));
      refmut.insert(
        this.common_floating_one[0],
        ConstantData::Floating(Floating::one(FloatFormat::IEEE32)),
      );
      this.common_floating_one[1] = ir_arena_ref.insert(Value::new(
        ast_context.float64_type(),
        this.float64_type,
        ConstantData::Floating(Floating::one(FloatFormat::IEEE64)),
        Default::default(),
      ));
      refmut.insert(
        this.common_floating_one[1],
        ConstantData::Floating(Floating::one(FloatFormat::IEEE64)),
      );

      let ast_types = [
        ast_context.i1_bool_type(),
        ast_context.uchar_type(),
        ast_context.ushort_type(),
        ast_context.uint_type(),
        ast_context.ulong_long_type(),
      ];
      let widths = [1, 8, 16, 32, 64];
      ast_types.iter().zip(widths).enumerate().for_each(
        |(index, (ast_type, width))| {
          this.common_integer_one[index] = ir_arena_ref.insert(Value::new(
            ast_type,
            this.common_integer_types[index],
            ConstantData::Integral(Integral::from_unsigned(1, width)),
            Default::default(),
          ));
          refmut.insert(
            this.common_integer_one[index],
            ConstantData::Integral(Integral::from_unsigned(1, width)),
          );

          this.common_integer_zero[index] = ir_arena_ref.insert(Value::new(
            ast_type,
            this.common_integer_types[index],
            ConstantData::Integral(Integral::from_unsigned(0, width)),
            Default::default(),
          ));
          refmut.insert(
            this.common_integer_zero[index],
            ConstantData::Integral(Integral::from_unsigned(0, width)),
          );
        },
      );
    }
    this
  }
}
