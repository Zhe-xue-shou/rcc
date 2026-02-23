use ::rcc_utils::SmallString;
use ::slotmap::new_key_type;

use super::value::{BlockID, ValueID};
use crate::{
  common::StrRef,
  types::{Constant, QualifiedType},
};

/// result = phi [val1, label1], [val2, label2]
///
/// left here as placeholder, do it later.
#[derive(Debug, Clone)]
pub struct Phi {
  pub incomings: Vec<(ValueID, BlockID)>, // (Value, From_Block_Label)
}

#[derive(Debug)]
pub struct Jump {
  pub label: BlockID,
}
#[derive(Debug)]
pub struct Branch {
  pub cond: ValueID,
  pub true_label: BlockID,
  pub false_label: BlockID,
}
#[derive(Debug)]
pub struct Return {
  pub result: Option<ValueID>,
}
#[derive(Debug)]
pub enum Terminator {
  /// Unconditional jump
  Jump(Jump),
  /// Conditional branch: if cond goto true_label else goto false_label
  Branch(Branch),
  /// Return from function
  Return(Return),
}

/// result = unary_op operand
#[derive(Debug)]
pub struct Unary {
  pub operator: UnaryOp,
  pub operand: ValueID,
}
#[derive(Debug)]
pub enum UnaryOp {
  Neg,
  Not,
  Compl,
}
/// result = binary_op lhs, rhs
#[derive(Debug)]
pub struct Binary {
  pub operator: BinaryOp,
  pub lhs: ValueID,
  pub rhs: ValueID,
}
// arithematic ops only consider integer for now
#[derive(Debug, Clone, Copy, ::strum_macros::Display)]
pub enum BinaryOp {
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  BitwiseAnd,
  BitwiseOr,
  BitwiseXor,
  LeftShift,
  RightShift,
}

#[derive(Debug)]
pub struct ICmp {
  pub predicate: ICmpPredicate,
  pub lhs: ValueID,
  pub rhs: ValueID,
}
#[derive(Debug, Clone, Copy, ::strum_macros::Display)]
pub enum ICmpPredicate {
  Eq,
  Ne,
  Slt,
  Sle,
  Sgt,
  Sge,
  Ult,
  Ule,
  Ugt,
  Uge,
}
/// Store value to address: *addr = value
#[derive(Debug)]
pub struct Store {
  pub addr: ValueID,
  pub value: ValueID,
}

/// Load value from address: result = *addr
#[derive(Debug)]
pub struct Load {
  pub addr: ValueID,
}
#[derive(Debug)]
pub enum Memory<'context> {
  Store(Store),
  Load(Load),
  Alloca(Alloca<'context>),
}
/// Stack allocation.
/// result = alloca typeof(type)
/// Used for local variables that must live in memory (e.g., if their address is taken).
#[derive(Debug)]
pub struct Alloca<'context> {
  pub qualified_type: QualifiedType<'context>,
}

#[derive(Debug)]
pub enum Cast<'a> {
  // add later.
  Placeholder(&'a u8),
}

/// Function call: result = call func(args)
#[derive(Debug)]
pub struct Call {
  pub callee: ValueID,
  pub args: Vec<ValueID>,
}

/// This mimics LLVM ir's catagory.
#[derive(Debug)]
pub enum Instruction<'context> {
  Phi(Phi),
  Terminator(Terminator),
  Unary(Unary),
  Binary(Binary),
  Memory(Memory<'context>),
  Cast(Cast<'context>),
  Call(Call),
  ICmp(ICmp),
  // etc...
}

::rcc_utils::interconvert!(Phi, Instruction<'context>);
::rcc_utils::interconvert!(Terminator, Instruction<'context>);
::rcc_utils::interconvert!(Unary, Instruction<'context>);
::rcc_utils::interconvert!(Binary, Instruction<'context>);
::rcc_utils::interconvert!(Memory, Instruction,'context);
::rcc_utils::interconvert!(Cast, Instruction,'context);
::rcc_utils::interconvert!(Call, Instruction<'context>);
::rcc_utils::interconvert!(ICmp, Instruction<'context>);

::rcc_utils::make_trio_for!(Call, Instruction<'context>);
