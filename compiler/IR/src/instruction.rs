use ::rcc_adt::Signedness;
use ::rcc_shared::Operator;

use super::ValueID;

pub trait User {
  fn use_list(&self) -> &[ValueID];
}

/// Creater must ensure [`Jump::label`] must be am ID points to a [`super::BasicBlock`].
#[derive(Debug)]
pub struct Jump {
  operands: [ValueID; 1],
}

impl Jump {
  pub fn new(to: ValueID) -> Self {
    Self { operands: [to] }
  }

  pub fn target(&self) -> ValueID {
    self.operands[0]
  }

  pub fn set_target(&mut self, to: ValueID) {
    self.operands[0] = to;
  }
}
impl User for Jump {
  fn use_list(&self) -> &[ValueID] {
    &self.operands
  }
}
/// Creater must ensure [`Branch::true_label`] and [`Branch::false_label`] must be am ID points to a [`super::BasicBlock`].
///
/// The owner of this instruction must ensure the type of [`Branch::cond`] is i1 (boolean).
#[derive(Debug)]
pub struct Branch {
  operands: [ValueID; 3], // [cond, then_label, else_label]
}

impl Branch {
  pub fn new(
    condition: ValueID,
    then_branch: ValueID,
    else_branch: ValueID,
  ) -> Self {
    Self {
      operands: [condition, then_branch, else_branch],
    }
  }

  pub fn condition(&self) -> ValueID {
    self.operands[0]
  }

  pub fn then_branch(&self) -> ValueID {
    self.operands[1]
  }

  pub fn else_branch(&self) -> ValueID {
    self.operands[2]
  }

  pub fn set_condition(&mut self, condition: ValueID) {
    self.operands[0] = condition;
  }

  pub fn set_then_branch(&mut self, then_branch: ValueID) {
    self.operands[1] = then_branch;
  }

  pub fn set_else_branch(&mut self, else_branch: ValueID) {
    self.operands[2] = else_branch;
  }
}
impl User for Branch {
  fn use_list(&self) -> &[ValueID] {
    &self.operands
  }
}
/// Must match the return type of the function. For void function, [`Return::result`] should be [`None`].
#[derive(Debug)]
pub struct Return {
  operands: [ValueID; 1], // for void function, this operand should be null
}

impl Return {
  pub fn new(result: Option<ValueID>) -> Self {
    Self {
      operands: [result.unwrap_or(ValueID::null())],
    }
  }

  pub fn result(&self) -> Option<ValueID> {
    if self.operands[0].is_null() {
      None
    } else {
      Some(self.operands[0])
    }
  }

  pub fn set_result(&mut self, result: Option<ValueID>) {
    self.operands[0] = result.unwrap_or(ValueID::null());
  }
}
impl User for Return {
  fn use_list(&self) -> &[ValueID] {
    if self.operands[0].is_null() {
      &[]
    } else {
      &self.operands
    }
  }
}
#[derive(Debug, Default)]
pub struct Unreachable;

impl Unreachable {
  pub fn new() -> Self {
    Self
  }
}
impl User for Unreachable {
  fn use_list(&self) -> &[ValueID] {
    &[]
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
  /// Placeholder or unreachable.
  Unreachable(Unreachable),
}

impl User for Terminator {
  fn use_list(&self) -> &[ValueID] {
    static_dispatch!(self, |variant| variant.use_list() => Jump Branch Return Unreachable)
  }
}

/// result = unary_op operand
#[derive(Debug)]
pub struct Unary {
  operator: UnaryOp,
  operand: [ValueID; 1],
}
#[derive(Debug)]
pub enum UnaryOp {
  FNeg,
}
impl Unary {
  pub fn new(operator: UnaryOp, operand: ValueID) -> Self {
    Self {
      operator,
      operand: [operand],
    }
  }

  pub fn operand(&self) -> ValueID {
    self.operand[0]
  }

  pub fn operator(&self) -> &UnaryOp {
    &self.operator
  }

