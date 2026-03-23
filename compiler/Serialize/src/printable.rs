#![allow(unused)]

use ::rcc_adt::Integral;
use ::rcc_ir::{
  Module, Value, ValueData, ValueID, instruction as inst, module,
};
use ::rcc_shared::Constant;

use crate::{Palette, Printable, pre, printer::Printer, quoted, suff};

#[macro_use]
mod macros {
  macro_rules! lookup {
    ($self:ident, $value_id:expr) => {
      $self.ir().get($value_id)
    };
  }
}
fn pretty_print_contant_or_id<'c>(
  printer: &mut impl Printer<'c>,
  value_id: ValueID,
  palette: &Palette,
  ir_type: bool,
) {
  if ir_type {
    printer.write(
      suff!(" " => printer.ir().get(value_id).ir_type),
      &palette.meta,
    );
  }
  if let Some(value) = printer.ir().get_by_constant_id(&value_id) {
    use ::rcc_ir::Type::*;
    match printer.ir().get(value_id).ir_type {
      Floating(_) => printer.write_fmt(
        format_args!("{:.e}", value.as_floating_unchecked()),
        &palette.literal,
      ),
      Pointer() => {
        debug_assert!(value.is_nullptr());
        printer.write("null", &palette.literal)
      },
      Integer(1u8) => printer.write(value.is_one(), &palette.literal),
      // if the value is max, print it as -1 for better readability.
      Integer(width) => match value.as_integral_unchecked() {
        bitmask if *bitmask == Integral::bitmask(*width) =>
          printer.write("-1", &palette.literal),
        integer => printer.write(integer, &palette.literal),
      },
      _ => printer.write(value, &palette.literal),
    }
  } else {
    printer.write(pre!("%"=> printer.get_id(value_id)), &palette.skeleton)
  }
}
fn print_users<'c>(
  printer: &mut impl Printer<'c>,
  palette: &Palette,
  value_id: ValueID,
) {
  let print_current_id = printer.ir().get(value_id).ir_type.is_void();
  let use_list = printer.ir().get_use_list(value_id);
  let usees = || {
    use_list
      .iter()
      .map(|&user_id| format!("%{}", printer.get_id(user_id)))
      .collect::<Vec<_>>()
      .join(", ")
  };
  let args = match (print_current_id, use_list.is_empty()) {
    (true, true) => format_args!("\t\t\t\t; id %{}", printer.get_id(value_id)),
    (true, false) => format_args!(
      "\t\t\t\t; id %{}, used by {}",
      printer.get_id(value_id),
      usees()
    ),
    (false, true) => format_args!(""),
    (false, false) => format_args!("\t\t\t\t; used by {}", usees()),
  };
  printer.write_fmt(args, &palette.info);
}
impl<'c> Printable<'c> for Module {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    self.globals.iter().for_each(|&value_id| {
      Printable::print(
        &*lookup!(printer, value_id),
        printer,
        prefix,
        is_last,
        palette,
      )
    })
  }
}

impl<'c> Printable<'c> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    ::rcc_utils::static_dispatch!(
        ValueData: &self.data,
        |variant| Print::print(self, printer, prefix, is_last, palette, variant) =>
        Instruction Constant Function Variable BasicBlock Argument
    )
  }
}
trait Print<'c, DataTy> {
  /// This is a special version of [`Printable::print`] for printing a specific variant of [`ValueData`].
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &DataTy,
  );
}
/// Useless stuffs toi bypass type checker for now.
#[allow(unused)]
macro_rules! please_print_me {
  ($DataTy:ty) => {
    #[allow(unused)]
    impl<'c> Print<'c, $DataTy> for Value<'c> {
      fn print(
        &self,
        printer: &mut impl Printer<'c>,
        prefix: &str,
        is_last: bool,
        palette: &Palette,
        variant: &$DataTy,
      ) {
        todo!()
      }
    }
  };
}
impl<'c> Print<'c, inst::Instruction> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Instruction,
  ) {
    // my static_dispatch uses `ident` instead of
    // `type` of the 1st arg(qual path is unstable and rust-analyzer is having a hard time to hightlighing that).
    // hence strip the `::` path here.
    use inst::Instruction;
    ::rcc_utils::static_dispatch!(
        Instruction : variant,
        |variant| Print::print(self, printer, prefix, is_last, palette, variant) =>
        Phi Terminator Unary Binary Memory Cast Call Cmp
    )
  }
}

