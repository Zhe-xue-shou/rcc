#![allow(unused)]

use ::rcc_utils::DisplayWith;

use super::{
  Context, Module, Value, ValueData, ValueID, instruction as inst, module,
};
use crate::{
  common::{Dumpable, Dumper, FakeDumpRes, Palette, TreeDumper},
  ir,
  types::Constant,
};
// no tree structure for IR
pub type IRDumper<'c> = TreeDumper<
  'c,
  /* "    ", */
  /* "    ", */
  /* "    ", */
  /* "    ", */
  /* ""    , */
>;

macro_rules! id {
  ($id:expr, $dumper:expr, $palette:expr) => {
    if let Some(value) = $dumper.session().ir_context.get_by_constant_id(&$id) {
      $dumper.write_fmt(format_args!("{}", value), &$palette.literal);
    } else {
      $dumper.write_fmt(format_args!("%{}", $id.handle()), &$palette.skeleton);
    }
  };
}

impl<'c> Dumpable<'c> for Module {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    self.globals.iter().for_each(|value_id| {
      Dumpable::dump(
        &*ctx!(dumper).get(*value_id),
        dumper,
        prefix,
        is_last,
        palette,
      )
    })
  }
}

impl<'c> Dumpable<'c> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> FakeDumpRes {
    ::rcc_utils::static_dispatch!(
        ValueData: &self.data,
        |variant| Dump::dump(self, dumper, prefix, is_last, palette, variant) =>
        Instruction Constant Function Variable BasicBlock Argument
    )
  }
}
trait Dump<'c, DataTy> {
  /// This is a special version of [`Dumpable::dump`] for dumping a specific variant of [`ValueData`].
  ///
  /// Please refer to the doc of [`Dumpable::dump`] for the meaning of the parameters.
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &DataTy,
  ) -> FakeDumpRes;
}
/// Useless stuffs toi bypass type checker for now.
#[allow(unused)]
macro_rules! please_dump_me {
  ($DataTy:ty) => {
    #[allow(unused)]
    impl<'c> Dump<'c, $DataTy> for Value<'c> {
      fn dump(
        &self,
        dumper: &mut impl Dumper<'c>,
        prefix: &str,
        is_last: bool,
        palette: &Palette,
        variant: &$DataTy,
      ) -> FakeDumpRes {
        todo!()
      }
    }
  };
}
impl<'c> Dump<'c, inst::Instruction> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Instruction,
  ) -> FakeDumpRes {
    // my static_dispatch uses `ident` instead of
    // `type` of the 1st arg(qual path is unstable and rust-analyzer is having a hard time to hightlighing that).
    // hence strip the `::` path here.
    use inst::Instruction;
    ::rcc_utils::static_dispatch!(
        Instruction : variant,
        |variant| Dump::dump(self, dumper, prefix, is_last, palette, variant) =>
        Phi Terminator Unary Binary Memory Cast Call ICmp
    )
  }
}

