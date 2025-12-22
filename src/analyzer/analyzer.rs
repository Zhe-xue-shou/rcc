use crate::{analyzer::Analyzer, parser::declaration::Program};
// tmeporary
pub struct TranslationUnit;
impl Analyzer {
  pub fn new(program: Program) -> Self {
    Self {
      program,
      errors: Vec::new(),
      warnings: Vec::new(),
    }
  }
  pub fn add_error(&mut self, error: String) {
    self.errors.push(error);
  }
  pub fn add_warning(&mut self, warning: String) {
    self.warnings.push(warning);
  }
  pub fn analyze(&mut self) -> TranslationUnit {
    // todo!()
    TranslationUnit {}
  }
  pub fn errors(&self) -> &Vec<String> {
    &self.errors
  }
  pub fn warnings(&self) -> &Vec<String> {
    &self.warnings
  }
}