impl<'c> Print<'c, Constant<'_>> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &Constant<'_>,
  ) {
    printer.write(suff!(" " => self.ir_type), &palette.meta);
    printer.write(variant, &palette.literal);
  }
}
impl<'c> Print<'c, module::Function<'_>> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::Function<'_>,
  ) {
    fn preds<'c>(
      printer: &mut impl Printer<'c>,
      palette: &Palette,
      block_id: ValueID,
    ) {
      let use_list = printer.ir().get_use_list(block_id);
      if !use_list.is_empty() {
        printer.write_fmt(
          format_args!(
            "\t\t\t\t\t; preds = {}",
            use_list
              .iter()
              .map(|&user_id| format!(
                "%{}",
                printer.get_id(lookup!(printer, user_id).parent)
              ))
              .collect::<Vec<_>>()
              .join(", ")
          ),
          &palette.info,
        );
      }
    }

    printer.write(
      suff!(
        " " =>
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
      &palette.literal,
    );

    printer.write(
      suff!(" " => self.ir_type.as_function_unchecked().return_type),
      &palette.meta,
    );

    printer.write(pre!("@" => variant.name), &palette.skeleton);
    printer.write("(", &palette.skeleton);
    if variant.is_definition() {
      variant
        .params
        .iter()
        .enumerate()
        .for_each(|(index, &arg_id)| {
          let arg = &*lookup!(printer, arg_id);
          Print::print(
            arg,
            printer,
            /* index */ &format!("{}", printer.get_id(arg_id)),
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
          printer.write(suff!(" " => param_ty), &palette.meta);
          printer.write(
            if variant.params.is_empty() || index + 1 == variant.params.len() {
              ""
            } else {
              ", "
            },
            &palette.dim,
          );
        });
    }
    printer.write(")", &palette.skeleton);
    if variant.is_definition() {
      printer.writeln(" {", &palette.skeleton);
      variant.blocks.iter().for_each(|&block_id| {
        printer
          .write(suff!(":" => printer.get_id(block_id)), &palette.skeleton);
        let block = &*lookup!(printer, block_id);
        preds(printer, palette, block_id);
        printer.newline();
        Print::print(
          block,
          printer,
          "\t",
          false,
          palette,
          block.data.as_basicblock_unchecked(),
        );
      });
      printer.write("}", &palette.skeleton);
    }
    printer.newline();
    printer.reset_counter();
  }
}
impl<'c> Print<'c, module::Variable<'_>> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::Variable<'_>,
  ) {
    todo!()
  }
}
impl<'c> Print<'c, module::BasicBlock> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &module::BasicBlock,
  ) {
    variant.instructions.iter().for_each(|&inst_id| {
      printer.write(prefix, &palette.dim);
      let value = lookup!(printer, inst_id);

      if !value.ir_type.is_void() && !value.ir_type.is_label() {
        printer
          .write_fmt(pre!("%"=> printer.get_id(inst_id)), &palette.skeleton);
        printer.write(" = ", &palette.skeleton);
      }

      Printable::print(&*value, printer, "", is_last, palette);
      // print_users(printer, palette, inst_id);
      printer.newline();
    });
    let terminator = &*lookup!(printer, variant.terminator);
    printer.write(prefix, &palette.dim);
    Print::print(
      terminator,
      printer,
      "",
      true,
      palette,
      terminator
        .data
        .as_instruction_unchecked()
        .as_terminator_unchecked(),
    );
    // print_users(printer, palette, variant.terminator);
    printer.newline();
  }
}
impl<'c> Print<'c, module::Argument> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    index: &str, // coontext is actually an index
    is_last: bool,
    palette: &Palette,
    _variant: &module::Argument,
  ) {
    printer.write(suff!(" " => self.ir_type), &palette.meta);
    printer.write(pre!("%" => index), &palette.skeleton);
    printer.write(if is_last { "" } else { ", " }, &palette.dim);
  }
}

