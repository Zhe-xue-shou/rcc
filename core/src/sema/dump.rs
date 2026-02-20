use super::{
  declaration::{
    ExternalDeclaration, Function, Initializer, TranslationUnit, VarDef,
  },
  expression::Expression,
  statement::{
    self, Break, Case, Compound, Continue, DoWhile, For, Goto, If, Label,
    Return, Statement, Switch, While,
  },
};
use crate::common::{DumpRes, Dumpable, Dumper, Palette};

impl<'context> Dumpable for Expression<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    use super::expression::{RawExpr::*, *};

    dumper.print_indent(prefix, is_last)?;
    macro_rules! header {
      ($name:expr, $raw:ident) => {
        header!($name, $raw, "")
      };
      ($name:expr, $raw:ident, $newline:literal) => {
        dumper.write($name, &palette.node_type)?;
        dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
        $raw.span.dump(dumper, prefix, is_last, palette)?;
        dumper.write_fmt(
          format_args!("'{}' ", self.qualified_type()),
          &palette.meta,
        )?;
        dumper.write_fmt(
          format_args!(concat!("{} ", $newline), self.value_category()),
          &palette.info,
        )?;
      };
    }

    let subprefix = dumper.child_prefix(prefix, is_last);

    match self.raw_expr() {
      Empty(_) => dumper.write("<<<Recovery/Invalid>>>\n", &palette.error),

      Constant(constant) => {
        dumper.write("ConstantLiteral", &palette.node_type)?;
        dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
        constant.span.dump(dumper, prefix, is_last, palette)?;
        dumper.write_fmt(
          format_args!("'{}' ", self.qualified_type()),
          &palette.meta,
        )?;
        // didnt print RValue.
        dumper.write_fmt(format_args!("{}\n", constant.value), &palette.literal)
      },

      Variable(variable) => {
        header!("Variable", variable);
        dumper.write_fmt(
          format_args!(" '{}'\n", variable.name.borrow()),
          &palette.literal,
        )
      },

      Unary(unary) => {
        header!("Unary", unary);
        dumper.write_fmt(
          format_args!(" {} '{}'\n", unary.kind, unary.operator),
          &palette.operator,
        )?;
        // One child: the operand (it's the last child)
        unary.operand.dump(dumper, &subprefix, true, palette)
      },

      Binary(binary) => {
        header!("Binary", binary);
        dumper.write_fmt(
          format_args!(" '{}'\n", binary.operator),
          &palette.operator,
        )?;
        // Two children: left (not last), right (last)
        binary.left.dump(dumper, &subprefix, false, palette)?;
        binary.right.dump(dumper, &subprefix, true, palette)
      },

      Ternary(ternary) => {
        header!("Ternary", ternary, "\n");
        ternary.condition.dump(dumper, &subprefix, false, palette)?;
        ternary.then_expr.dump(dumper, &subprefix, false, palette)?;
        ternary.else_expr.dump(dumper, &subprefix, true, palette)
      },

      Call(call) => {
        header!("Call", call, "\n");
        // callee + N arguments
        let total = 1 + call.arguments.len();
        call.callee.dump(dumper, &subprefix, total == 1, palette)?;
        for (i, arg) in call.arguments.iter().enumerate() {
          arg.dump(
            dumper,
            &subprefix,
            i == call.arguments.len() - 1,
            palette,
          )?;
        }
        Ok(())
      },

      Paren(paren) => {
        header!("Paren", paren, "\n");
        paren.expr.dump(dumper, &subprefix, true, palette)
      },

      ImplicitCast(cast) => {
        header!("ImplicitCast", cast);
        dumper.write(" <", &palette.skeleton)?;
        dumper.write_fmt(format_args!("{}", cast.cast_type), &palette.kind)?;
        dumper.write(">\n", &palette.skeleton)?;
        cast.expr.dump(dumper, &subprefix, true, palette)
      },

      Assignment(assign) => {
        header!("Assignment", assign);
        dumper.write_fmt(
          format_args!(" '{}'\n", assign.operator),
          &palette.operator,
        )?;
        assign.left.dump(dumper, &subprefix, false, palette)?;
        assign.right.dump(dumper, &subprefix, true, palette)
      },

      MemberAccess(ma) => {
        header!("MemberAccess", ma);
        dumper
          .write_fmt(format_args!(" .{}\n", ma.member), &palette.literal)?;
        ma.object.dump(dumper, &subprefix, true, palette)
      },

      ArraySubscript(sub) => {
        header!("ArraySubscript", sub, "\n");
        sub.array.dump(dumper, &subprefix, false, palette)?;
        sub.index.dump(dumper, &subprefix, true, palette)
      },

      SizeOf(sof) => {
        header!("SizeOf", sof, "\n");
        match &sof.sizeof {
          SizeOfKind::Type(ty) => {
            // // Type child — just print it inline (no recursion needed)
            // dumper.print_indent(&subprefix, true)?;
            // dumper.write_fmt(format_args!("Type '{}'\n", ty), &palette.meta)

            dumper.print_indent(&subprefix, true)?;
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
        // )?;
        cast.expr.dump(dumper, &subprefix, true, palette)
      },

      CompoundLiteral(cl) => {
        header!("CompoundLiteral", cl, "\n");
        Ok(())
      },
    }
  }
}
impl<'context> Dumpable for TranslationUnit<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    dumper.write("TranslationUnit", &palette.node_type)?;
    dumper.write_fmt(format_args!(" {:p}\n", self), &palette.dim)?;
    let subprefix = dumper.child_prefix(prefix, is_last);
    self
      .declarations
      .iter()
      .enumerate()
      .try_for_each(|(i, decl)| {
        decl.dump(
          dumper,
          &subprefix,
          i == self.declarations.len() - 1,
          palette,
        )
      })
  }
}
impl<'context> Dumpable for ExternalDeclaration<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    ::rcc_utils::static_dispatch!(
      self.dump(dumper, prefix, is_last, palette),
      Variable Function
    )
  }
}

