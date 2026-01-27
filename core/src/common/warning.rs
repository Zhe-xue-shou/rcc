use ::rc_utils::{DisplayWith, IntoWith};

use super::{SourceManager, SourceSpan, Storage};
use crate::types::Qualifiers;

#[allow(dead_code)]
type CustomMessage = String;
type Elem = String;

#[derive(Debug)]
pub struct Warning {
  pub span: SourceSpan,
  pub data: Data,
}
#[derive(Debug)]
pub enum Data {
  UnusedVariable(Elem),
  RedundantStorageSpecs(Storage),
  RedundantQualifier(Qualifiers),
  ExternVariableWithInitializer(Elem),
  VariableUninitialized(CustomMessage),
  DeprecatedFunctionNoProto,
  DeprecatedStmtDeclCvt,
  EmptyTypedef,
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
      "{}: ",
      self.warning.span.display_with(self.source_manager)
    )?;

    match &self.warning.data {
      Data::UnusedVariable(name) => write!(f, "Unused variable '{}'", name),
      Data::EmptyStatement => write!(f, "Empty statement"),
      Data::RedundantStorageSpecs(storage) =>
        write!(f, "Redundant storage specifiers '{storage}'"),
      Data::RedundantQualifier(qualifiers) =>
        write!(f, "Redundant type qualifiers '{qualifiers}'"),
      Data::EmptyTypedef => write!(f, "Typedef defines nothing"),
      Data::ExternVariableWithInitializer(name) => write!(
        f,
        "Extern global variable '{}' should not have an initializer",
        name
      ),
      Data::DeprecatedFunctionNoProto => write!(
        f,
        "Function declarations without prototypes(e.g., int main()) are deprecated and removed in C23. Please provide a prototype (e.g., int main(void)) rather than leaving it empty."
      ),
      Data::DeprecatedStmtDeclCvt => write!(
        f,
        "C standard pre C23 does not allow declaration after label, if/else, while, do-while, for, and switch statements(e.g.`while(cond) int i = 0;` is invalid). If it's intended, please use surrounding braces to form a block."
      ),
      Data::VariableUninitialized(msg) => write!(f, "{}", msg),
    }
  }
}