please_print_me!(inst::Phi);
impl<'c> Print<'c, inst::Terminator> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Terminator,
  ) {
    use inst::Terminator;
    ::rcc_utils::static_dispatch!(
        Terminator : variant,
        |variant| Print::print(self, printer, prefix, is_last, palette, variant) =>
        Jump Branch Return Unreachable
    )
  }
}
please_print_me!(inst::Unary);
impl<'c> Print<'c, inst::Binary> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Binary,
  ) {
    printer.write(suff!(" " => variant.operator()), &palette.literal);

    self::pretty_print_contant_or_id(printer, variant.left(), palette, true);
    printer.write(", ", &palette.skeleton);
    self::pretty_print_contant_or_id(printer, variant.right(), palette, false);
  }
}
impl<'c> Print<'c, inst::Memory> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Memory,
  ) {
    use inst::Memory;
    ::rcc_utils::static_dispatch!(
        Memory : variant,
        |variant| Print::print(self, printer, prefix, is_last, palette, variant) =>
        Alloca Load Store
    )
  }
}
impl<'c> Print<'c, inst::Cast> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Cast,
  ) {
    use inst::Cast;
    ::rcc_utils::static_dispatch!(
        Cast : variant,
        |variant| Print::print(self, printer, prefix, is_last, palette, variant) =>
        Zext Sext Trunc
    )
  }
}
impl<'c> Print<'c, inst::Call> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Call,
  ) {
    printer.write("call ", &palette.literal);
    match &lookup!(printer, variant.callee()).data {
      ValueData::Instruction(instruction) => todo!(),
      ValueData::Constant(constant) => todo!(),
      ValueData::Variable(variable) => todo!(),
      ValueData::Argument(_) =>
        unreachable!("this should be impossible, or not implemented."),
      ValueData::Function(function) => {
        printer.write(suff!(" " => self.ir_type), &palette.meta);
        printer.write(quoted!("@", function.name, "("), &palette.skeleton);
        variant.args().iter().for_each(|&arg_id| {
          let arg = &*lookup!(printer, arg_id);
          printer.write(suff!(" " => arg.ir_type), &palette.meta);
          printer.write(
            arg.data.as_constant().map_or_else(
              || format!("%{}", printer.get_id(arg_id)),
              |constant| format!("{}", constant),
            ),
            &palette.skeleton,
          );
          printer.write(", ", &palette.skeleton);
        });
        printer.write(")", &palette.skeleton);
      },
      ValueData::BasicBlock(_) => unreachable!(),
    }
  }
}
impl<'c> Print<'c, inst::Cmp> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Cmp,
  ) {
    use inst::Cmp;
    ::rcc_utils::static_dispatch!(
        Cmp : variant,
        |variant| Print::print(self, printer, prefix, is_last, palette, variant) =>
        ICmp FCmp
    )
  }
}
impl<'c> Print<'c, inst::ICmp> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::ICmp,
  ) {
    printer.write("icmp ", &palette.literal);
    printer.write(suff!(" " => variant.predicate()), &palette.literal);

    self::pretty_print_contant_or_id(printer, variant.lhs(), palette, true);
    printer.write(", ", &palette.skeleton);
    self::pretty_print_contant_or_id(printer, variant.rhs(), palette, false);
  }
}

impl<'c> Print<'c, inst::FCmp> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::FCmp,
  ) {
    printer.write("fcmp ", &palette.literal);
    printer.write(suff!(" " => variant.predicate()), &palette.literal);

    self::pretty_print_contant_or_id(printer, variant.lhs(), palette, true);
    printer.write(", ", &palette.skeleton);
    self::pretty_print_contant_or_id(printer, variant.rhs(), palette, false);
  }
}

