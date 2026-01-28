use ::rc_utils::{DisplayWith, Dummy, static_assert};
use ::std::path::PathBuf;

pub trait Display<'a, DisplayHelperType: ::std::fmt::Display>:
  DisplayWith<'a, Manager, DisplayHelperType>
{
}
/// if a type implements DisplayWith for a Manager, automatically give it the Display trait.
impl<'a, T, DisplayHelperType: ::std::fmt::Display>
  Display<'a, DisplayHelperType> for T
where
  T: DisplayWith<'a, Manager, DisplayHelperType>,
{
}

pub type Id = u32;
static_assert!(
  ::std::mem::needs_drop::<Id>() == false,
  "Id should be a POD type"
);

pub type Index = u32;
static_assert!(
  ::std::mem::needs_drop::<Index>() == false,
  "Index should be a POD type"
);

#[derive(Debug, Clone, Copy, Default)]
pub struct Span {
  pub file_index: Id,
  pub start: Index,
  pub end: Index,
}
impl Span {
  pub fn new(file_index: Id, start: Index, end: Index) -> Self {
    Self {
      file_index,
      start,
      end,
    }
  }
}
impl Dummy for Span {
  #[inline]
  fn dummy() -> Self {
    Self {
      file_index: Id::dummy(),
      start: Index::dummy(),
      end: Index::dummy(),
    }
  }
}
static_assert!(
  ::std::mem::needs_drop::<Span>() == false,
  "Span should be a POD type"
);

#[derive(Debug, Clone, Copy, Default)]
pub struct Coordinate {
  pub line: Index,
  pub column: Index,
}
static_assert!(
  ::std::mem::needs_drop::<Coordinate>() == false,
  "Coordinate should be a POD type"
);

#[derive(Debug, Default)]
pub struct File {
  pub path: PathBuf,
  pub source: String,
  /// The byte index where each line starts.
  /// line_offsets[0] is always 0.
  /// line_offsets[1] is the start of line 2, etc.
  pub line_offsets: Vec<usize>,
}
#[derive(Debug, Default)]
pub struct Manager {
  pub files: Vec<File>,
}
impl Manager {
  pub fn try_add_file(&mut self, path: PathBuf) -> ::std::io::Result<u32> {
    let source = ::std::fs::read_to_string(&path)?;
    Ok(self.add_file(path, source))
  }

  pub fn add_file(&mut self, path: PathBuf, source: String) -> u32 {
    let mut line_offsets = vec![0]; // line 1 starts at byte 0

    for (idx, char) in source.char_indices() {
      if char == '\n' {
        line_offsets.push(idx + 1);
      }
    }

    let file_id = self.files.len() as u32;
    self.files.push(File {
      path,
      source,
      line_offsets,
    });

    file_id
  }

  pub fn lookup_line_col(&self, span: Span) -> Coordinate {
    let file = &self.files[span.file_index as usize];

    // Binary search to find the line number --
    //   we want the largest index `i` such that `line_offsets[i] <= span.start`.
    let line_idx = file
      .line_offsets
      .partition_point(|&offset| offset <= span.start as usize)
      - 1;

    //(Current Byte) - (Start of Current Line)
    let line_start = file.line_offsets[line_idx];
    let col_idx = (span.start as usize) - line_start;

    Coordinate {
      line: (line_idx + 1) as Index,
      column: (col_idx + 1) as Index,
    }
  }

  /// get the actual text for a span
  pub fn get_slice(&self, span: Span) -> &str {
    let file = &self.files[span.file_index as usize];
    &file.source[span.start as usize..span.end as usize]
  }
}

pub struct SpanDisplay<'a> {
  span: &'a Span,
  source_manager: &'a Manager,
}
impl<'a> DisplayWith<'a, Manager, SpanDisplay<'a>> for Span {
  fn display_with(&'a self, source_manager: &'a Manager) -> SpanDisplay<'a> {
    SpanDisplay {
      span: self,
      source_manager,
    }
  }
}
impl<'a> ::std::fmt::Display for SpanDisplay<'a> {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    let span = self.span;
    let file = &self.source_manager.files[span.file_index as usize];
    let coord = self.source_manager.lookup_line_col(*span);

    write!(
      f,
      "{}:{}:{}",
      file.path.to_str().unwrap_or("<invalid utf8>"),
      coord.line,
      coord.column
    )
  }
}
