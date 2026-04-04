use ::rcc_ast::types::{
  Array, Enum, FunctionProto, Pointer, Primitive, QualifiedType, Record, Type,
  Union,
};
use ::rcc_sema::{
  declaration::{
    Designated, Designator, ExternalDeclarationRef, Function, Initializer,
    InitializerEntry, InitializerList, InitializerListEntry, TranslationUnit,
    VarDef,
  },
  expression::{Empty, Expression},
  statement::{
    self, Break, Case, Compound, Continue, DoWhile, For, Goto, If, Label,
    Return, Statement, Switch, While,
  },
};

use crate::{DumpSpan, Dumpable, Dumper, Palette, quoted};

impl<'c, T> Dumpable<'c> for &T
where
  T: Dumpable<'c> + ?Sized,
{
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    (**self).dump(dumper, prefix, is_last, palette)
  }
}

impl<'c> Dumpable<'c> for QualifiedType<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
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
  ) {
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
  ) {
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
  ) {
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
  ) {
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
  ) {
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
  ) {
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
  ) {
    todo!()
  }
}
#[allow(unused)]
impl<'c> Dumpable<'c> for Union<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    todo!()
  }
}

impl<'c> Dumpable<'c> for Empty {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    dumper.print_indent(prefix, is_last);
    dumper.write("EmptyStmt", &palette.node);
    dumper.write_fmt(format_args!(" {:p}\n", self), &palette.dim)
  }
}

macro_rules! headers {
  (
    $self:ident,
    $dumper:ident,
    $prefix:ident,
    $is_last:ident,
    $palette:ident,
    $name:expr
  ) => {{
    $dumper.print_indent($prefix, $is_last);
    $dumper.write($name, &$palette.node);
    $dumper.write_fmt(format_args!(" {:p} ", $self), &$palette.dim);
    $self.span.dump($dumper, $prefix, $is_last, &$palette);
    $dumper.newline()
  }};
}

impl<'c> Dumpable<'c> for Return<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "Return");

    if let Some(expr) = &self.expression {
      let subprefix = dumper.child_prefix(prefix, is_last);
      expr.dump(dumper, &subprefix, true, palette);
    }
  }
}

impl<'c> Dumpable<'c> for Compound<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "Compound");

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.statements.iter().enumerate().for_each(|(i, stmt)| {
      stmt.dump(dumper, &subprefix, i == self.statements.len() - 1, palette)
    })
  }
}

impl<'c> Dumpable<'c> for If<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "If");

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.condition.dump(dumper, &subprefix, false, palette);
    self.then_branch.dump(
      dumper,
      &subprefix,
      self.else_branch.is_none(),
      palette,
    );
    if let Some(else_branch) = &self.else_branch {
      else_branch.dump(dumper, &subprefix, true, palette);
    }
  }
}

impl<'c> Dumpable<'c> for While<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "While");

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.condition.dump(dumper, &subprefix, false, palette);
    self.body.dump(dumper, &subprefix, true, palette)
  }
}

impl<'c> Dumpable<'c> for DoWhile<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "DoWhile");

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.body.dump(dumper, &subprefix, false, palette);
    self.condition.dump(dumper, &subprefix, true, palette)
  }
}

impl<'c> Dumpable<'c> for For<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "For");

    let subprefix = dumper.child_prefix(prefix, is_last);
    if let Some(init) = &self.initializer {
      init.dump(dumper, &subprefix, false, palette);
    }
    if let Some(cond) = &self.condition {
      cond.dump(dumper, &subprefix, false, palette);
    }
    if let Some(incr) = &self.increment {
      incr.dump(dumper, &subprefix, false, palette);
    }
    self.body.dump(dumper, &subprefix, true, palette)
  }
}

impl<'c> Dumpable<'c> for Switch<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "Switch");

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.condition.dump(dumper, &subprefix, false, palette);
    self.cases.iter().enumerate().for_each(|(i, case)| {
      case.dump(
        dumper,
        &subprefix,
        (i == self.cases.len() - 1) && self.default.is_none(),
        palette,
      )
    });
    if let Some(default) = &self.default {
      default.dump(dumper, &subprefix, true, palette);
    }
  }
}
impl<'c> Dumpable<'c> for Case<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "Case");

    let subprefix = dumper.child_prefix(prefix, is_last);
    dumper.write_fmt(format_args!("Value: {}\n", self.value), &palette.literal);
    self.body.iter().enumerate().for_each(|(i, stmt)| {
      stmt.dump(dumper, &subprefix, i == self.body.len() - 1, palette)
    })
  }
}
impl<'c> Dumpable<'c> for statement::Default<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "Default");

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.body.iter().enumerate().for_each(|(i, stmt)| {
      stmt.dump(dumper, &subprefix, i == self.body.len() - 1, palette)
    })
  }
}