  pub fn set_operand(&mut self, operand: ValueID) {
    self.operand[0] = operand;
  }
}

impl User for Unary {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
  }
}

/// result = binary_op lhs, rhs
///
/// - The type of `lhs` and `rhs` must be the same.
/// - `lhs` and `rhs` cannot be [`super::module::Function`], [`super::module::BasicBlock`] or [`super::module::Variable`].  
#[derive(Debug)]
pub struct Binary {
  operator: BinaryOp,
  operand: [ValueID; 2],
}

impl Binary {
  pub fn new(operator: BinaryOp, left: ValueID, right: ValueID) -> Self {
    Self {
      operator,
      operand: [left, right],
    }
  }

  pub fn operator(&self) -> BinaryOp {
    self.operator
  }

  pub fn left(&self) -> ValueID {
    self.operand[0]
  }

  pub fn right(&self) -> ValueID {
    self.operand[1]
  }
}
impl User for Binary {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
  }
}
// arithematic ops only consider integer for now
#[derive(Debug, Clone, Copy, ::strum_macros::Display)]
#[strum(serialize_all = "lowercase")]
pub enum BinaryOp {
  Add,
  FAdd,
  Sub,
  FSub,
  Mul,
  FMul,
  UDiv,
  SDiv,
  FDiv,
  URem,
  SRem,
  FRem,
  /// Bitwise And.
  And,
  /// Bitwise Or.
  Or,
  /// Bitwise eXclusive or.
  Xor,
  /// Shift Left.
  Shl,
  /// Logical Shift Right for unsigned integers.
  LShr,
  /// for signed integers.
  AShr,
}
impl BinaryOp {
  pub const fn from_op_and_sign(
    operator: Operator,
    signedness: Signedness,
  ) -> Option<BinaryOp> {
    use BinaryOp::*;
    use Operator::*;
    use Signedness::*;
    match (operator, signedness) {
      (Plus, _) => Some(Add),
      (Minus, _) => Some(Sub),
      (Star, _) => Some(Mul),
      (Slash, Signed) => Some(SDiv),
      (Slash, Unsigned) => Some(UDiv),
      (Percent, Signed) => Some(SRem),
      (Percent, Unsigned) => Some(URem),
      (Ampersand, _) => Some(BinaryOp::And),
      (Pipe, _) => Some(BinaryOp::Or),
      (Caret, _) => Some(Xor),
      (LeftShift, _) => Some(Shl),
      (RightShift, Signed) => Some(AShr),
      (RightShift, Unsigned) => Some(LShr),
      _ => None,
    }
  }
}
#[derive(Debug)]
pub struct ICmp {
  predicate: ICmpPredicate,
  operand: [ValueID; 2],
}

impl ICmp {
  pub fn new(predicate: ICmpPredicate, lhs: ValueID, rhs: ValueID) -> Self {
    Self {
      predicate,
      operand: [lhs, rhs],
    }
  }

  pub fn predicate(&self) -> ICmpPredicate {
    self.predicate
  }

  pub fn lhs(&self) -> ValueID {
    self.operand[0]
  }

  pub fn rhs(&self) -> ValueID {
    self.operand[1]
  }
}
impl User for ICmp {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
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
  predicate: FCmpPredicate,
  // pub lhs: ValueID,
  // pub rhs: ValueID,
  operand: [ValueID; 2],
}

impl FCmp {
  pub fn new(predicate: FCmpPredicate, lhs: ValueID, rhs: ValueID) -> Self {
    Self {
      predicate,
      operand: [lhs, rhs],
    }
  }

  pub fn predicate(&self) -> FCmpPredicate {
    self.predicate
  }

  pub fn lhs(&self) -> ValueID {
    self.operand[0]
  }

