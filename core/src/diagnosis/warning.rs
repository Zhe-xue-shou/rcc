use ::rc_utils::{DisplayWith, IntoWith};
use ::thiserror::Error;

use crate::{
  common::{SourceManager, SourceSpan, Storage},
  types::Qualifiers,
};

#[allow(dead_code)]
type CustomMessage = String;
type Elem = String;

#[derive(Debug)]
pub struct Warning {
  pub span: SourceSpan,
  pub data: Data,
}
#[derive(Debug, Error)]
pub enum Data {
  #[error("Unused variable '{0}'")]
  UnusedVariable(Elem),
  #[error("Redundant storage specifiers '{0}'")]
  RedundantStorageSpecs(Storage),
  #[error("Redundant type qualifiers '{0}'")]
  RedundantQualifier(Qualifiers),
  #[error("Extern global variable '{0}' should not have an initializer")]
  ExternVariableWithInitializer(Elem),
  #[error("{0}")]
  VariableUninitialized(CustomMessage),
  #[error(
    "Function declarations without prototypes(e.g., int main()) are deprecated and removed in C23. Please provide a prototype (e.g., int main(void)) rather than leaving it empty."
  )]
  DeprecatedFunctionNoProto,
  #[error(
    "C standard pre C23 does not allow declaration after label, if/else, while, do-while, for, and switch statements(e.g.`while(cond) int i = 0;` is invalid). If it's intended, please use surrounding braces to form a block."
  )]
  DeprecatedStmtDeclCvt,
  #[error("Typedef defines nothing")]
  EmptyTypedef,
  #[error("Empty statement")]
  EmptyStatement,
}

impl Warning {
  pub fn new(span: SourceSpan, data: Data) -> Self {
    Self { span, data }
  }
}
impl IntoWith<SourceSpan, Warning> for Data {
  fn into_with(self, span: SourceSpan) -> Warning {
    Warning::new(span, self)
  }
}
pub struct WarningDisplay<'a> {
  warning: &'a Warning,
  source_manager: &'a SourceManager,
}
impl<'a> DisplayWith<'a, SourceManager, WarningDisplay<'a>> for Warning {
  fn display_with(
    &'a self,
    source_manager: &'a SourceManager,
  ) -> WarningDisplay<'a> {
    WarningDisplay {
      warning: self,
      source_manager,
    }
  }
}
impl<'a> ::std::fmt::Display for WarningDisplay<'a> {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    write!(
      f,
      "{}: {}",
      self.warning.span.display_with(self.source_manager),
      self.warning.data
    )
  }
}
