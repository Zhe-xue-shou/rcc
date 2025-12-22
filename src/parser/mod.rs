pub mod declaration;
pub mod expression;
#[allow(internal_features)]
#[allow(unused_variables)]
pub mod parser;
pub mod statement;

use crate::common::{environment::UnitScope, token::Token};
pub struct Parser {
  tokens: Vec<Token>,
  cursor: usize,
  errors: Vec<String>,
  warnings: Vec<String>,
  loop_labels: Vec<String>,
  // contest-sensitive part :(. needed to parse `T * x`.
  typedefs: UnitScope,
}
