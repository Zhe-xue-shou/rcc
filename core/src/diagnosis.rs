mod error;
mod warning;
pub use self::{
  error::{Data as ErrorData, Error, ErrorDisplay},
  warning::{Data as WarningData, Warning, WarningDisplay},
};

pub trait Diagnosis {
  #[must_use]
  fn has_errors(&self) -> bool;
  #[must_use]
  fn has_warnings(&self) -> bool;
  #[must_use]
  fn errors(&self) -> &[Error];
  #[must_use]
  fn warnings(&self) -> &[Warning];
  fn add_error(&mut self, error: Error);
  fn add_warning(&mut self, warning: Warning);
}

pub fn default() -> OperationalDiag {
  OperationalDiag::default()
}

pub fn noop() -> NoOpDiag {
  NoOpDiag::default()
}

#[derive(Default)]

pub struct OperationalDiag {
  errors: Vec<Error>,
  warnings: Vec<Warning>,
}

impl Diagnosis for OperationalDiag {
  #[inline]
  fn has_errors(&self) -> bool {
    !self.errors.is_empty()
  }

  #[inline]
  fn has_warnings(&self) -> bool {
    !self.warnings.is_empty()
  }

  #[inline]
  fn errors(&self) -> &[Error] {
    &self.errors
  }

  #[inline]
  fn warnings(&self) -> &[Warning] {
    &self.warnings
  }

  #[inline]
  fn add_error(&mut self, error: Error) {
    self.errors.push(error);
  }

  #[inline]
  fn add_warning(&mut self, warning: Warning) {
    self.warnings.push(warning);
  }
}
#[derive(Default)]
pub struct NoOpDiag;
impl Diagnosis for NoOpDiag {
  #[inline]
  fn has_errors(&self) -> bool {
    false
  }

  #[inline]
  fn has_warnings(&self) -> bool {
    false
  }

  #[inline]
  fn errors(&self) -> &[Error] {
    &[]
  }

  #[inline]
  fn warnings(&self) -> &[Warning] {
    &[]
  }

  #[inline]
  fn add_error(&mut self, _error: Error) {}

  #[inline]
  fn add_warning(&mut self, _warning: Warning) {}
}
