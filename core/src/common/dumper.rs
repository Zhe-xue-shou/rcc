use ::termcolor::{
  BufferedStandardStream, Color, ColorChoice, ColorSpec, WriteColor,
};

use super::SourceSpan;
use crate::session::Session;

pub type FakeDumpRes = ();
type DumpRes = ::std::io::Result<()>;

use ::std::{
  io::Write,
  ops::{Deref, DerefMut},
};

pub struct StickyWriter<W: Write> {
  inner: W,
  error: DumpRes,
}
impl<W: Write> Deref for StickyWriter<W> {
  type Target = W;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
impl<W: Write> DerefMut for StickyWriter<W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}
impl<W: Write> StickyWriter<W> {
  fn new(inner: W) -> Self {
    Self {
      inner,
      error: Ok(()),
    }
  }

  fn write_fmt(&mut self, args: std::fmt::Arguments) {
    if self.error.is_ok()
      && let Err(e) = self.inner.write_fmt(args)
    {
      self.error = Err(e);
    }
  }

  fn finalize(self) -> DumpRes {
    self.error
  }
}
#[repr(transparent)]
#[derive(Debug)]
pub struct FlushOnDropRAII<W: Write> {
  inner: W,
}

impl<W: Write> FlushOnDropRAII<W> {
  pub fn new(inner: W) -> Self {
    Self { inner }
  }
}
impl<W: Write> Drop for FlushOnDropRAII<W> {
  fn drop(&mut self) {
    if let Err(e) = self.inner.flush()
      && const { cfg!(debug_assertions) }
    {
      eprintln!("\nWarning: stream flush failed: {e}");
    }
    // drop(self.0); // no need, rust drop is recursive
  }
}
impl<W: Write> Deref for FlushOnDropRAII<W> {
  type Target = W;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
impl<W: Write> DerefMut for FlushOnDropRAII<W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}
impl<W: Write> Write for FlushOnDropRAII<W> {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.inner.write(buf)
  }

  /// no effect.
  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}
impl<W: Write + WriteColor> WriteColor for FlushOnDropRAII<W> {
  fn supports_color(&self) -> bool {
    self.inner.supports_color()
  }

  fn set_color(&mut self, spec: &ColorSpec) -> std::io::Result<()> {
    self.inner.set_color(spec)
  }