  pub fn rhs(&self) -> ValueID {
    self.operand[1]
  }
}
impl User for FCmp {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
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
impl FCmpPredicate {
  pub const fn from_op(operator: Operator) -> Self {
    use FCmpPredicate::*;
    use Operator::*;
    match operator {
      Less => Olt,
      LessEqual => Ole,
      Greater => Ogt,
      GreaterEqual => Oge,
      EqualEqual => Oeq,
      // `NaN` always not equal than other, even both are `NaN`.
      NotEqual => Une,
      _ => unreachable!(),
    }
  }
}
#[derive(Debug)]
pub enum Cmp {
  ICmp(ICmp),
  FCmp(FCmp),
}
impl User for Cmp {
  fn use_list(&self) -> &[ValueID] {
    static_dispatch!(self, |variant| variant.use_list() => ICmp FCmp)
  }
}
/// Store value to address: *addr = value
///
/// [`Store::addr`] must have pointer type
#[derive(Debug)]
pub struct Store {
  operand: [ValueID; 2],
}

impl Store {
  pub fn new(target: ValueID, from: ValueID) -> Self {
    Self {
      operand: [target, from],
    }
  }

  pub fn data(&self) -> ValueID {
    self.operand[0]
  }

  pub fn addr(&self) -> ValueID {
    self.operand[1]
  }
}
impl User for Store {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
  }
}

/// Load value from address: result = *addr
#[derive(Debug)]
pub struct Load {
  operand: [ValueID; 1],
}

impl Load {
  pub fn new(from: ValueID) -> Self {
    Self { operand: [from] }
  }

  pub fn addr(&self) -> ValueID {
    self.operand[0]
  }
}
impl User for Load {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
  }
}
/// Stack allocation.
/// result = alloca typeof(type)
/// Used for local variables that must live in memory (e.g., if their address is taken).
#[derive(Debug, Default)]
pub struct Alloca;

impl Alloca {
  pub fn new() -> Self {
    Self
  }
}
impl User for Alloca {
  fn use_list(&self) -> &[ValueID] {
    &[]
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
impl User for Memory {
  fn use_list(&self) -> &[ValueID] {
    static_dispatch!(self, |variant| variant.use_list() => Store Load Alloca)
  }
}
/// the target width must be smaller than the operand.
#[derive(Debug)]
pub struct Trunc {
  /// operand type must be [`super::Type::Integer`].
  // pub operand: ValueID,
  operand: [ValueID; 1],
}

impl Trunc {
  pub fn new(operand: ValueID) -> Self {
    Self { operand: [operand] }
  }

  pub fn operand(&self) -> ValueID {
    self.operand[0]
  }
}
impl User for Trunc {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
  }
}
/// the target width must be larger than the operand.
// #[repr(align(0x10))]
#[derive(Debug)]
pub struct Zext {
  /// operand type must be [`super::Type::Integer`].
  // pub operand: ValueID,
  operand: [ValueID; 1],
}

impl Zext {
  pub fn new(operand: ValueID) -> Self {
    Self { operand: [operand] }
  }

  pub fn operand(&self) -> ValueID {
    self.operand[0]
  }
}
impl User for Zext {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
  }
}
/// the target width must be larger than the operand.
#[derive(Debug)]
pub struct Sext {
  /// operand type must be [`super::Type::Integer`].
  // pub operand: ValueID,
  operand: [ValueID; 1],
}

impl Sext {
  pub fn new(operand: ValueID) -> Self {
    Self { operand: [operand] }
  }

  pub fn operand(&self) -> ValueID {
    self.operand[0]
  }
}

impl User for Sext {
  fn use_list(&self) -> &[ValueID] {
    &self.operand
  }
}

#[derive(Debug)]
pub enum Cast {
  Trunc(Trunc),
  Zext(Zext),
  Sext(Sext),
  // ...
}
impl User for Cast {
  fn use_list(&self) -> &[ValueID] {
    static_dispatch!(self, |variant| variant.use_list() => Trunc Zext Sext)
  }
}

/// Function call: result = call func(args)
///
/// - [`Call::callee`] is usually a [`super::module::Function`], but can also be other except [`super::BasicBlock`].
/// - [`Call::args`] cannot contain [`super::BasicBlock`] and [`super::Function`] (always as a pointer form -- load ptr inst)
/// - The size of [`Call::args`] must match the parameter count of the parameter counts in [`super::types::Function`].
#[derive(Debug)]
pub struct Call {
  // pub callee: ValueID,
  // pub args: Vec<ValueID>,
  operands: Vec<ValueID>, // [callee, arg1, arg2, ...]
}

