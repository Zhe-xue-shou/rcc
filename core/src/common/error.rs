use ::rc_utils::DisplayWith;

use super::{SourceManager, SourceSpan};

/// im focusing the ast right now, so left error handling as a placeholder
pub type Error = ();
/// Error `Version 2`. Will replace the old `Error` type (which is just ()) soon.
#[derive(Debug)]
pub struct ErrorV2 {
  pub span: SourceSpan,
  pub data: Data,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Data {
  // lexing errors
  UnexpectedCharacter(char),
  UnterminatedString,
  InvalidNumberFormat(String),
}
impl ErrorV2 {
  pub fn new(span: SourceSpan, data: Data) -> Self {
    Self { span, data }
  }
}
impl<'a> DisplayWith<'a, SourceManager, ErrorDisplay<'a>> for ErrorV2 {
  fn display_with(
    &'a self,
    source_manager: &'a SourceManager,
  ) -> ErrorDisplay<'a> {
    ErrorDisplay {
      error: self,
      source_manager,
    }
  }
}

pub struct ErrorDisplay<'a> {
  error: &'a ErrorV2,
  source_manager: &'a SourceManager,
}

impl<'a> ::std::fmt::Display for ErrorDisplay<'a> {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    write!(f, "{}: ", self.error.span.display_with(self.source_manager))?;

    match &self.error.data {
      Data::UnexpectedCharacter(c) => write!(f, "Unexpected character '{}'", c),
      Data::UnterminatedString => write!(f, "Unterminated string literal"),
      Data::InvalidNumberFormat(s) =>
        write!(f, "Invalid number format '{}'", s),
    }
  }
}
