use super::value::ValueID;
use crate::common::{Operator, Signedness};

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
  pub to: ValueID,
}

impl Jump {
  pub fn new(to: ValueID) -> Self {
    Self { to }
  }
}
/// Creater must ensure [`Branch::true_label`] and [`Branch::false_label`] must be am ID points to a [`super::BasicBlock`].
///
/// The owner of this instruction must ensure the type of [`Branch::cond`] is i1 (boolean).
#[derive(Debug)]
pub struct Branch {
  pub condition: ValueID,
  pub then_branch: ValueID,
  pub else_branch: ValueID,
}

impl Branch {
  pub fn new(
    condition: ValueID,
    then_branch: ValueID,
    else_branch: ValueID,
  ) -> Self {
    Self {
      condition,
      then_branch,
      else_branch,
    }
  }
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
  pub left: ValueID,
  pub right: ValueID,
}

impl Binary {
  pub fn new(operator: BinaryOp, left: ValueID, right: ValueID) -> Self {
    Self {
      operator,
      left,
      right,
    }
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
  /// Bitwise And.
  And,
  /// Bitwise Or.
  Or,
  Xor,
  Shl,
  /// Logical Shift Right for unsigned integers.
  LShr,
  /// for signed integers.
  AShr,
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
impl ICmpPredicate {
  pub const fn from_op_and_sign(
    operator: Operator,
    signedness: Signedness,
  ) -> Self {
    use ICmpPredicate::*;
    use Operator::*;
    use Signedness::*;
    match (operator, signedness) {
      (Less, Signed) => Slt,
      (Less, Unsigned) => Ult,
      (LessEqual, Signed) => Sle,
      (LessEqual, Unsigned) => Ule,
      (Greater, Signed) => Sgt,
      (Greater, Unsigned) => Ugt,
      (GreaterEqual, Signed) => Sge,
      (GreaterEqual, Unsigned) => Uge,
      (EqualEqual, _) => Eq,
      (NotEqual, _) => Ne,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug)]
pub struct FCmp {
  pub predicate: FCmpPredicate,
  pub lhs: ValueID,
  pub rhs: ValueID,
}

impl FCmp {
  pub fn new(predicate: FCmpPredicate, lhs: ValueID, rhs: ValueID) -> Self {
    Self {
      predicate,
      lhs,
      rhs,
    }
  }
}

#[derive(Debug, Clone, Copy, ::strum_macros::Display)]
#[strum(serialize_all = "lowercase")]
pub enum FCmpPredicate {
  /// Always `false` if `NaN` is involved.
  Oeq,
  One,
  Olt,
  Ole,
  Ogt,
  Oge,
  /// Always `true` if `NaN` is involved.
  Ueq,
  Une,
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
  pub into: ValueID,
  pub from: ValueID,
}

impl Store {
  pub fn new(into: ValueID, from: ValueID) -> Self {
    Self { into, from }
  }
}

/// Load value from address: result = *addr
#[derive(Debug)]
pub struct Load {
  pub from: ValueID,
}

impl Load {
  pub fn new(from: ValueID) -> Self {
    Self { from }
  }
}
/// Stack allocation.
/// result = alloca typeof(type)
/// Used for local variables that must live in memory (e.g., if their address is taken).
#[derive(Debug)]
pub struct Alloca;

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
  FCmp(FCmp), // etc...
}
use ::rcc_utils::{interconvert, make_trio_for};

interconvert!(Branch, Terminator);
interconvert!(Jump, Terminator);
interconvert!(Return, Terminator);

make_trio_for!(Branch, Terminator);
make_trio_for!(Jump, Terminator);
make_trio_for!(Return, Terminator);

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
interconvert!(FCmp, Instruction);

make_trio_for!(Call, Instruction);
make_trio_for!(Phi, Instruction);
make_trio_for!(Terminator, Instruction);
