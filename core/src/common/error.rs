use super::{SourceManager, SourceSpan};

/// im focusing the ast right now, so left error handling as a placeholder
pub type Error = ();
#[derive(Debug)]
pub struct ErrorV2 {
  pub span: SourceSpan,
  pub error: Data,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Data {
  // lexing errors
  UnexpectedCharacter(char),
  UnterminatedString,
  InvalidNumberFormat(String),
}
impl ErrorV2 {
  pub fn new(span: SourceSpan, error: Data) -> Self {
    Self { span, error }
  }

  pub fn display_with<'a>(&'a self, sm: &'a SourceManager) -> ErrorDisplay<'a> {
    ErrorDisplay {
      error: self,
      source_manager: sm,
    }
  }
}

pub struct ErrorDisplay<'a> {
  error: &'a ErrorV2,
  source_manager: &'a SourceManager,
}

impl<'a> std::fmt::Display for ErrorDisplay<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let span = &self.error.span;
    let file = &self.source_manager.files[span.file_index as usize];
    let coord = self.source_manager.lookup_line_col(*span);

    write!(
      f,
      "{}:{}:{}: ",
      file.path.to_str().unwrap_or("<invalid utf8>"),
      coord.line,
      coord.column
    )?;

    match &self.error.error {
      Data::UnexpectedCharacter(c) => write!(f, "Unexpected character '{}'", c),
      Data::UnterminatedString => write!(f, "Unterminated string literal"),
      Data::InvalidNumberFormat(s) =>
        write!(f, "Invalid number format '{}'", s),
    }
  }
}