impl<'c> Dump<'c, Constant<'_>> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &Constant<'_>,
  ) -> FakeDumpRes {
    dumper.write_fmt(suff!(" " => self.ir_type), &palette.meta);
    dumper.write(variant, &palette.literal);
  }
}
impl<'c> Dump<'c, module::Function<'_>> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::Function<'_>,
  ) -> FakeDumpRes {
    dumper.write_fmt(
      format_args!(
        "{} ",
        if variant.is_definition() {
          debug_assert!(
            variant.params.len()
              == self.ir_type.as_function_unchecked().params.len()
          );
          "define"
        } else {
          debug_assert!(
            variant.params.is_empty(),
            "my design ensures function decl has correct ir type, but the \
             argid is not stored."
          );
          "declare"
        }
      ),
      &palette.meta,
    );

    dumper.write_fmt(
      suff!(" " => self.ir_type.as_function_unchecked().return_type),
      &palette.meta,
    );

    dumper.write_fmt(pre!("@" => variant.name), &palette.dim);
    dumper.write("(", &palette.skeleton);
    if variant.is_definition() {
      variant
        .params
        .iter()
        .enumerate()
        .for_each(|(index, arg_id)| {
          let arg = &*ctx!(dumper).get(*arg_id);
          Dump::dump(
            arg,
            dumper,
            /* index */ &format!("{}", arg_id.handle()),
            index == variant.params.len() - 1,
            palette,
            arg.data.as_argument_unchecked(),
          );
        });
    } else {
      self
        .ir_type
        .as_function_unchecked()
        .params
        .iter()
        .enumerate()
        .for_each(|(index, param_ty)| {
          dumper.write_fmt(suff!(" " => param_ty), &palette.meta);
          dumper.write_fmt(
            format_args!(
              "{}",
              if variant.params.is_empty() || index + 1 == variant.params.len()
              {
                ""
              } else {
                ", "
              }
            ),
            &palette.dim,
          );
        });
    }
    dumper.write(")", &palette.skeleton);
    if variant.is_definition() {
      dumper.writeln(" {", &palette.skeleton);
      variant.blocks.iter().for_each(|block_id| {
        dumper
          .write_fmt(format_args!("{}:\n", block_id.handle()), &palette.dim);
        let block = &*ctx!(dumper).get(*block_id);
        Dump::dump(
          block,
          dumper,
          "\t",
          false,
          palette,
          block.data.as_basicblock_unchecked(),
        );
      });
      dumper.write("\n}", &palette.skeleton);
    }
    dumper.newline();
  }
}
impl<'c> Dump<'c, module::Variable<'_>> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::Variable<'_>,
  ) -> FakeDumpRes {
    todo!()
  }
}
impl<'c> Dump<'c, module::BasicBlock> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::BasicBlock,
  ) -> FakeDumpRes {
    variant.instructions.iter().for_each(|inst_id| {
      dumper.write(prefix, &palette.dim);
      let value = ctx!(dumper).get(*inst_id);
      match &value.data {
        ValueData::Instruction(inst::Instruction::Memory(
          inst::Memory::Store(s),
        )) => Dump::dump(&*value, dumper, "", is_last, palette, s),
        _ => {
          dumper.write_fmt(
            format_args!("%{}", inst_id.handle()),
            &palette.skeleton,
          );
          dumper.write(" = ", &palette.skeleton);
          Dumpable::dump(&*value, dumper, "", is_last, palette);
        },
      }
      dumper.newline();
    });
    let terminator = &*ctx!(dumper).get(variant.terminator);
    dumper.write(prefix, &palette.dim);
    Dump::dump(
      terminator,
      dumper,
      "",
      true,
      palette,
      terminator
        .data
        .as_instruction_unchecked()
        .as_terminator_unchecked(),
    );
  }
}
impl<'c> Dump<'c, module::Argument> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    index: &str, // coontext is actually an index
    is_last: bool,
    palette: &Palette,
    variant: &module::Argument,
  ) -> FakeDumpRes {
    dumper.write_fmt(suff!(" " => self.ir_type), &palette.meta);
    dumper.write_fmt(pre!("%" => index), &palette.skeleton);
    dumper.write((if is_last { "" } else { ", " }), &palette.dim);
  }
}

