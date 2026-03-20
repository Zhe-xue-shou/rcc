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

    self.apply_mut(self.current_block, |value| {
      value
        .data
        .as_basicblock_mut_unchecked()
        .instructions
        .push(value_id);
      value_id
    })
  }
}

impl<'c> Emitable<'c, inst::ICmp> for Emitter<'c> {
  fn emit(
    &mut self,
    icmp: inst::ICmp,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    debug_assert!(
      RefEq::ref_eq(*qualified_type, *self.i1())
        || RefEq::ref_eq(*qualified_type, self.ast().converted_bool()),
      "ICmp inst must have boolean as return type. Vectors are unimplemented."
    );

    let cmp = self.emit_common_instruction(inst::Cmp::from(icmp), self.i1());
    if !RefEq::ref_eq(*qualified_type, self.ast().converted_bool()) {
      cmp
    } else {
      self.emit(
        inst::Cast::Zext(inst::Zext::new(cmp)),
        self.ast().converted_bool().into(),
      )
    }
  }
}

impl<'c> Emitable<'c, inst::FCmp> for Emitter<'c> {
  fn emit(
    &mut self,
    fcmp: inst::FCmp,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    debug_assert!(
      RefEq::ref_eq(*qualified_type, *self.i1())
        || RefEq::ref_eq(*qualified_type, self.ast().converted_bool()),
      "FCmp inst must have boolean as return type."
    );
    let cmp = self.emit_common_instruction(inst::Cmp::from(fcmp), self.i1());
    if !RefEq::ref_eq(*qualified_type, self.ast().converted_bool()) {
      cmp
    } else {
      self.emit(
        inst::Cast::Zext(inst::Zext::new(cmp)),
        self.ast().converted_bool().into(),
      )
    }
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
    self.apply_mut(self.current_block, |value| {
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

    self.apply_mut(block_id, |value| {
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
