use ::rcc_ast::types as ast;
use ::rcc_shared::Constant;
use ::rcc_utils::RefEq;

use super::{
  Argument, Emitter, Value, ValueData, ValueID,
  instruction::{self as inst, Instruction},
  module,
};

/// Overload helper. I love overloading.
pub trait Emitable<'a, ValueType> {
  #[must_use = "Usually the return value_id shall not be ignored; one such \
                exception is for `store` instruction, which returns void. use \
                `_` to explicitly` ignore the return value_id if you don't \
                need it."]
  fn emit(&mut self, value: ValueType, ast_type: ast::TypeRef<'a>) -> ValueID;
}

impl<'c> Emitable<'c, inst::Terminator> for Emitter<'c> {
  fn emit(
    &mut self,
    terminator: inst::Terminator,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit_terminator(terminator, ast_type, self.current_block)
  }
}
impl<'c> Emitable<'c, inst::Alloca> for Emitter<'c> {
  fn emit(
    &mut self,
    alloca: inst::Alloca,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    if self.current_block.is_null() {
      panic!("no block to emit into")
    }
    let value_id = self.ir().insert(Value::new(
      ast_type,
      self.ir().pointer_type(),
      Instruction::from(inst::Memory::from(alloca)),
      self.current_block,
    ));

    let entry_id = self
      .visit(self.current_function, |value| {
        value.data.as_function_unchecked().entry()
      })
      // shall only be called if current block is entry block
      .unwrap_or(self.current_block);
    self.apply(entry_id, |value| {
      value
        .data
        .as_basicblock_mut_unchecked()
        .instructions
        .push(value_id);
      value_id
    })
  }
}
impl<'c> Emitable<'c, inst::Unary> for Emitter<'c> {
  fn emit(
    &mut self,
    value: inst::Unary,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    assert!(self.visit(value.operand(), |val| {
      val.ir_type.is_pointer()
        || val.ir_type.is_integer()
        || val.ir_type.is_floating()
    }));
    self.emit_common_instruction(value, ast_type)
  }
}
impl<'c> Emitable<'c, inst::Binary> for Emitter<'c> {
  fn emit(
    &mut self,
    value: inst::Binary,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit_common_instruction(value, ast_type)
  }
}
impl<'c> Emitable<'c, inst::Memory> for Emitter<'c> {
  fn emit(
    &mut self,
    value: inst::Memory,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit_common_instruction(value, ast_type)
  }
}

impl<'c> Emitable<'c, inst::Cast> for Emitter<'c> {
  fn emit(&mut self, value: inst::Cast, ast_type: ast::TypeRef<'c>) -> ValueID {
    self.emit_common_instruction(value, ast_type)
  }
}
impl<'c> Emitable<'c, inst::Call> for Emitter<'c> {
  fn emit(&mut self, value: inst::Call, ast_type: ast::TypeRef<'c>) -> ValueID {
    self.emit_common_instruction(value, ast_type)
  }
}

impl<'c> Emitable<'c, inst::ICmp> for Emitter<'c> {
  fn emit(&mut self, icmp: inst::ICmp, ast_type: ast::TypeRef<'c>) -> ValueID {
    debug_assert!(
      RefEq::ref_eq(ast_type, self.ast().i1_bool_type())
        || RefEq::ref_eq(ast_type, self.ast().converted_bool()),
      "ICmp inst must have boolean as return type. Vectors are unimplemented."
    );

    let cmp = self.emit_common_instruction(inst::Cmp::from(icmp), {
      let this = &self;
      this.ast().i1_bool_type()
    });
    if !RefEq::ref_eq(ast_type, self.ast().converted_bool()) {
      cmp
    } else {
      self.emit(
        inst::Cast::Zext(inst::Zext::new(cmp)),
        self.ast().converted_bool(),
      )
    }
  }
}

impl<'c> Emitable<'c, inst::FCmp> for Emitter<'c> {
  fn emit(&mut self, fcmp: inst::FCmp, ast_type: ast::TypeRef<'c>) -> ValueID {
    debug_assert!(
      RefEq::ref_eq(ast_type, self.ast().i1_bool_type())
        || RefEq::ref_eq(ast_type, self.ast().converted_bool()),
      "FCmp inst must have boolean as return type."
    );
    let cmp = self.emit_common_instruction(inst::Cmp::from(fcmp), {
      let this = &self;
      this.ast().i1_bool_type()
    });
    if !RefEq::ref_eq(ast_type, self.ast().converted_bool()) {
      cmp
    } else {
      self.emit(
        inst::Cast::Zext(inst::Zext::new(cmp)),
        self.ast().converted_bool(),
      )
    }
  }
}

impl<'c> Emitter<'c> {
  fn emit_common_instruction<T: Into<Instruction>>(
    &self,
    value: T,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    if self.current_block.is_null() {
      panic!("no block to emit into")
    }
    let value_id = self.ir().insert(Value::new(
      ast_type,
      ty!(self, ast_type),
      value.into(),
      self.current_block,
    ));
    self.apply(self.current_block, |value| {
      value
        .data
        .as_basicblock_mut_unchecked()
        .instructions
        .push(value_id);
      value_id
    })
  }

  fn emit_globals<T: Into<ValueData<'c>>>(
    &mut self,
    value: T,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let value_id = self.ir().insert(Value::new(
      ast_type,
      ty!(self, ast_type),
      value.into(),
      Default::default(),
    ));
    self.module.globals.push(value_id);
    value_id
  }

  pub(super) fn emit_terminator<T: Into<inst::Terminator>>(
    &self,
    terminator: T,
    ast_type: ast::TypeRef<'c>,
    block_id: ValueID,
  ) -> ValueID {
    if block_id.is_null() {
      panic!("no block to emit terminator into")
    }

    let value_id = self.ir().insert(Value::new(
      ast_type,
      ty!(self, ast_type),
      Instruction::from(terminator.into()),
      self.current_block,
    ));

    self.apply(block_id, |value| {
      let mutref = value.data.as_basicblock_mut_unchecked();
      assert!(
        mutref.terminator.is_null(),
        "block already has a terminator"
      );
      mutref.terminator = value_id;
      value_id
    })
  }
}

impl<'c> Emitable<'c, module::Function<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: module::Function<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit_globals(value, ast_type)
  }
}
impl<'c> Emitable<'c, module::Variable<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: module::Variable<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit_globals(value, ast_type)
  }
}
impl<'c> Emitable<'c, Constant<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: Constant<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.ir().intern_constant(value, ast_type)
  }
}
impl<'c> Emitable<'c, Argument> for Emitter<'c> {
  fn emit(&mut self, value: Argument, ast_type: ast::TypeRef<'c>) -> ValueID {
    self.ir().insert(Value::new(
      ast_type,
      ty!(self, ast_type),
      value,
      self.current_function,
    ))
  }
}