impl<'c> Dumpable<'c> for Goto<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    dumper.print_indent(prefix, is_last);
    dumper.write("Goto", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    self.span.dump(dumper, prefix, is_last, palette);
    dumper.write_fmt(format_args!("'{}'", self.label), &palette.literal);
    dumper.newline()
  }
}

impl<'c> Dumpable<'c> for Label<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    dumper.print_indent(prefix, is_last);
    dumper.write("Label", &palette.node);

    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    self.span.dump(dumper, prefix, is_last, palette);
    dumper.write_fmt(format_args!(" '{}'\n", self.name), &palette.literal);

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.statement.dump(dumper, &subprefix, true, palette);
  }
}

impl<'c> Dumpable<'c> for Break {
  #[inline]
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "Break")
  }
}

impl<'c> Dumpable<'c> for Continue {
  #[inline]
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    headers!(self, dumper, prefix, is_last, palette, "Continue")
  }
}

impl<'c> Dumpable<'c> for Expression<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    use ::rcc_sema::expression::{RawExpr::*, *};

    dumper.print_indent(prefix, is_last);
    macro_rules! header {
      ($name:expr, $raw:ident) => {
        header!($name, $raw, "")
      };
      ($name:expr, $raw:ident, $newline:literal) => {
        dumper.write($name, &palette.node);
        dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
        self.span().dump(dumper, prefix, is_last, palette);
        dumper.write_fmt(
          format_args!("'{}' ", self.qualified_type()),
          &palette.meta,
        );
        dumper.write_fmt(
          format_args!(concat!("{} ", $newline), self.value_category()),
          &palette.info,
        );
      };
    }

    let subprefix = dumper.child_prefix(prefix, is_last);

    match self.raw_expr() {
      Empty(_) => {
        dumper.write("Recovery", &palette.error);
        dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
        self.span().dump(dumper, prefix, is_last, palette);
        dumper.write_fmt(
          format_args!("'{}' ", self.qualified_type()),
          &palette.meta,
        );
        dumper.write_fmt(
          format_args!("{}\n", self.value_category()),
          &palette.info,
        );
      },

      Constant(constant) => {
        dumper.write("ConstantLiteral", &palette.node);
        dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
        self.span().dump(dumper, prefix, is_last, palette);
        dumper.write_fmt(
          format_args!("'{}' ", self.qualified_type()),
          &palette.meta,
        );
        // didnt print RValue.
        dumper.write_fmt(format_args!("{}\n", constant), &palette.literal)
      },

      Variable(variable) => {
        header!("Variable", variable);
        dumper.write_fmt(
          format_args!(" '{}'\n", variable.declaration),
          &palette.literal,
        )
      },

      Unary(unary) => {
        header!("Unary", unary);
        dumper.write_fmt(
          format_args!(" {} '{}'\n", unary.kind, unary.operator),
          &palette.operator,
        );
        // One child: the operand (it's the last child)
        unary.operand.dump(dumper, &subprefix, true, palette)
      },

      Binary(binary) => {
        header!("Binary", binary);
        dumper.write_fmt(
          format_args!(" '{}'\n", binary.operator),
          &palette.operator,
        );
        // Two children: left (not last), right (last)
        binary.left.dump(dumper, &subprefix, false, palette);
        binary.right.dump(dumper, &subprefix, true, palette)
      },

      Ternary(ternary) => {
        header!("Ternary", ternary, "\n");
        ternary.condition.dump(dumper, &subprefix, false, palette);
        if let Some(then_expr) = ternary.then_expr {
          then_expr.dump(dumper, &subprefix, false, palette);
        }
        ternary.else_expr.dump(dumper, &subprefix, true, palette)
      },

      Call(call) => {
        header!("Call", call, "\n");
        // callee + N arguments
        let total = 1 + call.arguments.len();
        call.callee.dump(dumper, &subprefix, total == 1, palette);
        for (i, arg) in call.arguments.iter().enumerate() {
          arg.dump(dumper, &subprefix, i == call.arguments.len() - 1, palette);
        }
      },

      Paren(paren) => {
        header!("Paren", paren, "\n");
        paren.expr.dump(dumper, &subprefix, true, palette)
      },

      ImplicitCast(cast) => {
        header!("ImplicitCast", cast);
        dumper.write(" <", &palette.skeleton);
        dumper.write(cast.cast_type, &palette.kind);
        dumper.write(">\n", &palette.skeleton);
        cast.expr.dump(dumper, &subprefix, true, palette)
      },

      CompoundAssign(ca) => {
        header!("CompoundAssign", ca, "");
        dumper.write_fmt(format_args!(" '{}'", ca.operator), &palette.operator);
        dumper.write(" CLHSTy=", &palette.skeleton);
        dumper
          .write(quoted!("'", ca.intermediate_left_type, "'"), &palette.meta);
        dumper.write(" CResTy=", &palette.skeleton);
        dumper.write(
          quoted!("'", ca.intermediate_result_type, "'"),
          &palette.meta,
        );
        dumper.newline();
        ca.left.dump(dumper, &subprefix, false, palette);
        ca.right.dump(dumper, &subprefix, true, palette)
      },

      MemberAccess(ma) => {
        header!("MemberAccess", ma);
        dumper.write_fmt(format_args!(" .{}\n", ma.member), &palette.literal);
        ma.object.dump(dumper, &subprefix, true, palette)
      },

      ArraySubscript(sub) => {
        header!("ArraySubscript", sub, "\n");
        sub.array.dump(dumper, &subprefix, false, palette);
        sub.index.dump(dumper, &subprefix, true, palette)
      },

      SizeOf(sof) => {
        header!("SizeOf", sof, "\n");
        match &sof.sizeof {
          SizeOfKind::Type(ty) => {
            // dumper.print_indent(&subprefix, true);
            // dumper.write_fmt(format_args!("Type '{}'\n", ty), &palette.meta)

            dumper.print_indent(&subprefix, true);
            ty.dump(dumper, prefix, true, palette)
          },
          SizeOfKind::Expression(expr) =>
            expr.dump(dumper, &subprefix, true, palette),
        }
      },

      CStyleCast(cast) => {
        header!("CStyleCast", cast);
        // dumper.write_fmt(
        //   format_args!(" '{}'\n", cast.target_type),
        //   &palette.meta,
        // );
        cast.expr.dump(dumper, &subprefix, true, palette)
      },

      CompoundLiteral(_cl) => {
        header!("CompoundLiteral", _cl, "\n");
      },
    }
  }
}
impl<'c> Dumpable<'c> for TranslationUnit<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    dumper.write("TranslationUnit", &palette.node);
    dumper.write_fmt(format_args!(" {:p}\n", self), &palette.dim);
    let subprefix = dumper.child_prefix(prefix, is_last);
    self.declarations.iter().enumerate().for_each(|(i, decl)| {
      decl.dump(
        dumper,
        &subprefix,
        i == self.declarations.len() - 1,
        palette,
      )
    });
  }
}
impl<'c> Dumpable<'c> for ExternalDeclarationRef<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.dump(dumper, prefix, is_last, palette) =>
      Variable Function
    )
  }
}

