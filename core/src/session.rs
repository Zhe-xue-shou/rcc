use crate::{common::SourceManager, diagnosis::Operational, ir, types};

#[derive(Debug)]
pub struct Session<'c> {
  pub diagnosis: Operational<'c>,
  pub manager: &'c SourceManager,
  pub ast_context: &'c types::Context<'c>,
  pub ir_context: &'c ir::Context<'c>,
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
