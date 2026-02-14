use ::rcc_utils::SmallString;

use crate::types::{Constant, QualifiedType};

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
  /// A Virtual Register (vreg).
  ///
  /// Covers **both** user variables (`int x`) and compiler temps (`%1`).
  /// We use a usize ID because string lookups are slow in the backend.
  Reg(usize),

  /// A Global Label.
  ///
  /// This represents the **Address** of the global(i.e., [`Function`] and [`Variable`])
  /// Effectively a link-time constant.
  Label(SmallString),

  /// A Fixed Constyant (Immediate).
  Imm(Constant),
}

/// result = phi [val1, label1], [val2, label2]
#[derive(Debug, Clone)]
pub struct Phi {
  pub result: Operand, // The register defining the merged value
  pub incomings: Vec<(Operand, SmallString)>, // (Value, From_Block_Label)
}

pub struct Jump {
  pub label: SmallString,
}
pub struct Branch {
  pub cond: Operand,
  pub true_label: SmallString,
  pub false_label: SmallString,
}
pub struct Return {
  pub returne: Option<Operand>,
}
pub enum Terminator {
  /// Unconditional jump
  Jump(Jump),
  /// Conditional branch: if cond goto true_label else goto false_label
  Branch(Branch),
  /// Return from function
  Return(Return),
}

/// result = unary_op operand
pub struct Unary {
  pub result: Operand,
  pub operator: UnaryOp,
  pub operand: Operand,
  pub qualified_type: QualifiedType,
}
pub enum UnaryOp {
  Neg,
  Not,
  Compl,
}
/// result = binary_op lhs, rhs
pub struct Binary {
  pub result: Operand,
  pub operator: BinaryOp,
  pub lhs: Operand,
  pub rhs: Operand,
  pub qualified_type: QualifiedType,
}
// arithematic ops only consider integer for now
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

pub struct ICmp {
  pub result: Operand,
  pub predicate: ICmpPredicate,
  pub lhs: Operand,
  pub rhs: Operand,
  pub qualified_type: QualifiedType, // type of operands.
}
#[derive(Debug, Clone, Copy)]
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
pub struct Store {
  pub addr: Operand,
  pub value: Operand,
  pub qualified_type: QualifiedType,
}

/// Load value from address: result = *addr
pub struct Load {
  pub result: Operand,
  pub addr: Operand,
  pub qualified_type: QualifiedType,
}
pub enum Memory {
  Store(Store),
  Load(Load),
  Alloca(Alloca),
}
/// Stack allocation.
/// result = alloca typeof(type)
/// Used for local variables that must live in memory (e.g., if their address is taken).
pub struct Alloca {
  pub result: Operand,
  pub qualified_type: QualifiedType,
}

pub enum Cast {
  // add later.
}

/// Function call: result = call func(args)
pub struct Call {
  pub result: Option<Operand>,
  pub func: Operand,
  pub args: Vec<Operand>,
}

/// This mimics LLVM ir's catagory.
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
