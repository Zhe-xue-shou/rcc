use super::value::ValueID;

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

impl Binary {
  pub fn new(operator: BinaryOp, lhs: ValueID, rhs: ValueID) -> Self {
    Self { operator, lhs, rhs }
  }
}
// arithematic ops only consider integer for now
#[derive(Debug, Clone, Copy, ::strum_macros::Display)]
#[strum(serialize_all = "lowercase")]
pub enum BinaryOp {
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  BitwiseAnd,
  BitwiseOr,
  Xor,
  LeftShift,
  RightShift,
}

#[derive(Debug)]
pub struct ICmp {
  pub predicate: ICmpPredicate,
  pub lhs: ValueID,
  pub rhs: ValueID,
}

impl ICmp {
  pub fn new(predicate: ICmpPredicate, lhs: ValueID, rhs: ValueID) -> Self {
    Self {
      predicate,
      lhs,
      rhs,
    }
  }
}
#[derive(Debug, Clone, Copy, ::strum_macros::Display)]
#[strum(serialize_all = "lowercase")]
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
/// the target width must be smaller than the operand.
#[derive(Debug)]
pub struct Trunc {
  /// operand type must be [`super::Type::Integer`].
  pub operand: ValueID,
}

impl Trunc {
  pub fn new(operand: ValueID) -> Self {
    Self { operand }
  }
}
/// the target width must be larger than the operand.
// #[repr(align(0x10))]
#[derive(Debug)]
pub struct Zext {
  /// operand type must be [`super::Type::Integer`].
  pub operand: ValueID,
}

impl Zext {
  pub fn new(operand: ValueID) -> Self {
    Self { operand }
  }
}
/// the target width must be larger than the operand.
#[derive(Debug)]
pub struct Sext {
  /// operand type must be [`super::Type::Integer`].
  pub operand: ValueID,
}

impl Sext {
  pub fn new(operand: ValueID) -> Self {
    Self { operand }
  }
}

#[derive(Debug)]
pub enum Cast {
  Trunc(Trunc),
  Zext(Zext),
  Sext(Sext),
  // ...
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

impl Call {
  pub fn new(callee: ValueID, args: Vec<ValueID>) -> Self {
    Self { callee, args }
  }
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
use ::rcc_utils::{interconvert, make_trio_for};

interconvert!(Trunc, Cast);
interconvert!(Zext, Cast);
interconvert!(Sext, Cast);

interconvert!(Alloca, Memory);
interconvert!(Load, Memory);
interconvert!(Store, Memory);

interconvert!(Phi, Instruction);
interconvert!(Terminator, Instruction);
interconvert!(Unary, Instruction);
interconvert!(Binary, Instruction);
interconvert!(Memory, Instruction);
interconvert!(Cast, Instruction);
interconvert!(Call, Instruction);
interconvert!(ICmp, Instruction);

make_trio_for!(Call, Instruction);
make_trio_for!(Phi, Instruction);
make_trio_for!(Terminator, Instruction);
