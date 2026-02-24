use ::std::io::Write;
use ::termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use super::SourceSpan;
use crate::session::Session;

pub type DumpRes = ::std::io::Result<()>;

#[derive(Default, Clone)]
pub struct Palette {
  pub node_type: ColorSpec, // "BinaryExpr"
  pub operator: ColorSpec,  // "+"/"*"
  pub literal: ColorSpec,   // "42", "'a'"
  pub meta: ColorSpec,      // types, offsets
  pub kind: ColorSpec, // enums like `LValueConversion` in `ImplicitCastExpr`
  pub info: ColorSpec, // span info, pointers
  pub dim: ColorSpec,
  pub skeleton: ColorSpec, // tree
  pub error: ColorSpec,    // overflow info, error nodes
}
::rcc_utils::ensure_is_pod!(Palette);
impl Palette {
  pub fn colored() -> Self {
    Self {
      node_type: ColorSpec::new()
        .set_fg(Some(Color::Cyan))
        .set_bold(true)
        .to_owned(),
      operator: ColorSpec::new().set_fg(Some(Color::Yellow)).to_owned(),
      literal: ColorSpec::new().set_fg(Some(Color::Green)).to_owned(),
      meta: ColorSpec::new().set_fg(Some(Color::Blue)).to_owned(),
      kind: ColorSpec::new()
        .set_fg(Some(Color::Magenta))
        .set_bold(true)
        .to_owned(),
      info: ColorSpec::new()
        .set_fg(Some(Color::Rgb(173, 216, 230)))
        .to_owned(),
      dim: ColorSpec::new()
        .set_fg(Some(Color::Yellow))
        .set_intense(false)
        .to_owned(),
      skeleton: ColorSpec::new()
        .set_fg(Some(Color::White))
        .set_intense(false)
        .to_owned(),
      error: ColorSpec::new()
        .set_fg(Some(Color::Red))
        .set_bold(true)
        .to_owned(),
    }
  }
}

pub trait Dumper<'context, 'source> {
  #[inline(always)]
  fn write(&mut self, text: &str, spec: &ColorSpec) -> DumpRes {
    self.write_fmt(format_args!("{}", text), spec)
  }
  #[inline(always)]
  fn writeln(&mut self, text: &str, spec: &ColorSpec) -> DumpRes {
    self.write_fmt(format_args!("{}\n", text), spec)
  }

  fn write_fmt(
    &mut self,
    args: ::std::fmt::Arguments<'_>,
    spec: &ColorSpec,
  ) -> DumpRes;

  fn newline(&mut self) -> DumpRes;
  fn reset(&mut self) -> DumpRes;
  fn print_indent(&mut self, prefix: &str, is_last: bool) -> DumpRes;

  /// Build the new prefix for children based on whether the current node is the last child.
  #[must_use]
  fn child_prefix(&self, prefix: &str, is_last: bool) -> String;

  #[must_use]
  fn palette(&self) -> &Palette;
  #[must_use]
  fn session(&self) -> &Session<'context, 'source>;
}
pub struct ASTDumper<'session, 'context, 'source>
where
  'context: 'session,
  'source: 'context,
{
  pub(crate) stream: StandardStream,
  pub(crate) palette: Palette,
  /// currently no use, just keep maybe for future extensions.
  #[allow(dead_code)]
  pub(crate) session: &'session Session<'context, 'source>,
}
impl<'session, 'context, 'source> Dumper<'context, 'source>
  for ASTDumper<'session, 'context, 'source>
{
  #[inline]
  fn write_fmt(
    &mut self,
    args: ::std::fmt::Arguments<'_>,
    spec: &ColorSpec,
  ) -> DumpRes {
    self.stream.set_color(spec)?;
    self.stream.write_fmt(args)
  }

  #[inline(always)]
  fn newline(&mut self) -> DumpRes {
    writeln!(self.stream)
  }

  fn print_indent(&mut self, prefix: &str, is_last: bool) -> DumpRes {
    let marker = if is_last { "└── " } else { "├── " };
    self.stream.set_color(&self.palette.skeleton)?;
    write!(self.stream, "{}{}", prefix, marker)
  }

  #[inline(always)]
  fn palette(&self) -> &Palette {
    &self.palette
  }

  /// Build the new prefix for children based on whether the current node is the last child.
  #[inline]
  fn child_prefix(&self, prefix: &str, is_last: bool) -> String {
    if is_last {
      format!("{}    ", prefix) // parent was last → no vertical bar
    } else {
      format!("{}│   ", prefix) // parent was not last → vertical bar continues
    }
  }

  #[inline(always)]
  fn reset(&mut self) -> DumpRes {
    self.stream.reset()
  }

  #[inline(always)]
  fn session(&self) -> &Session<'context, 'source> {
    self.session
  }
}
impl<'session, 'context, 'source> ASTDumper<'session, 'context, 'source> {
  pub fn dump(
    dumpable: &impl Dumpable,
    session: &'session Session<'context, 'source>,
  ) -> DumpRes {
    let mut dumper = Self::new(
      session,
      StandardStream::stdout(ColorChoice::Auto),
      Palette::colored(),
    );
    let palette = dumper.palette().clone();
    dumpable.dump(&mut dumper, "", true, &palette)?;
    dumper.reset()
  }
}
impl<'session, 'context, 'source> ASTDumper<'session, 'context, 'source> {
  pub fn new(
    session: &'session Session<'context, 'source>,
    stream: StandardStream,
    palette: Palette,
  ) -> Self {
    Self {
      session,
      stream,
      palette,
    }
  }
}

pub trait Dumpable {
  /// Recurse through the tree.
  /// 'prefix' is the string of vertical bars from parents.
  /// 'is_last' determines if we use an end marker or a middle marker (e.g., `└──` and `├──` in [`ASTDumper`]) for this node, and also affects how we build the prefix for children.
  ///
  /// Usually, the implementation should:
  /// 1. print the indent for **this** node. i.e., use [`Dumper::print_indent`] with the given `prefix` and `is_last`.
  /// 2. print the node header info like type name, address, span, etc. using [`Dumper::write_fmt`].
  /// 3. compute the prefix for children using [`Dumper::child_prefix`] and recurse into children with the new `prefix` and correct `is_last`.
  fn dump<'context, 'source>(
    &self,
    dumper: &mut impl Dumper<'context, 'source>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    'source: 'context;
}

impl Dumpable for SourceSpan {
  fn dump<'context, 'source>(
    &self,
    dumper: &mut impl Dumper<'context, 'source>,
    _prefix: &str,
    _is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    'source: 'context,
  {
    dumper.write("<", &palette.skeleton)?;
    let (l, c) = dumper
      .session()
      .manager
      .lookup_line_col(*self)
      .destructure();
    dumper.write_fmt(format_args!("{}:{}", l, c), &palette.dim)?;
    dumper.write("> ", &palette.skeleton)
  }
}
