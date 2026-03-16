use super::{
  Array, Enum, FunctionProto, Pointer, Primitive, QualifiedType, Record, Type,
  Union,
};
use crate::common::{FakeDumpRes, Dumpable, Dumper, Palette};

impl Dumpable for QualifiedType<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
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

impl Dumpable for Type<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.dump(dumper, prefix, is_last, palette) =>
      Primitive Pointer Array FunctionProto Union Enum Record
    )
  }
}
impl Dumpable for Primitive {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.print_indent(prefix, is_last);
    dumper.write("Primitive", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta)
  }
}

impl Dumpable for Pointer<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.print_indent(prefix, is_last);
    dumper.write("Pointer", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta);

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.pointee.dump(dumper, &subprefix, true, palette)
  }
}
impl Dumpable for Array<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
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

impl Dumpable for FunctionProto<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.print_indent(prefix, is_last);
    dumper.write("FunctionProto", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta)
  }
}
#[allow(unused)]
impl Dumpable for Enum<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    todo!()
  }
}

#[allow(unused)]
impl Dumpable for Record<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    todo!()
  }
}
impl Dumpable for Union<'_> {
  #[allow(unused)]
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    todo!()
  }
}
