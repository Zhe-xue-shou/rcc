use ::rcc_ir::{Context, Session, ValueID};
use ::rcc_shared::{Diagnosis, SourceManager};
use ::slotmap::SecondaryMap;
use ::std::cell::RefCell;
use ::termcolor::{BufferedStandardStream, ColorSpec};

use crate::{
  FlushOnDropRAII, Palette, RenderEngine, StickyWriter, TreeDumper, pre,
};

pub trait Printable<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  );
}

type Inner = TreeDumper;
pub trait Printer<'c>: RenderEngine<'c> {
  #[must_use]
  fn ir(&self) -> &'c Context<'c>;
  #[must_use]
  fn get_id(&self, value_id: ValueID) -> usize;
  /// Temporary workaround. usually [`Self::get_id`] is sufficient.
  ///
  /// Only use when the `value_id` is probably a global constant.   
  fn write_id(&mut self, value_id: ValueID);
  fn reset_counter(&self);
}

pub struct IRPrinter<'c> {
  inner: Inner,
  context: &'c Context<'c>,
  manager: &'c SourceManager,
  counter: RefCell<SecondaryMap<ValueID, usize>>,
}
impl<'c> RenderEngine<'c> for IRPrinter<'c> {
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

  fn src(&self) -> &'c SourceManager {
    self.manager
  }

  fn finalize(self) -> ::std::io::Result<()> {
    self.inner.finalize()
  }
}

impl<'c> Printer<'c> for IRPrinter<'c> {
  fn ir(&self) -> &'c Context<'c> {
    self.context
  }

  fn get_id(&self, value_id: ValueID) -> usize {
    if let Some(&id) = self.counter.borrow().get(value_id) {
      id
    } else {
      let new_id = self.counter.borrow().len();
      self.counter.borrow_mut().insert(value_id, new_id);
      new_id
    }
  }

  fn write_id(&mut self, value_id: ValueID) {
    let skeleton = self.palette().skeleton.clone();
    if let Some(name) = self.ir().visit(value_id, |value| {
      value
        .data
        .as_constant()
        .and_then(|c| c.as_global().map(|g| g.name()))
    }) {
      self.write_fmt(pre!("@" => name), &skeleton)
    } else {
      self.write_fmt(pre!("%" => self.get_id(value_id)), &skeleton)
    }
  }

  fn reset_counter(&self) {
    self.counter.borrow_mut().clear();
  }
}

impl<'c> IRPrinter<'c> {
  pub fn new(
    context: &'c Context<'c>,
    manager: &'c SourceManager,
    stream: StickyWriter<FlushOnDropRAII<BufferedStandardStream>>,
    palette: Palette,
  ) -> Self {
    Self {
      inner: Inner::new(stream, palette),
      context,
      manager,
      counter: Default::default(),
    }
  }

  #[inline(never)]
  pub fn print<D: Diagnosis<'c>>(
    printable: &'c impl Printable<'c>,
    session: &'c Session<'c, D>,
  ) -> ::std::io::Result<()> {
    let mut printer = Self::new(
      session.ir(),
      session.src(),
      StickyWriter::new(FlushOnDropRAII::new(BufferedStandardStream::stdout(
        Self::auto_color(),
      ))),
      Palette::colored(),
    );
    let palette = printer.palette().clone();
    printable.print(&mut printer, "", true, &palette);
    printer.finalize()
  }
}
