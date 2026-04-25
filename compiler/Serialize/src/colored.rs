use ::std::{
  io::Write,
  ops::{Deref, DerefMut},
};
use ::termcolor::{Color, ColorSpec, WriteColor};

/// I'm being a bit of lazy here, ususally we use [`?`](std::ops::Try)
/// to propagate errors of [`std::io::Result`],
/// but in this case we want to keep writing even if some writes fail,
/// and report the first error at the end.
pub struct StickyWriter<W: Write> {
  inner: W,
  error: ::std::io::Result<()>,
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
  pub fn new(inner: W) -> Self {
    Self {
      inner,
      error: Ok(()),
    }
  }

  pub fn write_fmt(&mut self, args: std::fmt::Arguments) {
    if self.error.is_ok()
      && let Err(e) = self.inner.write_fmt(args)
    {
      self.error = Err(e);
    }
  }

  pub fn finalize(self) -> ::std::io::Result<()> {
    self.error
  }
}
/// A wrapper around a [`Write`] that flushes the stream when dropped.
/// This is useful for ensuring that all output is flushed even if the dumper panics or returns early.
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
    // println!(); /// < IO desync: this would cause issue. you can check it... it caught me for hours!
    if let Err(e) = self.inner.flush()
      && const { cfg!(debug_assertions) }
    {
      eprintln!("\nWarning: stream flush failed: {e}");
    }
    println!()
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
///
/// The dumper will use the appropriate color from the palette when printing different parts of the output.
/// This allows for consistent and customizable coloring of the dump output across different dumpable types.
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
  ///
  /// This should always left as default(no color, no bold, no intense).
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

  /// A dimmed palette. Useful for less important info.
  pub fn dimmed() -> Self {
    let dimmed = ColorSpec::new()
      .set_fg(Some(Color::White))
      .set_intense(false)
      .set_bold(false)
      .set_italic(true)
      .to_owned();
    Self {
      node: dimmed.clone(),
      operator: dimmed.clone(),
      literal: dimmed.clone(),
      meta: dimmed.clone(),
      kind: dimmed.clone(),
      info: dimmed.clone(),
      dim: dimmed.clone(),
      skeleton: dimmed.clone(),
      error: ColorSpec::new()
        .set_fg(Some(Color::Red))
        .set_bold(true)
        .to_owned(),
    }
  }
}
