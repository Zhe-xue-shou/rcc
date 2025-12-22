use crate::parser::declaration::Program;

pub mod analyzer;

pub struct Analyzer {
  program: Program,
  errors: Vec<String>,
  warnings: Vec<String>,
}
