use ::slotmap::Key;

use super::{
  Argument, Emitter, Value, ValueData, ValueID,
  instruction::{self as inst, Instruction},
  module,
};
use crate::{
  common::RefEq,
  types::{Constant, QualifiedType},
};

/// Overload helper. I love overloading.
pub trait Emitable<'a, ValueType> {
  #[must_use = "Usually the return value_id shall not be ignored; one such \
                exception is for `store` instruction, which returns void. use \
                `_` to explicitly` ignore the return value_id if you don't \
                need it."]
  fn emit(
    &mut self,
    value: ValueType,
    qualified_type: QualifiedType<'a>,
  ) -> ValueID;
}

impl<'c> Emitable<'c, inst::Terminator> for Emitter<'c> {
  fn emit(
    &mut self,
    terminator: inst::Terminator,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    self.emit_terminator(terminator, qualified_type, self.current_block)
  }
}
impl<'c> Emitable<'c, inst::Alloca> for Emitter<'c> {
  fn emit(
    &mut self,
    alloca: inst::Alloca,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    if self.current_block.is_null() {
      panic!("no block to emit into")
    }
    let value_id = self.ir().insert(Value::new(
      qualified_type,
      self.ir().pointer_type(),
      Instruction::from(inst::Memory::from(alloca)).into(),
    ));
    let mut refmut = lookup_mut!(self, self.current_block);
    let mutref = refmut.data.as_basicblock_mut_unchecked();
    mutref.instructions.push(value_id);
    value_id
  }
}

impl<'c> Emitable<'c, inst::ICmp> for Emitter<'c> {
  fn emit(
    &mut self,
    icmp: inst::ICmp,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    debug_assert!(
      RefEq::ref_eq(*qualified_type, self.ast().i1_bool_type()),
      "ICmp inst must have i1 as return type. Vectors are unimplemented."
    );
    self.emit_common_instruction(icmp, qualified_type)
  }
}

impl<'c> Emitable<'c, inst::FCmp> for Emitter<'c> {
  fn emit(
    &mut self,
    fcmp: inst::FCmp,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    debug_assert!(
      RefEq::ref_eq(*qualified_type, self.ast().i1_bool_type()),
      "FCmp inst must have i1 as return type."
    );
    self.emit_common_instruction(fcmp, qualified_type)
  }
}

impl<'c> Emitter<'c> {
  fn emit_common_instruction<T: Into<Instruction>>(
    &self,
    value: T,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    if self.current_block.is_null() {
      panic!("no block to emit into")
    }
    let value_id = self.ir().insert(Value::new(
      qualified_type,
      ty!(self, qualified_type),
      value.into().into(),
    ));
    let mut refmut = lookup_mut!(self, self.current_block);
    let mutref = refmut.data.as_basicblock_mut_unchecked();
    mutref.instructions.push(value_id);
    value_id
  }

  fn emit_globals<T: Into<ValueData<'c>>>(
    &mut self,
    value: T,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let value_id = self.ir().insert(Value::new(
      qualified_type,
      ty!(self, qualified_type),
      value.into(),
    ));
    self.module.globals.push(value_id);
    value_id
  }

  pub(super) fn emit_terminator<T: Into<inst::Terminator>>(
    &self,
    terminator: T,
    qualified_type: QualifiedType<'c>,
    block_id: ValueID,
  ) -> ValueID {
    if block_id.is_null() {
      panic!("no block to emit terminator into")
    }

    let value_id = self.ir().insert(Value::new(
      qualified_type,
      ty!(self, qualified_type),
      Instruction::from(terminator.into()).into(),
    ));

    let mut refmut = lookup_mut!(self, block_id);
    let mutref = refmut.data.as_basicblock_mut_unchecked();
    assert!(
      mutref.terminator.is_null(),
      "block already has a terminator"
    );
    mutref.terminator = value_id;
    value_id
  }
}

impl<'c, InstType: Into<Instruction>> Emitable<'c, InstType> for Emitter<'c> {
  default fn emit(
    &mut self,
    value: InstType,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    self.emit_common_instruction(value, qualified_type)
  }
}

impl<'c> Emitable<'c, module::Function<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: module::Function<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    self.emit_globals(value, qualified_type)
  }
}
impl<'c> Emitable<'c, module::Variable<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: module::Variable<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    self.emit_globals(value, qualified_type)
  }
}
impl<'c> Emitable<'c, Constant<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: Constant<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    self.ir().intern_constant(value, qualified_type)
  }
}
impl<'c> Emitable<'c, Argument> for Emitter<'c> {
  fn emit(
    &mut self,
    value: Argument,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    self.ir().insert(Value::new(
      qualified_type,
      ty!(self, qualified_type),
      value.into(),
    ))
  }
}