impl<'context> Dumpable for VarDef<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    dumper.print_indent(prefix, is_last)?;
    let borrowed = self.symbol.borrow();
    dumper.write(
      if borrowed.is_typedef() {
        "Typedef"
      } else {
        "VarDef"
      },
      &palette.node_type,
    )?;
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
    self.span.dump(dumper, prefix, is_last, palette)?;

    dumper.write("<", &palette.skeleton)?;
    dumper.write_fmt(format_args!("{}", borrowed.declkind), &palette.kind)?;
    dumper.write(">", &palette.skeleton)?;

    dumper
      .write_fmt(format_args!(" '{}' ", borrowed.name), &palette.literal)?;

    dumper.write("[", &palette.skeleton)?;
    dumper.write_fmt(
      format_args!("'{}'", borrowed.qualified_type),
      &palette.meta,
    )?;

    dumper.write_fmt(
      format_args!(" {:p}", borrowed.qualified_type.unqualified_type),
      &palette.skeleton,
    )?;
    dumper.write("]\n", &palette.skeleton)?;

    if let Some(initializer) = &self.initializer {
      let subprefix = dumper.child_prefix(prefix, is_last);
      initializer.dump(dumper, &subprefix, true, palette)?;
    }
    Ok(())
  }
}
impl<'context> Dumpable for Function<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("Function", &palette.node_type)?;
    dumper.write_fmt(format_args!(" {:p}", self), &palette.dim)?;
    self.span.dump(dumper, prefix, is_last, palette)?;

    dumper.write("<", &palette.skeleton)?;
    dumper.write_fmt(
      format_args!("{}", self.symbol.borrow().declkind),
      &palette.kind,
    )?;
    dumper.write(">", &palette.skeleton)?;
    dumper.write_fmt(
      format_args!(" '{}' ", self.symbol.borrow().name),
      &palette.literal,
    )?;
    dumper.write("[", &palette.skeleton)?;
    dumper.write_fmt(
      format_args!("'{}'", self.symbol.borrow().qualified_type),
      &palette.meta,
    )?;
    dumper.write_fmt(
      format_args!(
        " {:p}",
        self.symbol.borrow().qualified_type.unqualified_type
      ),
      &palette.skeleton,
    )?;
    dumper.write("]\n", &palette.skeleton)?;

    if let Some(body) = &self.body {
      let subprefix = dumper.child_prefix(prefix, is_last);
      body.dump(dumper, &subprefix, true, palette)?;
    }
    Ok(())
  }
}

impl<'context> Dumpable for Initializer<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("Initializer", &palette.node_type)?;
    match self {
      Self::Scalar(expression) => {
        dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
        expression.span().dump(dumper, prefix, is_last, palette)?;
        dumper.newline()?;
        let subprefix = dumper.child_prefix(prefix, is_last);
        expression.dump(dumper, &subprefix, true, palette)
      },
      Self::Aggregate(_) => todo!(),
    }
  }
}

impl<'context> Dumpable for Statement<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    ::rcc_utils::static_dispatch!(
      self.dump(dumper, prefix, is_last, palette),
      Empty Return Expression Declaration Compound If While DoWhile For Switch Goto Label Break Continue
    )
  }
}