impl<'c> Dumpable<'c> for VarDef<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    dumper.print_indent(prefix, is_last);
    let decl = self.declaration;
    dumper.write(
      if matches!(decl.storage_class(), ::rcc_shared::Storage::Typedef) {
        "Typedef"
      } else {
        "VarDef"
      },
      &palette.node,
    );
    dumper.write_fmt(format_args!(" {:p} ", decl), &palette.dim);

    if let Some(prev) = decl.previous_decl() {
      dumper.write_fmt(format_args!("prev {:p} ", prev,), &palette.skeleton);
    }
    dumper.write_fmt(
      format_args!("can {:p} ", decl.canonical_decl()),
      &palette.skeleton,
    );

    if let Some(def) = decl.definition() {
      dumper.write_fmt(format_args!("def {:p} ", def,), &palette.skeleton);
    }
    self.span.dump(dumper, prefix, is_last, palette);

    dumper.write("<", &palette.skeleton);
    dumper.write(decl.declkind(), &palette.kind);
    dumper.write(">", &palette.skeleton);

    dumper.write_fmt(format_args!(" '{}' ", decl.name()), &palette.literal);

    dumper.write("[", &palette.skeleton);
    dumper
      .write_fmt(format_args!("'{}'", decl.qualified_type()), &palette.meta);

    dumper.write_fmt(
      format_args!(" {:p}", decl.qualified_type().unqualified_type),
      &palette.skeleton,
    );
    dumper.write("]\n", &palette.skeleton);

    if let Some(initializer) = &self.initializer {
      let subprefix = dumper.child_prefix(prefix, is_last);
      initializer.dump(dumper, &subprefix, true, palette);
    }
  }
}
impl<'c> Dumpable<'c> for Function<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    dumper.print_indent(prefix, is_last);
    let decl = self.declaration;

    dumper.write("Function", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", decl), &palette.dim);

    if let Some(prev) = decl.previous_decl() {
      dumper.write_fmt(format_args!("prev {:p} ", prev), &palette.skeleton);
    }
    dumper.write_fmt(
      format_args!("can {:p} ", decl.canonical_decl()),
      &palette.skeleton,
    );

    if let Some(def) = decl.definition() {
      dumper.write_fmt(format_args!("def {:p} ", def), &palette.skeleton);
    }

    self.span.dump(dumper, prefix, is_last, palette);

    dumper.write("<", &palette.skeleton);
    dumper.write(decl.declkind(), &palette.kind);
    dumper.write(">", &palette.skeleton);
    dumper.write_fmt(quoted!(" '", decl.name(), "' "), &palette.literal);
    dumper.write("[", &palette.skeleton);
    dumper.write(quoted!("'" => decl.qualified_type()), &palette.meta);
    dumper.write_fmt(
      format_args!(" {:p}", decl.qualified_type().unqualified_type),
      &palette.skeleton,
    );
    dumper.write("]\n", &palette.skeleton);

    if let Some(body) = &self.body {
      let subprefix = dumper.child_prefix(prefix, is_last);
      body.dump(dumper, &subprefix, true, palette);
    }
  }
}

