use super::{
  Array, Enum, FunctionProto, Pointer, Primitive, QualifiedType, Record, Type,
  Union,
};
use crate::common::{DumpRes, Dumpable, Dumper, Palette};

impl Dumpable for QualifiedType {
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("QualifiedType", &palette.node_type)?;
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;

    dumper.write_fmt(
      format_args!("{} {}\n", self.unqualified_type(), self.qualifiers()),
      &palette.meta,
    )?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self
      .unqualified_type()
      .dump(dumper, &subprefix, true, palette)
  }
}

impl Dumpable for Type {
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    ::rcc_utils::static_dispatch!(
      self.dump(dumper, prefix, is_last, palette),
      Primitive Pointer Array FunctionProto Union Enum Record
    )
  }
}
impl Dumpable for Primitive {
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("Primitive", &palette.node_type)?;
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta)
  }
}

impl Dumpable for Pointer {
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("Pointer", &palette.node_type)?;
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta)?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.pointee.dump(dumper, &subprefix, true, palette)
  }
}
impl Dumpable for Array {
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("Array", &palette.node_type)?;
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
    dumper.write_fmt(
      format_args!("{}, {} elements\n", self.element_type, self.size),
      &palette.meta,
    )?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.element_type.dump(dumper, &subprefix, true, palette)
  }
}

impl Dumpable for FunctionProto {
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("FunctionProto", &palette.node_type)?;
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
    dumper.write_fmt(format_args!("{}\n", self), &palette.meta)
  }
}
impl Dumpable for Enum {
  #[allow(unused)]
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    todo!()
  }
}

impl Dumpable for Record {
  #[allow(unused)]
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    todo!()
  }
}
impl Dumpable for Union {
  #[allow(unused)]
  fn dump(
    &self,
    dumper: &mut impl Dumper,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes {
    todo!()
  }
}