  fn reset(&mut self) -> std::io::Result<()> {
    self.inner.reset()
  }
}
/// A palette of colors for different parts of the dump output.
/// The dumper will use the appropriate color from the palette when printing different parts of the output.
///  This allows for consistent and customizable coloring of the dump output across different dumpable types.
#[derive(Default, Clone)]
pub struct Palette {
  /// The color for node headers, e.g., "BinaryExpr", "ArrayType", etc.
  pub node: ColorSpec,
  /// The color for various operators, like "+", "*", "->".
  pub operator: ColorSpec,
  /// The color for literals and identifiers.
  pub literal: ColorSpec,
  /// The color for metadata like types, offsets, etc.
  pub meta: ColorSpec,
  /// The color for kind info, usually an internal string describing the specific kind of node, like "LValueConversion" in `ImplicitCastExpr`.
  pub kind: ColorSpec,
  /// The color for additional info, e.g., source location, internal memory addresses.
  pub info: ColorSpec,
  /// The color for dimmed text, less important info.
  pub dim: ColorSpec,
  /// The color for the skeleton of the tree, like vertical bars.
  pub skeleton: ColorSpec,
  /// For error nodes and error messages.
  pub error: ColorSpec,
}
::rcc_utils::ensure_is_pod!(Palette);
impl Palette {
  /// A default colorful palette. Output of AST dumps of this palette resembles `clang -Xclang -ast-dump`.
  pub fn colored() -> Self {
    Self {
      node: ColorSpec::new()
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

pub trait Dumper<'source, 'context, 'session>
where
  'source: 'context,
  'context: 'session,
{
  #[inline(always)]
  fn write(&mut self, text: &str, spec: &ColorSpec) -> FakeDumpRes {
    self.write_fmt(format_args!("{}", text), spec)
  }
  #[inline(always)]
  fn writeln(&mut self, text: &str, spec: &ColorSpec) -> FakeDumpRes {
    self.write_fmt(format_args!("{}\n", text), spec)
  }

  fn write_fmt(
    &mut self,
    args: ::std::fmt::Arguments<'_>,
    spec: &ColorSpec,
  ) -> FakeDumpRes;

  fn newline(&mut self) -> FakeDumpRes;

  fn print_indent(&mut self, prefix: &str, is_last: bool) -> FakeDumpRes;

  /// Build the new prefix for children based on whether the current node is the last child.
  #[must_use]
  fn child_prefix(&self, prefix: &str, is_last: bool) -> String;

  #[must_use]
  fn palette(&self) -> &Palette;

  fn finalize(self) -> DumpRes;
  #[must_use]
  fn session(&self) -> &'session Session<'source, 'context>;
}
pub struct Default<
  'source,
  'context,
  'session,
  const INDENT_BODY: &'static str = "    ",
  const INDENT_LAST: &'static str = "    ",
  const PARENT_BODY: &'static str = "    ",
  const PARENT_LAST: &'static str = "    ",
  const PREFIX_LEFT: &'static str = "",
> where
  'source: 'context,
  'context: 'session,
{
  pub stream: StickyWriter<FlushOnDropRAII<BufferedStandardStream>>,
  pub palette: Palette,
  pub session: &'session Session<'source, 'context>,
}
impl<
  'source,
  'context,
  'session,
  const INDENT_BODY: &'static str,
  const INDENT_LAST: &'static str,
  const PARENT_BODY: &'static str,
  const PARENT_LAST: &'static str,
  const PREFIX_LEFT: &'static str,
> Dumper<'source, 'context, 'session>
  for Default<
    'source,
    'context,
    'session,
    INDENT_BODY,
    INDENT_LAST,
    PARENT_BODY,
    PARENT_LAST,
    PREFIX_LEFT,
  >
where
  'source: 'context,
  'context: 'session,
{
  #[inline]
  fn write_fmt(
    &mut self,
    args: ::std::fmt::Arguments<'_>,
    spec: &ColorSpec,
  ) -> FakeDumpRes {
    let _ = self.stream.set_color(spec);
    self.stream.write_fmt(args)
  }

  #[inline(always)]
  fn newline(&mut self) -> FakeDumpRes {
    writeln!(self.stream)
  }

  fn print_indent(&mut self, prefix: &str, is_last: bool) -> FakeDumpRes {
    let _ = self.stream.set_color(&self.palette.skeleton);
    write!(
      self.stream,
      "{}{}",
      prefix,
      if is_last { INDENT_LAST } else { INDENT_BODY }
    )
  }

  #[inline(always)]
  fn palette(&self) -> &Palette {
    &self.palette
  }

  /// Build the new prefix for children based on whether the current node is the last child.
  #[inline]
  fn child_prefix(&self, prefix: &str, is_last: bool) -> String {
    format!(
      "{}{}",
      prefix,
      // parent was last → no vertical bar
      // parent was not last → vertical bar continues
      if is_last { PARENT_LAST } else { PARENT_BODY }
    )
  }

  #[inline(always)]
  fn finalize(self) -> DumpRes {
    let mut stream = self.stream;
    stream.reset()?;
    stream.finalize()
  }

  #[inline(always)]
  fn session(&self) -> &'session Session<'source, 'context> {
    self.session
  }
}
impl<
  'source,
  'context,
  'session,
  const INDENT_BODY: &'static str,
  const INDENT_LAST: &'static str,
  const PARENT_BODY: &'static str,
  const PARENT_LAST: &'static str,
  const PREFIX_LEFT: &'static str,
>
  Default<
    'source,
    'context,
    'session,
    INDENT_BODY,
    INDENT_LAST,
    PARENT_BODY,
    PARENT_LAST,
    PREFIX_LEFT,
  >
{
  #[inline(never)]
  pub fn dump(
    dumpable: &impl Dumpable,
    session: &'session Session<'source, 'context>,
  ) -> DumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    let mut dumper = Self::new(
      session,
      StickyWriter::new(FlushOnDropRAII::new(BufferedStandardStream::stdout(
        ColorChoice::Auto,
      ))),
      Palette::colored(),
    );
    let palette = dumper.palette().clone();
    dumpable.dump(&mut dumper, PREFIX_LEFT, true, &palette);
    dumper.finalize()
  }
}
impl<
  'source,
  'context,
  'session,
  const INDENT_BODY: &'static str,
  const INDENT_LAST: &'static str,
  const PARENT_BODY: &'static str,
  const PARENT_LAST: &'static str,
  const PREFIX_LEFT: &'static str,
>
  Default<
    'source,
    'context,
    'session,
    INDENT_BODY,
    INDENT_LAST,
    PARENT_BODY,
    PARENT_LAST,
    PREFIX_LEFT,
  >
{
  pub fn new(
    session: &'session Session<'source, 'context>,
    stream: StickyWriter<FlushOnDropRAII<BufferedStandardStream>>,
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
  /// - 'prefix' is the string of vertical bars from parents.
  /// - 'is_last' determines if we use an end marker or a middle marker
  ///   (e.g., `└──` and `├──` in [`crate::sema::ASTDumper`]) for this node, and also affects how we build the prefix for children.
  ///
  /// Usually, the implementation should:
  /// 1. print the indent for **this** node. i.e., use [`Dumper::print_indent`] with the given `prefix` and `is_last`.
  /// 2. print the node header info like type name, address, span, etc. using [`Dumper::write_fmt`].
  /// 3. compute the prefix for children using [`Dumper::child_prefix`] and recurse into children with the new `prefix` and correct `is_last`.
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session;
}

impl Dumpable for SourceSpan {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    _prefix: &str,
    _is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.write("<", &palette.skeleton);
    let (l, c) = dumper
      .session()
      .manager
      .lookup_line_col(*self)
      .destructure();
    dumper.write_fmt(format_args!("{}:{}", l, c), &palette.dim);
    dumper.write("> ", &palette.skeleton)
  }
}
