mod data;
use ::std::cell::{Ref, RefCell};

pub use self::data::{Data as DiagData, Diag, Meta as DiagMeta, Severity};
use crate::common::SourceSpan;

pub trait Diagnosis {
  #[must_use]
  fn has_errors(&self) -> bool;
  #[must_use]
  fn has_warnings(&self) -> bool;
  #[must_use]
  fn errors(&self) -> Ref<'_, Vec<Diag>>;
  #[must_use]
  fn warnings(&self) -> Ref<'_, Vec<Diag>>;
  fn add_error(&self, error: DiagData, span: SourceSpan);
  fn add_warning(&self, warning: DiagData, span: SourceSpan);
  fn add_diag(&self, diag: Diag) {
    match diag.metadata.severity {
      Severity::Error => self.add_error(diag.metadata.data, diag.span),
      Severity::Warning => self.add_warning(diag.metadata.data, diag.span),
      Severity::Info => {}, // ignore info for now
    }
  }
}

#[derive(Default, Debug)]

pub struct Operational {
  warnings: RefCell<Vec<Diag>>,
  errors: RefCell<Vec<Diag>>,
}

impl Diagnosis for Operational {
  #[inline]
  fn has_errors(&self) -> bool {
    !self.errors.borrow().is_empty()
  }

  #[inline]
  fn has_warnings(&self) -> bool {
    !self.warnings.borrow().is_empty()
  }

  #[inline]
  fn errors(&self) -> Ref<'_, Vec<Diag>> {
    self.errors.borrow()
  }

  #[inline]
  fn warnings(&self) -> Ref<'_, Vec<Diag>> {
    self.warnings.borrow()
  }

  #[inline]
  fn add_error(&self, error: DiagData, span: SourceSpan) {
    self
      .errors
      .borrow_mut()
      .push(Diag::new(span, Severity::Error, error));
  }

  #[inline]
  fn add_warning(&self, data: DiagData, span: SourceSpan) {
    self
      .warnings
      .borrow_mut()
      .push(Diag::new(span, Severity::Warning, data));
  }
}

pub struct NoOp {
  /// rust strict rules w.r.t. thread safety(!Sync)
  /// and lifetime issues makes it difficult to just create a dummmy noop struct.
  idk: RefCell<Vec<Diag>>,
}
impl ::std::default::Default for NoOp {
  #[inline]
  fn default() -> Self {
    Self {
      idk: RefCell::new(Vec::with_capacity(0)),
    }
  }
}

impl NoOp {
  #[inline]
  pub fn new() -> Self {
    Self::default()
  }
}
impl Diagnosis for NoOp {
  #[inline]
  fn has_errors(&self) -> bool {
    false
  }

  #[inline]
  fn has_warnings(&self) -> bool {
    false
  }

  #[inline]
  fn errors(&self) -> Ref<'_, Vec<Diag>> {
    self.idk.borrow()
  }

  #[inline]
  fn warnings(&self) -> Ref<'_, Vec<Diag>> {
    self.idk.borrow()
  }

  #[inline]
  fn add_error(&self, _error: DiagData, _span: SourceSpan) {}

  #[inline]
  fn add_warning(&self, _warning: DiagData, _span: SourceSpan) {}
}
#[derive(Default, Debug)]
pub struct Session {
  pub diagnosis: Operational,
}