impl<'c> Print<'c, inst::Jump> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Jump,
  ) {
    printer.write("br ", &palette.literal);
    debug_assert!(lookup!(printer, variant.target()).ir_type.is_label());
    printer.write("label ", &palette.meta);
    printer.write(
      pre!("%" => printer.get_id(variant.target())),
      &palette.skeleton,
    );
  }
}
impl<'c> Print<'c, inst::Branch> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Branch,
  ) {
    printer.write("br ", &palette.literal);

    debug_assert!(self.ir_type.is_void());
    printer.write("i1 ", &palette.meta);
    printer.write(
      pre!("%" => printer.get_id(variant.condition())),
      &palette.skeleton,
    );
    printer.write(", ", &palette.skeleton);
    debug_assert!(lookup!(printer, variant.then_branch()).ir_type.is_label());
    printer.write("label ", &palette.meta);
    printer.write(
      pre!("%" => printer.get_id(variant.then_branch())),
      &palette.skeleton,
    );

    printer.write(", ", &palette.skeleton);
    debug_assert!(lookup!(printer, variant.else_branch()).ir_type.is_label());
    printer.write("label ", &palette.meta);
    printer.write(
      pre!("%" => printer.get_id(variant.else_branch())),
      &palette.skeleton,
    );
  }
}
impl<'c> Print<'c, inst::Return> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    _prefix: &str,
    _is_last: bool,
    palette: &Palette,
    variant: &inst::Return,
  ) {
    debug_assert!(_is_last);
    printer.write("ret ", &palette.literal);
    if let Some(value_id) = variant.result() {
      self::pretty_print_contant_or_id(printer, value_id, palette, true);
    } else {
      printer.write("void", &palette.meta);
    }
  }
}
impl<'c> Print<'c, inst::Unreachable> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    _is_last: bool,
    palette: &Palette,
    variant: &inst::Unreachable,
  ) {
    printer.write("unreachable", &palette.literal);
  }
}

impl<'c> Print<'c, inst::Alloca> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Alloca,
  ) {
    printer.write("alloca ", &palette.literal);
    printer.write(printer.ir().ir_type(self.ast_type), &palette.meta)
  }
}
impl<'c> Print<'c, inst::Load> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Load,
  ) {
    printer.write("load ", &palette.literal);
    printer.write(self.ir_type, &palette.meta);
    printer.write(", ", &palette.skeleton);

    debug_assert!(lookup!(printer, variant.addr()).ir_type.is_pointer());

    printer.write("ptr ", &palette.meta);
    printer.write(
      pre!("%" => printer.get_id(variant.addr())),
      &palette.skeleton,
    );
  }
}
impl<'c> Print<'c, inst::Store> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    _is_last: bool,
    palette: &Palette,
    variant: &inst::Store,
  ) {
    printer.write(prefix, &palette.dim);
    printer.write("store ", &palette.literal);

    self::pretty_print_contant_or_id(printer, variant.addr(), palette, true);

    printer.write(", ", &palette.skeleton);

    debug_assert!(lookup!(printer, variant.data()).ir_type.is_pointer());

    printer.write("ptr ", &palette.meta);
    printer.write(
      pre!("%" => printer.get_id(variant.data())),
      &palette.skeleton,
    );
  }
}

impl<'c> Print<'c, inst::Zext> for Value<'c> {
  fn print(
    &self,
    printer: &mut impl Printer<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
    variant: &inst::Zext,
  ) {
    printer.write("zext ", &palette.literal);

    self::pretty_print_contant_or_id(printer, variant.operand(), palette, true);

    printer.write(" to ", &palette.skeleton);
    printer.write(self.ir_type, &palette.meta);
  }
}
please_print_me!(inst::Sext);
please_print_me!(inst::Trunc);
