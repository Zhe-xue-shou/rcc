use ::rcc_shared::SourceManager;
use ::termcolor::{ColorChoice, ColorSpec};

use crate::Palette;
pub trait RenderEngineMixin<'c>: RenderEngine<'c> {
  #[inline(always)]
  fn write<T: ::std::fmt::Display>(&mut self, arg: T, spec: &ColorSpec) {
    self.write_fmt(format_args!("{}", arg), spec)
  }

  #[inline(always)]
  fn writeln<T: ::std::fmt::Display>(&mut self, arg: T, spec: &ColorSpec) {
    self.write_fmt(format_args!("{}\n", arg), spec)
  }

  #[inline(always)]
  #[allow(unused)]
  fn quoted<T: ::std::fmt::Display, const QUOTE: &'static str>(
    &mut self,
    arg: T,
    spec: &ColorSpec,
  ) {
    self.write_fmt(format_args!("{}{}{}", QUOTE, arg, QUOTE), spec)
  }

  #[inline(always)]
  #[allow(unused)]
  fn pre<T: ::std::fmt::Display, const PREFIX: &'static str>(
    &mut self,
    arg: T,
    spec: &ColorSpec,
  ) {
    self.write_fmt(format_args!("{}{}", PREFIX, arg), spec)
  }

  #[inline(always)]
  #[allow(unused)]
  fn suf<T: ::std::fmt::Display, const SUFFIX: &'static str>(
    &mut self,
    arg: T,
    spec: &ColorSpec,
  ) {
    self.write_fmt(format_args!("{}{}", arg, SUFFIX), spec)
  }
}
impl<'c, T> RenderEngineMixin<'c> for T where T: RenderEngine<'c> {}
/// Shared writer contract for AST/IR tree renderers.
pub trait RenderEngine<'c> {
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
  fn auto_color() -> ColorChoice
  where
    Self: Sized, // for `dyn`.
  {
    use ::std::io::{IsTerminal, stderr, stdout};

    if stdout().is_terminal() && stderr().is_terminal() {
      ColorChoice::Auto
    } else {
      ColorChoice::Never
    }
  }
}
