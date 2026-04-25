use ::rcc_ast::{Context, Session};
use ::rcc_shared::{Diagnosis, SourceManager, SourceSpan};
use ::termcolor::{BufferedStandardStream, ColorSpec};

use crate::{
  FlushOnDropRAII, Palette, RenderEngine, StickyWriter, TreeDumper,
  render::RenderEngineMixin,
};

pub trait Dumper<'c>: RenderEngine<'c> {
  #[must_use]
  fn ast(&self) -> &'c Context<'c>;
}

type Inner = TreeDumper<"├── ", "└── ", "│   ", "    ", "">;
pub struct ASTDumper<'c> {
  inner: Inner,

  context: &'c Context<'c>,
  manager: &'c SourceManager,
}

impl<'c> RenderEngine<'c> for ASTDumper<'c> {
  fn write_fmt(&mut self, args: ::std::fmt::Arguments<'_>, spec: &ColorSpec) {
    self.inner.write_fmt(args, spec);
  }

  fn newline(&mut self) {
    self.inner.newline();
  }

  fn print_indent(&mut self, prefix: &str, is_last: bool) {
    self.inner.print_indent(prefix, is_last);
  }

  fn child_prefix(&self, prefix: &str, is_last: bool) -> String {
    self.inner.child_prefix(prefix, is_last)
  }

  fn palette(&self) -> &Palette {
    self.inner.palette()
  }

  fn finalize(self) -> ::std::io::Result<()> {
    self.inner.finalize()
  }

  fn src(&self) -> &'c SourceManager {
    self.manager
  }
}

impl<'c> Dumper<'c> for ASTDumper<'c> {
  fn ast(&self) -> &'c Context<'c> {
    self.context
  }
}

impl<'c> ASTDumper<'c> {
  pub fn new(
    context: &'c Context<'c>,
    manager: &'c SourceManager,
    stream: StickyWriter<FlushOnDropRAII<BufferedStandardStream>>,
    palette: Palette,
  ) -> Self {
    Self {
      context,
      manager,
      inner: Inner::new(stream, palette),
    }
  }
}
impl<'c> ASTDumper<'c> {
  #[inline(never)]
  pub fn dump<D: Diagnosis<'c>>(
    dumpable: &impl Dumpable<'c>,
    session: &'c Session<'c, D>,
  ) -> ::std::io::Result<()> {
    let mut dumper = Self::new(
      session.ast(),
      session.src(),
      StickyWriter::new(FlushOnDropRAII::new(BufferedStandardStream::stdout(
        Self::auto_color(),
      ))),
      Palette::colored(),
    );
    let palette = dumper.palette().clone();
    dumpable.dump(&mut dumper, "", true, &palette);
    dumper.finalize()
  }
}

pub trait Dumpable<'c> {
  /// Recurse through the tree.
  /// - 'prefix' is the string of vertical bars from parents.
  /// - 'is_last' determines if we use an end marker or a middle marker
  ///   (e.g., `└──` and `├──` in [`crate::sema::ASTDumper`]) for this node, and also affects how we build the prefix for children.
  ///
  /// Usually, the implementation should:
  /// 1. print the indent for **this** node. i.e., use [`Dumper::print_indent`] with the given `prefix` and `is_last`.
  /// 2. print the node header info like type name, address, span, etc. using [`Dumper::write_fmt`].
  /// 3. compute the prefix for children using [`Dumper::child_prefix`] and recurse into children with the new `prefix` and correct `is_last`.
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  );
}

mod private {

  use super::*;

  pub trait Sealed {}
  impl Sealed for SourceSpan {}
}
pub trait DumpSpan: private::Sealed {
  fn dump<'c>(
    &self,
    dumper: &mut impl Dumper<'c>,
    _prefix: &str,
    _is_last: bool,
    palette: &Palette,
  );
}
impl DumpSpan for SourceSpan {
  fn dump<'c>(
    &self,
    dumper: &mut impl Dumper<'c>,
    _prefix: &str,
    _is_last: bool,
    palette: &Palette,
  ) {
    dumper.write("<", &palette.skeleton);
    let (l, c) = dumper.src().lookup_line_col(*self).destructure();
    dumper.write_fmt(format_args!("{}:{}", l, c), &palette.dim);
    dumper.write("> ", &palette.skeleton)
  }
}
