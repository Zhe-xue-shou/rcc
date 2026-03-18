use super::{
  Array, Enum, FunctionProto, Pointer, Primitive, QualifiedType, Record, Type,
  Union,
};
use crate::common::{Dumpable, Dumper, FakeDumpRes, Palette};

impl<'c> Dumpable<'c> for QualifiedType<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    dumper.print_indent(prefix, is_last);
    dumper.write("QualifiedType", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);

    dumper.write_fmt(
      format_args!("{} {}\n", self.unqualified_type, self.qualifiers),
      &palette.meta,
    );

    let subprefix = dumper.child_prefix(prefix, is_last);
    self
      .unqualified_type
      .dump(dumper, &subprefix, true, palette)
  }
}

impl<'c> Dumpable<'c> for Type<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.dump(dumper, prefix, is_last, palette) =>
      Primitive Pointer Array FunctionProto Union Enum Record
    )
  }
}
impl<'c> Dumpable<'c> for Primitive {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    dumper.print_indent(prefix, is_last);
    dumper.write("Primitive", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta)
  }
}

impl<'c> Dumpable<'c> for Pointer<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    dumper.print_indent(prefix, is_last);
    dumper.write("Pointer", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta);

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.pointee.dump(dumper, &subprefix, true, palette)
  }
}
impl<'c> Dumpable<'c> for Array<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    dumper.print_indent(prefix, is_last);
    dumper.write("Array", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    dumper.write_fmt(
      format_args!("{}, {} elements\n", self.element_type, self.size),
      &palette.meta,
    );

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.element_type.dump(dumper, &subprefix, true, palette)
  }
}

impl<'c> Dumpable<'c> for FunctionProto<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    dumper.print_indent(prefix, is_last);
    dumper.write("FunctionProto", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta)
  }
}
#[allow(unused)]
impl<'c> Dumpable<'c> for Enum<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    todo!()
  }
}

#[allow(unused)]
impl<'c> Dumpable<'c> for Record<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    todo!()
  }
}
impl<'c> Dumpable<'c> for Union<'_> {
  #[allow(unused)]
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    todo!()
  }
}