impl<'c> Dumpable<'c> for Initializer<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    match self {
      Self::Scalar(expression) =>
        expression.dump(dumper, prefix, is_last, palette),
      Self::List(list) => list.dump(dumper, prefix, is_last, palette),
    }
  }
}

impl<'c> Dumpable<'c> for InitializerList<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    dumper.print_indent(prefix, is_last);

    dumper.write("InitializerList", &palette.node);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);
    self.span.dump(dumper, prefix, is_last, palette);

    if self.entries.is_empty() {
      dumper.write("zeroinit\n", &palette.info);
    } else {
      dumper.newline();
      let subprefix = dumper.child_prefix(prefix, is_last);
      self.entries.iter().enumerate().for_each(|(i, entry)| {
        entry.dump(dumper, &subprefix, i == self.entries.len() - 1, palette)
      });
    }
  }
}

impl<'c> Dumpable<'c> for InitializerListEntry<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.dump(dumper, prefix, is_last, palette) =>
      Designated InitializerEntry
    )
  }
}
#[allow(unused)]
impl<'c> Dumpable<'c> for Designated<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    todo!()
  }
}

impl<'c> Dumpable<'c> for InitializerEntry<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    self.designator.dump(dumper, prefix, true, palette);
    self.initializer.dump(dumper, prefix, is_last, palette);
  }
}

#[allow(unused)]
impl<'c> Dumpable<'c> for Designator<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    implicit: bool,
    palette: &Palette,
  ) {
    dumper.print_indent(prefix, false);
    dumper.write("Designator", &palette.skeleton);
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim);

    match self {
      Self::Array(index) => {
        dumper.write("index ", &palette.skeleton);
        dumper.write(index, &palette.meta)
      },
      Self::Field(_) => todo!(),
    }

    if implicit {
      dumper.write(" implicit", &palette.skeleton);
    }
    dumper.newline();
  }
}

impl<'c> Dumpable<'c> for Statement<'_> {
  fn dump(
    &self,
    dumper: &mut impl Dumper<'c>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.dump(dumper, prefix, is_last, palette) =>
      Empty Return Expression Declaration Compound If While DoWhile For Switch Goto Label Break Continue
    )
  }
}
