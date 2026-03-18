use ::slotmap::Key;

use super::{
  Argument, Emitter, Value, ValueID, instruction as inst,
  instruction::Instruction, module,
};
use crate::types::{Constant, QualifiedType};

/// Overload helper. I love overloading.
pub trait Emitable<'a, ValueType> {
  #[must_use = "Usually the return value_id shall not be ignored; one \
                exception is for store instruction, which returns void. use \
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
    if let Some(block) = &mut self.current_block {
      assert!(block.terminator.is_null(), "block already has a terminator");
      let value_id = self.session.ir_context.insert(Value::new(
        qualified_type,
        ty!(self, qualified_type),
        Instruction::from(terminator).into(),
      ));
      block.terminator = value_id;
      value_id
    } else {
      panic!("no block to emit terminator into")
    }
  }
}
impl<'c> Emitable<'c, inst::Alloca> for Emitter<'c> {
  fn emit(
    &mut self,
    alloca: inst::Alloca,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    if let Some(block) = &mut self.current_block {
      let value_id = self.session.ir_context.insert(Value::new(
        qualified_type,
        self.session.ir_context.pointer_type(),
        Instruction::from(inst::Memory::from(alloca)).into(),
      ));

      block.instructions.push(value_id);
      value_id
    } else {
      panic!("no block to emit terminator into")
    }
  }
}
impl<'c, InstType: Into<Instruction>> Emitable<'c, InstType>
  for Emitter<'c>
{
  default fn emit(
    &mut self,
    value: InstType,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    if let Some(block) = &mut self.current_block {
      let value_id = self.session.ir_context.insert(Value::new(
        qualified_type,
        ty!(self, qualified_type),
        value.into().into(),
      ));
      block.instructions.push(value_id);
      value_id
    } else {
      panic!("no block to emit into")
    }
  }
}

impl<'c> Emitable<'c, module::Function<'c>>
  for Emitter<'c>
{
  fn emit(
    &mut self,
    value: module::Function<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let value_id = self.session.ir_context.insert(Value::new(
      qualified_type,
      ty!(self, qualified_type),
      value.into(),
    ));
    self.module.globals.push(value_id);
    value_id
  }
}
impl<'c> Emitable<'c, module::Variable<'c>>
  for Emitter<'c>
{
  fn emit(
    &mut self,
    value: module::Variable<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let value_id = self.session.ir_context.insert(Value::new(
      qualified_type,
      ty!(self, qualified_type),
      value.into(),
    ));
    self.module.globals.push(value_id);
    value_id
  }
}
impl<'c> Emitable<'c, Constant<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: Constant<'c>,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    let value_id = self
      .session
      .ir_context
      .intern_constant(value, qualified_type);
    // self.module.constants.push(value_id);
    value_id
  }
}
impl<'c> Emitable<'c, Argument> for Emitter<'c> {
  fn emit(
    &mut self,
    value: Argument,
    qualified_type: QualifiedType<'c>,
  ) -> ValueID {
    self.session.ir_context.insert(Value::new(
      qualified_type,
      ty!(self, qualified_type),
      value.into(),
    ))
  }
}
