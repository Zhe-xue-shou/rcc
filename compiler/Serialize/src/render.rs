use ::rcc_shared::SourceManager;
use ::termcolor::{ColorChoice, ColorSpec};

use crate::Palette;

/// Shared writer contract for AST/IR tree renderers.
pub trait RenderEngine<'c> {
  #[inline(always)]
  fn write<T: ::std::fmt::Display>(&mut self, arg: T, spec: &ColorSpec) {
    self.write_fmt(format_args!("{}", arg), spec)
  }

  #[inline(always)]
  fn writeln<T: ::std::fmt::Display>(&mut self, arg: T, spec: &ColorSpec) {
    self.write_fmt(format_args!("{}\n", arg), spec)
  }

  fn write_fmt(&mut self, args: ::std::fmt::Arguments<'_>, spec: &ColorSpec);

  #[inline(always)]
  fn writeln_fmt(&mut self, args: ::std::fmt::Arguments<'_>, spec: &ColorSpec) {
    self.write_fmt(format_args!("{}\n", args), spec)
  }

  fn newline(&mut self);

  fn print_indent(&mut self, prefix: &str, is_last: bool);

  /// Build the new prefix for children based on whether the current node is the last child.
  #[must_use]
  fn child_prefix(&self, prefix: &str, is_last: bool) -> String;

  #[must_use]
  fn palette(&self) -> &Palette;

  #[must_use]
  fn src(&self) -> &'c SourceManager;

  fn finalize(self) -> ::std::io::Result<()>;

  #[must_use]
  #[inline]
  fn auto_color() -> ColorChoice {
    use ::std::io::{IsTerminal, stdout};

    if stdout().is_terminal() {
      ColorChoice::Auto
    } else {
      ColorChoice::Never
    }
  }
}
