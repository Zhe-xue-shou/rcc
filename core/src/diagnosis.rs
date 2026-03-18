mod data;
use ::std::cell::{Ref, RefCell};

pub use self::data::{Data as DiagData, Diag, Meta as DiagMeta, Severity};
use crate::common::SourceSpan;

pub trait Diagnosis<'c> {
  #[must_use]
  fn has_errors(&self) -> bool;
  #[must_use]
  fn has_warnings(&self) -> bool;
  #[must_use]
  fn errors(&self) -> Ref<'_, Vec<Diag<'_>>>;
  #[must_use]
  fn warnings(&self) -> Ref<'_, Vec<Diag<'_>>>;
  fn add_error(&self, error: DiagData<'c>, span: SourceSpan);
  fn add_warning(&self, warning: DiagData<'c>, span: SourceSpan);
  fn add_diag(&self, diag: Diag<'c>) {
    match diag.metadata.severity {
      Severity::Error => self.add_error(diag.metadata.data, diag.span),
      Severity::Warning => self.add_warning(diag.metadata.data, diag.span),
      Severity::Info | Severity::Hint => {}, // ignore info for now
    }
  }
}

#[derive(Default, Debug)]

pub struct Operational<'c> {
  warnings: RefCell<Vec<Diag<'c>>>,
  errors: RefCell<Vec<Diag<'c>>>,
}

impl<'c> Diagnosis<'c> for Operational<'c> {
  #[inline]
  fn has_errors(&self) -> bool {
    !self.errors.borrow().is_empty()
  }

  #[inline]
  fn has_warnings(&self) -> bool {
    !self.warnings.borrow().is_empty()
  }

  #[inline]
  fn errors(&self) -> Ref<'_, Vec<Diag<'_>>> {
    self.errors.borrow()
  }

  #[inline]
  fn warnings(&self) -> Ref<'_, Vec<Diag<'_>>> {
    self.warnings.borrow()
  }

  #[inline]
  fn add_error(&self, error: DiagData<'c>, span: SourceSpan) {
    self
      .errors
      .borrow_mut()
      .push(Diag::new(span, Severity::Error, error));
  }

  #[inline]
  fn add_warning(&self, data: DiagData<'c>, span: SourceSpan) {
    self
      .warnings
      .borrow_mut()
      .push(Diag::new(span, Severity::Warning, data));
  }
}

pub struct NoOp {
  /// rust strict rules w.r.t. thread safety(!Sync)
  /// and lifetime issues makes it difficult to just create a dummmy noop struct.
  idk: RefCell<Vec<Diag<'static>>>,
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
impl Diagnosis<'_> for NoOp {
  #[inline]
  fn has_errors(&self) -> bool {
    false
  }

  #[inline]
  fn has_warnings(&self) -> bool {
    false
  }

  #[inline]
  fn errors(&self) -> Ref<'_, Vec<Diag<'_>>> {
    self.idk.borrow()
  }

  #[inline]
  fn warnings(&self) -> Ref<'_, Vec<Diag<'_>>> {
    self.idk.borrow()
  }

  #[inline]
  fn add_error(&self, _error: DiagData, _span: SourceSpan) {}

  #[inline]
  fn add_warning(&self, _warning: DiagData, _span: SourceSpan) {}
}
