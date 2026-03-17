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

impl<'context> Emitable<'context, inst::Terminator>
  for Emitter<'_, 'context, '_>
{
  fn emit(
    &mut self,
    terminator: inst::Terminator,
    qualified_type: QualifiedType<'context>,
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
impl<'context> Emitable<'context, inst::Alloca> for Emitter<'_, 'context, '_> {
  fn emit(
    &mut self,
    alloca: inst::Alloca,
    qualified_type: QualifiedType<'context>,
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
impl<'context, InstType: Into<Instruction>> Emitable<'context, InstType>
  for Emitter<'_, 'context, '_>
{
  default fn emit(
    &mut self,
    value: InstType,
    qualified_type: QualifiedType<'context>,
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

impl<'context> Emitable<'context, module::Function<'context>>
  for Emitter<'_, 'context, '_>
{
  fn emit(
    &mut self,
    value: module::Function<'context>,
    qualified_type: QualifiedType<'context>,
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
impl<'context> Emitable<'context, module::Variable<'context>>
  for Emitter<'_, 'context, '_>
{
  fn emit(
    &mut self,
    value: module::Variable<'context>,
    qualified_type: QualifiedType<'context>,
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
impl<'context> Emitable<'context, Constant<'context>>
  for Emitter<'_, 'context, '_>
{
  fn emit(
    &mut self,
    value: Constant<'context>,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    let value_id = self
      .session
      .ir_context
      .intern_constant(value, qualified_type);
    // self.module.constants.push(value_id);
    value_id
  }
}
impl<'context> Emitable<'context, Argument> for Emitter<'_, 'context, '_> {
  fn emit(
    &mut self,
    value: Argument,
    qualified_type: QualifiedType<'context>,
  ) -> ValueID {
    self.session.ir_context.insert(Value::new(
      qualified_type,
      ty!(self, qualified_type),
      value.into(),
    ))
  }
}
