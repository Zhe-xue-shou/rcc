use ::rcc_utils::SmallString;

use crate::types::{Constant, QualifiedType};

#[derive(Debug, Clone, PartialEq)]
pub enum Operand<'context> {
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

  /// A Fixed Constant (Immediate).
  Imm(Constant<'context>),
}

/// result = phi [val1, label1], [val2, label2]
///
/// left here as placeholder, do it later.
#[derive(Debug, Clone)]
pub struct Phi<'context> {
  pub result: Operand<'context>, // The register defining the merged value
  pub incomings: Vec<(Operand<'context>, SmallString)>, // (Value, From_Block_Label)
}

#[derive(Debug)]
pub struct Jump {
  pub label: SmallString,
}
#[derive(Debug)]
pub struct Branch<'context> {
  pub cond: Operand<'context>,
  pub true_label: SmallString,
  pub false_label: SmallString,
}
#[derive(Debug)]
pub struct Return<'context> {
  pub returne: Option<Operand<'context>>,
}
#[derive(Debug)]
pub enum Terminator<'context> {
  /// Unconditional jump
  Jump(Jump),
  /// Conditional branch: if cond goto true_label else goto false_label
  Branch(Branch<'context>),
  /// Return from function
  Return(Return<'context>),
}

/// result = unary_op operand
#[derive(Debug)]
pub struct Unary<'context> {
  pub result: Operand<'context>,
  pub operator: UnaryOp,
  pub operand: Operand<'context>,
  pub qualified_type: QualifiedType<'context>,
}
#[derive(Debug)]
pub enum UnaryOp {
  Neg,
  Not,
  Compl,
}
/// result = binary_op lhs, rhs
#[derive(Debug)]
pub struct Binary<'context> {
  pub result: Operand<'context>,
  pub operator: BinaryOp,
  pub lhs: Operand<'context>,
  pub rhs: Operand<'context>,
  pub qualified_type: QualifiedType<'context>,
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
pub struct ICmp<'context> {
  pub result: Operand<'context>,
  pub predicate: ICmpPredicate,
  pub lhs: Operand<'context>,
  pub rhs: Operand<'context>,
  pub qualified_type: QualifiedType<'context>, // type of operands.
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
pub struct Store<'context> {
  pub addr: Operand<'context>,
  pub value: Operand<'context>,
  pub qualified_type: QualifiedType<'context>,
}

/// Load value from address: result = *addr
#[derive(Debug)]
pub struct Load<'context> {
  pub result: Operand<'context>,
  pub addr: Operand<'context>,
  pub qualified_type: QualifiedType<'context>,
}
#[derive(Debug)]
pub enum Memory<'context> {
  Store(Store<'context>),
  Load(Load<'context>),
  Alloca(Alloca<'context>),
}
/// Stack allocation.
/// result = alloca typeof(type)
/// Used for local variables that must live in memory (e.g., if their address is taken).
#[derive(Debug)]
pub struct Alloca<'context> {
  pub result: Operand<'context>,
  pub qualified_type: QualifiedType<'context>,
}

#[derive(Debug)]
pub enum Cast<'a> {
  // add later.
  Placeholder(&'a u8),
}

/// Function call: result = call func(args)
#[derive(Debug)]
pub struct Call<'context> {
  pub result: Option<Operand<'context>>,
  pub func: Operand<'context>,
  pub args: Vec<Operand<'context>>,
}

impl<'context> Call<'context> {
  pub fn new(
    result: Option<Operand<'context>>,
    func: Operand<'context>,
    args: Vec<Operand<'context>>,
  ) -> Self {
    Self { result, func, args }
  }
}

/// This mimics LLVM ir's catagory.
#[derive(Debug)]
pub enum Instruction<'context> {
  Phi(Phi<'context>),
  Terminator(Terminator<'context>),
  Unary(Unary<'context>),
  Binary(Binary<'context>),
  Memory(Memory<'context>),
  Cast(Cast<'context>),
  Call(Call<'context>),
  ICmp(ICmp<'context>),
  // etc...
}

::rcc_utils::interconvert!(Phi, Instruction,'context);
::rcc_utils::interconvert!(Terminator, Instruction,'context);
::rcc_utils::interconvert!(Unary, Instruction,'context);
::rcc_utils::interconvert!(Binary, Instruction,'context);
::rcc_utils::interconvert!(Memory, Instruction,'context);
::rcc_utils::interconvert!(Cast, Instruction,'context);
::rcc_utils::interconvert!(Call, Instruction,'context);
::rcc_utils::interconvert!(ICmp, Instruction,'context);
