use ::rcc_utils::SmallString;

use super::instruction::{Instruction, Operand};
use crate::types::{Constant, QualifiedType};

/// keep it an alias type for latter convenience if choosing to optimize a lot
/// (e.g., switch to VecDeque, LinkedList or intrusive list. since now it's better for me to focus on the compiler design, not ADT.)
///
/// This name is from `llvm/ADT/ilist.h` and `llvm/ADT/ilist_node.h`, which is a doubly-linked intrusive list used to increase efficiency.
#[allow(non_camel_case_types)]
pub type ilist_type<T> = Vec<T>;

/// TODO: move counter inside a dedicated struct to decouple the module.
pub struct Module {
  pub functions: ilist_type<Function>,
  pub globals: Vec<Variable>,
  /// counter for generating unique temporary names
  temp_counter: usize,
  /// counter for generating unique label names
  label_counter: usize,
}

/// **Global** function in TAC-SSA form
pub struct Function {
  pub name: SmallString,
  pub params: Vec<Operand>,
  pub blocks: ilist_type<BasicBlock>,
  pub return_type: QualifiedType,
  pub is_variadic: bool,
}

/// **Global** Variable. Non-static local variable won't be stored here, but exists as [`Operand`].
pub struct Variable {
  pub name: SmallString,
  pub qualified_type: QualifiedType,
  // pub storage_class: Storage, // should either be extern or static?
  // pub declkind: VarDeclKind,
  pub initializer: Option<Initializer>,
}

pub struct BasicBlock {
  pub label: SmallString,
  pub instructions: ilist_type<Instruction>,
}

/// **Static** initializer.
#[derive(Debug, Clone)]
pub enum Initializer {
  Scalar(Constant),
  Aggregate(Vec<Initializer>),
}
