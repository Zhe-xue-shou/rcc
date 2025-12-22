pub mod lexer;

use ::std::{path::PathBuf, rc::Rc};
pub struct Lexer {
  source: String,
  chars: Vec<char>,
  byte_positions: Vec<usize>,
  cursor: usize,
  line: u32,
  column: u32,
  errors: Vec<String>,
  filepath: Rc<PathBuf>,
}