impl Dumpable for statement::Empty {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("EmptyStmt", &palette.node_type)?;
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
    $dumper.print_indent($prefix, $is_last)?;
    $dumper.write($name, &$palette.node_type)?;
    $dumper.write_fmt(format_args!(" {:p} ", $self), &$palette.dim)?;
    $self.span.dump($dumper, $prefix, $is_last, &$palette)?;
    $dumper.newline()
  }};
}

impl<'context> Dumpable for Return<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "Return")?;

    if let Some(expr) = &self.expression {
      let subprefix = dumper.child_prefix(prefix, is_last);
      expr.dump(dumper, &subprefix, true, palette)?;
    }
    Ok(())
  }
}

impl<'context> Dumpable for Compound<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "Compound")?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self
      .statements
      .iter()
      .enumerate()
      .try_for_each(|(i, stmt)| {
        stmt.dump(dumper, &subprefix, i == self.statements.len() - 1, palette)
      })
  }
}

impl<'context> Dumpable for If<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "If")?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.condition.dump(dumper, &subprefix, false, palette)?;
    self.then_branch.dump(
      dumper,
      &subprefix,
      self.else_branch.is_none(),
      palette,
    )?;
    if let Some(else_branch) = &self.else_branch {
      else_branch.dump(dumper, &subprefix, true, palette)?;
    }
    Ok(())
  }
}

impl<'context> Dumpable for While<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "While")?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.condition.dump(dumper, &subprefix, false, palette)?;
    self.body.dump(dumper, &subprefix, true, palette)
  }
}

impl<'context> Dumpable for DoWhile<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "DoWhile")?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.body.dump(dumper, &subprefix, false, palette)?;
    self.condition.dump(dumper, &subprefix, true, palette)
  }
}

impl<'context> Dumpable for For<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "For")?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    if let Some(init) = &self.initializer {
      init.dump(dumper, &subprefix, false, palette)?;
    }
    if let Some(cond) = &self.condition {
      cond.dump(dumper, &subprefix, false, palette)?;
    }
    if let Some(incr) = &self.increment {
      incr.dump(dumper, &subprefix, false, palette)?;
    }
    self.body.dump(dumper, &subprefix, true, palette)
  }
}

impl<'context> Dumpable for Switch<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "Switch")?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.condition.dump(dumper, &subprefix, false, palette)?;
    self.cases.iter().enumerate().try_for_each(|(i, case)| {
      case.dump(
        dumper,
        &subprefix,
        (i == self.cases.len() - 1) && self.default.is_none(),
        palette,
      )
    })?;
    if let Some(default) = &self.default {
      default.dump(dumper, &subprefix, true, palette)?;
    }
    Ok(())
  }
}
impl<'context> Dumpable for Case<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "Case")?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    dumper
      .write_fmt(format_args!("Value: {}\n", self.value), &palette.literal)?;
    self.body.iter().enumerate().try_for_each(|(i, stmt)| {
      stmt.dump(dumper, &subprefix, i == self.body.len() - 1, palette)
    })
  }
}
impl<'context> Dumpable for statement::Default<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "Default")?;

    let subprefix = dumper.child_prefix(prefix, is_last);
    self.body.iter().enumerate().try_for_each(|(i, stmt)| {
      stmt.dump(dumper, &subprefix, i == self.body.len() - 1, palette)
    })
  }
}

impl<'context> Dumpable for Goto<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("Goto", &palette.node_type)?;
    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
    self.span.dump(dumper, prefix, is_last, palette)?;
    dumper.write_fmt(format_args!("'{}'", self.label), &palette.literal)?;
    dumper.newline()
  }
}

impl<'context> Dumpable for Label<'context> {
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    dumper.print_indent(prefix, is_last)?;
    dumper.write("Label", &palette.node_type)?;

    dumper.write_fmt(format_args!(" {:p} ", self), &palette.dim)?;
    self.span.dump(dumper, prefix, is_last, palette)?;
    dumper.write_fmt(format_args!(" '{}'\n", self.name), &palette.literal)?;

    if !matches!(*self.statement, Statement::Empty(_)) {
      let subprefix = dumper.child_prefix(prefix, is_last);
      self.statement.dump(dumper, &subprefix, true, palette)?;
    }
    Ok(())
  }
}

impl<'context> Dumpable for Break<'context> {
  #[inline]
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "Break")
  }
}

impl<'context> Dumpable for Continue<'context> {
  #[inline]
  fn dump<'t, 's>(
    &self,
    dumper: &mut impl Dumper<'t, 's>,
    prefix: &str,
    is_last: bool,
    palette: &Palette,
  ) -> DumpRes
  where
    's: 't,
  {
    headers!(self, dumper, prefix, is_last, palette, "Continue")
  }
}
