use super::{
  Constant, Context, Module, Value, ValueData, instruction as inst, module,
};
use crate::{
  common::{Dumpable, Dumper, FakeDumpRes, Palette, TreeDumper},
  ir,
};
// no tree structure for IR
pub type IRDumper<'source, 'context, 'session> = TreeDumper<
  'source,
  'context,
  'session,
  /* "    ", */
  /* "    ", */
  /* "    ", */
  /* "    ", */
  /* ""    , */
>;

macro_rules! ctx {
  ($this:expr) => {
    $this.session().ir_context
  };
}

impl Dumpable for Module {
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

impl Dumpable for Value<'_> {
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
        ValueData: &self.data,
        |variant| Dump::dump(self, dumper, prefix, is_last, palette, variant) =>
        Instruction Constant Function Variable BasicBlock Argument
    )
  }
}
trait Dump<DataTy> {
  /// This is a special version of [`Dumpable::dump`] for dumping a specific variant of [`ValueData`].
  ///
  /// Please refer to the doc of [`Dumpable::dump`] for the meaning of the parameters.
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &DataTy,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session;
}
/// Useless stuffs toi bypass type checker for now.
#[allow(unused)]
macro_rules! please_dump_me {
  ($DataTy:ty) => {
    impl Dump<$DataTy> for Value<'_> {
      fn dump<'source, 'context, 'session>(
        &self,
        dumper: &mut impl Dumper<'source, 'context, 'session>,
        prefix: &str,
        is_last: bool,
        palette: &Palette,
        variant: &$DataTy,
      ) -> FakeDumpRes
      where
        'source: 'context,
        'context: 'session,
      {
        todo!()
      }
    }
  };
}
impl Dump<inst::Instruction> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Instruction,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
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

impl Dump<Constant<'_>> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &Constant<'_>,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.write_fmt(format_args!("{} ", self.ir_type), &palette.meta);
    dumper.write_fmt(format_args!("{}", variant), &palette.literal);
  }
}
impl Dump<module::Function<'_>> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::Function<'_>,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.write_fmt(
      format_args!(
        "{} ",
        if variant.is_definition() {
          "define"
        } else {
          "declare"
        }
      ),
      &palette.meta,
    );

    dumper.write_fmt(
      format_args!("{} ", self.ir_type.as_function_unchecked().return_type),
      &palette.meta,
    );

    dumper.write_fmt(format_args!("@{}(", variant.name,), &palette.dim);
    variant
      .params
      .iter()
      .enumerate()
      .for_each(|(index, arg_id)| {
        let arg = &*ctx!(dumper).get(*arg_id);
        Dump::dump(
          arg,
          dumper,
          /* index */ &format!("{}", arg_id),
          index == variant.params.len() - 1,
          palette,
          arg.data.as_argument_unchecked(),
        );
      });
    dumper.write(")", &palette.dim);
    if variant.is_definition() {
      dumper.writeln(" {", &palette.meta);
      variant.blocks.iter().for_each(|block_id| {
        dumper.write_fmt(format_args!("{}:\n", block_id), &palette.dim);
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
      dumper.write("}", &palette.meta);
    }
    dumper.newline();
  }
}
impl Dump<module::Variable<'_>> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::Variable<'_>,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    todo!()
  }
}
impl Dump<module::BasicBlock> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::BasicBlock,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    variant.instructions.iter().for_each(|inst_id| {
      dumper.write(prefix, &palette.dim);
      dumper.write_fmt(format_args!("%{} = ", inst_id), &palette.dim);
      Dumpable::dump(
        &*ctx!(dumper).get(*inst_id),
        dumper,
        "",
        is_last,
        palette,
      );
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
impl Dump<module::Argument> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    index: &str, // this is actually an index
    is_last: bool,
    palette: &Palette,
    variant: &module::Argument,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.write_fmt(format_args!("{} ", self.ir_type), &palette.meta);
    dumper.write_fmt(format_args!("%{}", index), &palette.dim);
    dumper.write_fmt(
      format_args!("{}", if is_last { "" } else { ", " }),
      &palette.dim,
    );
  }
}

please_dump_me!(inst::Phi);
impl Dump<inst::Terminator> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Terminator,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    use inst::Terminator;
    ::rcc_utils::static_dispatch!(
        Terminator : variant,
        |variant| Dump::dump(self, dumper, prefix, is_last, palette, variant) =>
        Jump Branch Return
    )
  }
}
please_dump_me!(inst::Unary);
please_dump_me!(inst::Binary);
impl Dump<inst::Memory> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Memory,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    use inst::Memory;
    ::rcc_utils::static_dispatch!(
        Memory : variant,
        |variant| Dump::dump(self, dumper, prefix, is_last, palette, variant) =>
        Alloca Load Store
    )
  }
}
please_dump_me!(inst::Cast);
impl Dump<inst::Call> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Call,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.write("call ", &palette.literal);
    match &ctx!(dumper).get(variant.callee).data {
      ValueData::Instruction(instruction) => todo!(),
      ValueData::Constant(raw_constant) => todo!(),
      ValueData::Variable(variable) => todo!(),
      ValueData::Argument(argument) => todo!(),
      ValueData::Function(function) => {
        dumper.write_fmt(format_args!("{} ", self.ir_type), &palette.meta);
        dumper.write_fmt(format_args!("@{}(", function.name,), &palette.dim);
        variant.args.iter().enumerate().for_each(|(index, arg_id)| {
          let arg = &*ctx!(dumper).get(*arg_id);
          dumper.write_fmt(format_args!("{} ", arg.ir_type), &palette.meta);
          dumper.write_fmt(
            format_args!(
              "{}{}",
              arg.data.as_constant().map_or_else(
                || format!("%{}", arg_id),
                |constant| format!("{}", constant)
              ),
              if index == variant.args.len() - 1 {
                ""
              } else {
                ", "
              }
            ),
            &palette.dim,
          );
        });
        dumper.write(")", &palette.dim);
      },
      ValueData::BasicBlock(basic_block) => unreachable!(),
    }
  }
}
please_dump_me!(inst::ICmp);

please_dump_me!(inst::Jump);
please_dump_me!(inst::Branch);
impl Dump<inst::Return> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    _is_last: bool,
    palette: &Palette,
    variant: &inst::Return,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    debug_assert!(_is_last);
    dumper.write("ret ", &palette.literal);
    dumper.write_fmt(format_args!("{} ", self.ir_type), &palette.meta);
    if let Some(value_id) = variant.result {
      dumper.write_fmt(format_args!("%{}\n", value_id), &palette.dim);
    }
  }
}

impl Dump<inst::Alloca> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Alloca,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.write("alloca ", &palette.literal);
    dumper.write_fmt(format_args!("{}", self.ir_type), &palette.meta)
  }
}
impl Dump<inst::Load> for Value<'_> {
  fn dump<'source, 'context, 'session>(
    &self,
    dumper: &mut impl Dumper<'source, 'context, 'session>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Load,
  ) -> FakeDumpRes
  where
    'source: 'context,
    'context: 'session,
  {
    dumper.write("load ", &palette.literal);
    dumper.write_fmt(format_args!("{}", self.ir_type), &palette.meta);
    dumper.write(", ", &palette.skeleton);

    debug_assert!(matches!(
      ctx!(dumper).get(variant.addr).data,
      ValueData::Variable(_) | ValueData::Instruction(_)
    ));

    dumper.write("ptr ", &palette.meta);
    dumper.write_fmt(format_args!("%{}", variant.addr), &palette.dim);
  }
}
please_dump_me!(inst::Store);
