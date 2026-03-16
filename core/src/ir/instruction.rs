use super::value::ValueID;
use crate::types::QualifiedType;

/// result = phi [val1, label1], [val2, label2]
///
/// left here as placeholder, do it later.
#[derive(Debug, Clone)]
pub struct Phi {
  pub incomings: Vec<(ValueID, ValueID)>, // (Value, From_Block_Label)
}

/// Creater must ensure [`Jump::label`] must be am ID points to a [`super::BasicBlock`].
#[derive(Debug)]
pub struct Jump {
  pub label: ValueID,
}
/// Creater must ensure [`Branch::true_label`] and [`Branch::false_label`] must be am ID points to a [`super::BasicBlock`].
///
/// The owner of this instruction must ensure the type of [`Branch::cond`] is i1 (boolean).
#[derive(Debug)]
pub struct Branch {
  pub cond: ValueID,
  pub true_label: ValueID,
  pub false_label: ValueID,
}
/// Must match the return type of the function. For void function, [`Return::result`] should be [`None`].
#[derive(Debug)]
pub struct Return {
  pub result: Option<ValueID>,
}

impl Return {
  pub fn new(result: Option<ValueID>) -> Self {
    Self { result }
  }
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
///
/// - The type of `lhs` and `rhs` must be the same.
/// - `lhs` and `rhs` cannot be [`super::module::Function`], [`super::module::BasicBlock`] or [`super::module::Variable`].  
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
///
/// [`Store::addr`] must have pointer type
#[derive(Debug)]
pub struct Store {
  pub addr: ValueID,
  pub value: ValueID,
}

impl Store {
  pub fn new(addr: ValueID, value: ValueID) -> Self {
    Self { addr, value }
  }
}

/// Load value from address: result = *addr
#[derive(Debug)]
pub struct Load {
  pub addr: ValueID,
}

impl Load {
  pub fn new(addr: ValueID) -> Self {
    Self { addr }
  }
}
/// Stack allocation.
/// result = alloca typeof(type)
/// Used for local variables that must live in memory (e.g., if their address is taken).
#[derive(Debug)]
pub struct Alloca {}

impl Alloca {
  pub fn new() -> Self {
    Self {}
  }
}
/// memory opeartion's `addr` must have type [`super::Type::Pointer`]
/// and the pointee type cannot be [`super::Type::Function`] or [`super::Type::Label`] (opaque pointer, we cannotr know, MUST check at construction),
/// which means the `Value` behind `ValueID` cannnot be a [`super::module::Function`] or [`super::BasicBlock`].
#[derive(Debug)]
pub enum Memory {
  Store(Store),
  Load(Load),
  Alloca(Alloca),
}

#[derive(Debug)]
pub enum Cast {
  // add later.
}

/// Function call: result = call func(args)
///
/// - [`Call::callee`] is usually a [`super::module::Function`], but can also be other except [`super::BasicBlock`].
/// - [`Call::args`] cannot contain [`super::BasicBlock`] and [`super::Function`] (always as a pointer form -- load ptr inst)
/// - The size of [`Call::args`] must match the parameter count of the parameter counts in [`super::types::Function`].
#[derive(Debug)]
pub struct Call {
  pub callee: ValueID,
  pub args: Vec<ValueID>,
}

/// This mimics LLVM ir's catagory.
#[derive(Debug)]
pub enum Instruction {
  Phi(Phi),
  Terminator(Terminator),
  Unary(Unary),
  Binary(Binary),
  Memory(Memory),
  Cast(Cast),
  Call(Call),
  ICmp(ICmp),
  // etc...
}
::rcc_utils::interconvert!(Alloca, Memory);
::rcc_utils::interconvert!(Load, Memory);
::rcc_utils::interconvert!(Store, Memory);

::rcc_utils::interconvert!(Phi, Instruction);
::rcc_utils::interconvert!(Terminator, Instruction);
::rcc_utils::interconvert!(Unary, Instruction);
::rcc_utils::interconvert!(Binary, Instruction);
::rcc_utils::interconvert!(Memory, Instruction);
::rcc_utils::interconvert!(Cast, Instruction);
::rcc_utils::interconvert!(Call, Instruction);
::rcc_utils::interconvert!(ICmp, Instruction);

::rcc_utils::make_trio_for!(Call, Instruction);
::rcc_utils::make_trio_for!(Phi, Instruction);
::rcc_utils::make_trio_for!(Terminator, Instruction);
