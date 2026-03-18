use crate::{common::SourceManager, diagnosis::Operational, ir, types};

#[derive(Debug)]
pub struct Session<'c> {
  diagnosis: Operational<'c>,
  manager: &'c SourceManager,
  ast_context: &'c types::Context<'c>,
  ir_context: &'c ir::Context<'c>,
}

pub type SessionRef<'c> = &'c Session<'c>;

impl<'c> Session<'c> {
  pub fn new(
    manager: &'c SourceManager,
    ast_context: &'c types::Context<'c>,
    ir_context: &'c ir::Context<'c>,
  ) -> Self {
    Self {
      diagnosis: Operational::default(),
      manager,
      ast_context,
      ir_context,
    }
  }
}
impl<'c> Session<'c> {
  pub fn ast(&self) -> &'c types::Context<'c> {
    self.ast_context
  }

  pub fn ir(&self) -> &'c ir::Context<'c> {
    self.ir_context
  }

  pub fn diag(&self) -> &Operational<'c> {
    &self.diagnosis
  }

  pub fn src(&self) -> &'c SourceManager {
    self.manager
  }
}