please_dump_me!(inst::Phi);
impl<'c> Dump<'c, inst::Terminator> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Terminator,
  ) -> FakeDumpRes {
    use inst::Terminator;
    ::rcc_utils::static_dispatch!(
        Terminator : variant,
        |variant| Dump::dump(self, dumper, prefix, is_last, palette, variant) =>
        Jump Branch Return
    )
  }
}
please_dump_me!(inst::Unary);
impl<'c> Dump<'c, inst::Binary> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Binary,
  ) -> FakeDumpRes {
    dumper.write(suff!(" " => variant.operator), &palette.literal);
    dumper.write(suff!(" " => self.ir_type), &palette.meta);
    id!(variant.lhs, dumper, palette);
    dumper.write(", ", &palette.skeleton);
    id!(variant.rhs, dumper, palette);
  }
}
impl<'c> Dump<'c, inst::Memory> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Memory,
  ) -> FakeDumpRes {
    use inst::Memory;
    ::rcc_utils::static_dispatch!(
        Memory : variant,
        |variant| Dump::dump(self, dumper, prefix, is_last, palette, variant) =>
        Alloca Load Store
    )
  }
}
impl<'c> Dump<'c, inst::Cast> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Cast,
  ) -> FakeDumpRes {
    use inst::Cast;
    ::rcc_utils::static_dispatch!(
        Cast : variant,
        |variant| Dump::dump(self, dumper, prefix, is_last, palette, variant) =>
        Zext Sext Trunc
    )
  }
}
impl<'c> Dump<'c, inst::Call> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Call,
  ) -> FakeDumpRes {
    dumper.write("call ", &palette.literal);
    match &ctx!(dumper).get(variant.callee).data {
      ValueData::Instruction(instruction) => todo!(),
      ValueData::Constant(raw_constant) => todo!(),
      ValueData::Variable(variable) => todo!(),
      ValueData::Argument(argument) => todo!(),
      ValueData::Function(function) => {
        dumper.write_fmt(suff!(" " => self.ir_type), &palette.meta);
        dumper.write_fmt(quoted!(" @", function.name, "("), &palette.dim);
        variant.args.iter().enumerate().for_each(|(index, arg_id)| {
          let arg = &*ctx!(dumper).get(*arg_id);
          dumper.write_fmt(suff!(" " => arg.ir_type), &palette.meta);
          dumper.write_fmt(
            format_args!(
              "{}",
              arg.data.as_constant().map_or_else(
                || format!("%{}", arg_id.handle()),
                |constant| format!("{}", constant)
              ),
            ),
            &palette.skeleton,
          );
          dumper.write(", ", &palette.skeleton);
        });
        dumper.write(")", &palette.dim);
      },
      ValueData::BasicBlock(basic_block) => unreachable!(),
    }
  }
}
impl<'c> Dump<'c, inst::ICmp> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::ICmp,
  ) -> FakeDumpRes {
    dumper.write("icmp ", &palette.literal);
    dumper.write(suff!(" " => variant.predicate), &palette.literal);
    dumper.write(suff!(" " => self.ir_type), &palette.meta);
    id!(variant.lhs, dumper, palette);
    dumper.write(", ", &palette.skeleton);
    id!(variant.rhs, dumper, palette);
  }
}

please_dump_me!(inst::Jump);
please_dump_me!(inst::Branch);
impl<'c> Dump<'c, inst::Return> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    _is_last: bool,
    palette: &Palette,
    variant: &inst::Return,
  ) -> FakeDumpRes {
    debug_assert!(_is_last);
    dumper.write("ret ", &palette.literal);
    dumper.write_fmt(suff!(" " => self.ir_type), &palette.meta);
    if let Some(value_id) = variant.result {
      id!(value_id, dumper, palette);
    }
  }
}

impl<'c> Dump<'c, inst::Alloca> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Alloca,
  ) -> FakeDumpRes {
    dumper.write("alloca ", &palette.literal);
    dumper.write(
      dumper.session().ir_context.ir_type(&self.qualified_type),
      &palette.meta,
    )
  }
}
impl<'c> Dump<'c, inst::Load> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Load,
  ) -> FakeDumpRes {
    dumper.write("load ", &palette.literal);
    dumper.write((self.ir_type), &palette.meta);
    dumper.write(", ", &palette.skeleton);

    debug_assert!(ctx!(dumper).get(variant.addr).ir_type.is_pointer());

    dumper.write("ptr ", &palette.meta);
    dumper.write_fmt(pre!("%" => variant.addr.handle()), &palette.skeleton);
  }
}
impl<'c> Dump<'c, inst::Store> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    _is_last: bool,
    palette: &Palette,
    variant: &inst::Store,
  ) -> FakeDumpRes {
    dumper.write(prefix, &palette.dim);
    dumper.write("store ", &palette.literal);
    dumper.write(suff!(" " => self.ir_type), &palette.meta);

    id!(variant.value, dumper, palette);

    dumper.write(", ", &palette.skeleton);

    debug_assert!(ctx!(dumper).get(variant.addr).ir_type.is_pointer());

    dumper.write("ptr ", &palette.meta);
    dumper.write_fmt(pre!("%" => variant.addr.handle()), &palette.skeleton);
  }
}

impl<'c> Dump<'c, inst::Zext> for Value<'c> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Zext,
  ) -> FakeDumpRes {
    dumper.write("zext ", &palette.literal);

    dumper.write(
      suff!(" " => ctx!(dumper).get(variant.operand).ir_type),
      &palette.meta,
    );
    dumper.write(variant.operand.handle(), &palette.skeleton);

    dumper.write(" to ", &palette.skeleton);
    dumper.write(self.ir_type, &palette.meta);
  }
}
please_dump_me!(inst::Sext);
please_dump_me!(inst::Trunc);