impl Call {
  pub fn new(operands: Vec<ValueID>) -> Self {
    Self { operands }
  }

  pub fn callee(&self) -> ValueID {
    self.operands[0]
  }

  pub fn args(&self) -> &[ValueID] {
    &self.operands[1..]
  }
}

impl User for Call {
  fn use_list(&self) -> &[ValueID] {
    &self.operands
  }
}
/// result = phi [val1, label1], [val2, label2]
///
/// if phi being used, it must be at the start of current block and has as many pairs as the branch had.
#[derive(Debug, Clone)]
pub struct Phi {
  operands: Vec<ValueID>, // (Value, From_Block_Label) pair.
}

impl Phi {
  pub fn new(operands: Vec<ValueID>) -> Self {
    Self { operands }
  }

  pub fn flat_view(&self) -> &[ValueID] {
    &self.operands
  }

  pub fn incomings(&self) -> &[(ValueID, ValueID)] {
    debug_assert!(self.operands.len().is_multiple_of(2));
    unsafe {
      ::std::slice::from_raw_parts(
        self.operands.as_ptr() as *const (ValueID, ValueID),
        self.operands.len() / 2,
      )
    }
  }
}
impl User for Phi {
  fn use_list(&self) -> &[ValueID] {
    &self.operands
  }
}

#[derive(Debug)]
pub struct Select {
  operands: [ValueID; 3], // [condition, true_value, false_value]
}
impl User for Select {
  fn use_list(&self) -> &[ValueID] {
    &self.operands
  }
}
impl Select {
  pub fn new(
    condition: ValueID,
    true_value: ValueID,
    false_value: ValueID,
  ) -> Self {
    Self {
      operands: [condition, true_value, false_value],
    }
  }

  pub fn condition(&self) -> ValueID {
    self.operands[0]
  }

  pub fn true_value(&self) -> ValueID {
    self.operands[1]
  }

  pub fn false_value(&self) -> ValueID {
    self.operands[2]
  }
}
/// This mimics LLVM ir's catagory.
#[derive(Debug)]
pub enum Instruction {
  Terminator(Terminator),
  Unary(Unary),
  Binary(Binary),
  Memory(Memory),
  Cast(Cast),
  Call(Call),
  Cmp(Cmp),
  Phi(Phi),
  Select(Select),
}
impl User for Instruction {
  fn use_list(&self) -> &[ValueID] {
    static_dispatch!(
      self,
      |variant| variant.use_list() => Phi Terminator Unary Binary Memory Cast Call Cmp Select
    )
  }
}

use ::rcc_utils::{interconvert, make_trio_for, static_dispatch};

interconvert!(Branch, Terminator);
interconvert!(Jump, Terminator);
interconvert!(Return, Terminator);
interconvert!(Unreachable, Terminator);

make_trio_for!(Branch, Terminator);
make_trio_for!(Jump, Terminator);
make_trio_for!(Return, Terminator);
make_trio_for!(Unreachable, Terminator);

interconvert!(Trunc, Cast);
interconvert!(Zext, Cast);
interconvert!(Sext, Cast);

interconvert!(Alloca, Memory);
interconvert!(Load, Memory);
interconvert!(Store, Memory);

interconvert!(ICmp, Cmp);
interconvert!(FCmp, Cmp);

interconvert!(Phi, Instruction);
interconvert!(Terminator, Instruction);
interconvert!(Unary, Instruction);
interconvert!(Binary, Instruction);
interconvert!(Memory, Instruction);
interconvert!(Cast, Instruction);
interconvert!(Call, Instruction);
interconvert!(Cmp, Instruction);
interconvert!(Select, Instruction);

make_trio_for!(Call, Instruction);
make_trio_for!(Phi, Instruction);
make_trio_for!(Terminator, Instruction);
