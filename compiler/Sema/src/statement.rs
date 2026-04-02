use ::rcc_ast::{Context, blueprints::Placeholder};
use ::rcc_shared::{ArenaVec, CollectIn, SourceSpan};
use ::rcc_utils::{StrRef, interconvert};

use super::{
  declaration::ExternalDeclarationRef as Declaration, expression::ExprRef,
};
use crate::expression::Constant;

pub type StmtRef<'c> = &'c Statement<'c>;

pub trait IntoStmtRef<'c> {
  fn into_stmt_ref(self, context: &'c Context<'c>) -> StmtRef<'c>;
}

impl<'c> IntoStmtRef<'c> for StmtRef<'c> {
  fn into_stmt_ref(self, _: &'c Context<'c>) -> StmtRef<'c> {
    self
  }
}

pub type Empty = Placeholder;

#[derive(Debug)]
pub enum Statement<'c> {
  Empty(Empty),
  Return(Return<'c>),
  Expression(ExprRef<'c>),
  Declaration(Declaration<'c>),
  Compound(Compound<'c>),
  If(If<'c>),
  While(While<'c>),
  DoWhile(DoWhile<'c>),
  For(For<'c>),
  Switch(Switch<'c>),
  Goto(Goto<'c>),
  Label(Label<'c>),
  Break(Break),
  Continue(Continue),
}

interconvert!(Declaration, Statement, 'c);
interconvert!(ExprRef, Statement, 'c, Expression);
interconvert!(Return, Statement, 'c);
interconvert!(Compound, Statement, 'c);
interconvert!(If, Statement, 'c);
interconvert!(While, Statement, 'c);
interconvert!(DoWhile, Statement, 'c);
interconvert!(For, Statement, 'c);
interconvert!(Switch, Statement, 'c);
interconvert!(Goto, Statement, 'c);
interconvert!(Label, Statement, 'c);
interconvert!(Break, Statement<'c>);
interconvert!(Continue, Statement<'c>);

#[derive(Debug)]
pub struct Return<'c> {
  pub expression: Option<ExprRef<'c>>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct If<'c> {
  pub condition: ExprRef<'c>,
  pub then_branch: StmtRef<'c>,
  pub else_branch: Option<StmtRef<'c>>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct While<'c> {
  pub condition: ExprRef<'c>,
  pub body: StmtRef<'c>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct DoWhile<'c> {
  pub body: StmtRef<'c>,
  pub condition: ExprRef<'c>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct For<'c> {
  pub initializer: Option<StmtRef<'c>>,
  pub condition: Option<ExprRef<'c>>,
  pub increment: Option<ExprRef<'c>>,
  pub body: StmtRef<'c>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Switch<'c> {
  pub condition: ExprRef<'c>,
  pub cases: &'c [Case<'c>],
  pub default: Option<Default<'c>>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Case<'c> {
  pub value: Constant<'c>,
  pub body: &'c [StmtRef<'c>],
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Default<'c> {
  pub body: &'c [StmtRef<'c>],
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Label<'c> {
  pub name: StrRef<'c>,
  pub statement: StmtRef<'c>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Goto<'c> {
  pub label: StrRef<'c>,
  pub span: SourceSpan,
}

#[derive(Debug, Default)]
pub struct Compound<'c> {
  pub statements: &'c [StmtRef<'c>],
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Break {
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Continue {
  pub span: SourceSpan,
}

::rcc_utils::ensure_is_pod!(Statement<'_>);

impl<'c> Goto<'c> {
  pub fn new(label: StrRef<'c>, span: SourceSpan) -> Self {
    Self { label, span }
  }
}

impl<'c> Compound<'c> {
  pub fn new(
    context: &'c Context<'c>,
    statements: impl IntoIterator<Item = impl IntoStmtRef<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      statements: statements
        .into_iter()
        .map(|statement| statement.into_stmt_ref(context))
        .collect_in::<ArenaVec<_>>(context.arena())
        .into_bump_slice(),
      span,
    }
  }
}

impl<'c> Switch<'c> {
  pub fn new(
    context: &'c Context<'c>,
    condition: ExprRef<'c>,
    cases: impl IntoIterator<Item = Case<'c>>,
    default: Option<Default<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      condition,
      cases: cases
        .into_iter()
        .collect_in::<ArenaVec<_>>(context.arena())
        .into_bump_slice(),
      default,
      span,
    }
  }
}

impl<'c> Label<'c> {
  pub fn new(
    context: &'c Context<'c>,
    name: StrRef<'c>,
    statement: impl IntoStmtRef<'c>,
    span: SourceSpan,
  ) -> Self {
    Self {
      name,
      statement: statement.into_stmt_ref(context),
      span,
    }
  }
}

impl<'c> Case<'c> {
  pub fn new(
    context: &'c Context<'c>,
    value: Constant<'c>,
    body: impl IntoIterator<Item = impl IntoStmtRef<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      value,
      body: body
        .into_iter()
        .map(|statement| statement.into_stmt_ref(context))
        .collect_in::<ArenaVec<_>>(context.arena())
        .into_bump_slice(),
      span,
    }
  }
}

impl<'c> If<'c> {
  pub fn new(
    context: &'c Context<'c>,
    condition: ExprRef<'c>,
    then_branch: impl IntoStmtRef<'c>,
    else_branch: Option<StmtRef<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      condition,
      then_branch: then_branch.into_stmt_ref(context),
      else_branch,
      span,
    }
  }
}

impl<'c> Return<'c> {
  pub fn new(expression: Option<ExprRef<'c>>, span: SourceSpan) -> Self {
    Self { expression, span }
  }
}

impl<'c> While<'c> {
  pub fn new(
    context: &'c Context<'c>,
    condition: ExprRef<'c>,
    body: impl IntoStmtRef<'c>,
    span: SourceSpan,
  ) -> Self {
    Self {
      condition,
      body: body.into_stmt_ref(context),
      span,
    }
  }
}

impl<'c> DoWhile<'c> {
  pub fn new(
    context: &'c Context<'c>,
    body: impl IntoStmtRef<'c>,
    condition: ExprRef<'c>,
    span: SourceSpan,
  ) -> Self {
    Self {
      body: body.into_stmt_ref(context),
      condition,
      span,
    }
  }
}

impl<'c> For<'c> {
  pub fn new(
    context: &'c Context<'c>,
    initializer: Option<StmtRef<'c>>,
    condition: Option<ExprRef<'c>>,
    increment: Option<ExprRef<'c>>,
    body: impl IntoStmtRef<'c>,
    span: SourceSpan,
  ) -> Self {
    Self {
      initializer,
      condition,
      increment,
      body: body.into_stmt_ref(context),
      span,
    }
  }
}

impl<'c> Default<'c> {
  pub fn new(
    context: &'c Context<'c>,
    body: impl IntoIterator<Item = impl IntoStmtRef<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      body: body
        .into_iter()
        .map(|statement| statement.into_stmt_ref(context))
        .collect_in::<ArenaVec<_>>(context.arena())
        .into_bump_slice(),
      span,
    }
  }
}

impl Break {
  pub fn new(span: SourceSpan) -> Self {
    Self { span }
  }
}

impl Continue {
  pub fn new(span: SourceSpan) -> Self {
    Self { span }
  }
}

impl<'c> ::std::default::Default for Statement<'c> {
  fn default() -> Self {
    Statement::Empty(Empty::default())
  }
}

impl<'c> Statement<'c> {
  pub fn alloc(
    context: &'c Context<'c>,
    statement: Statement<'c>,
  ) -> StmtRef<'c> {
    context.arena().alloc(statement)
  }

  pub fn alloc_slice<I, S>(
    context: &'c Context<'c>,
    statements: I,
  ) -> &'c [StmtRef<'c>]
  where
    I: IntoIterator<Item = S>,
    S: IntoStmtRef<'c>,
  {
    statements
      .into_iter()
      .map(|statement| statement.into_stmt_ref(context))
      .collect_in::<ArenaVec<_>>(context.arena())
      .into_bump_slice()
  }
}

#[allow(clippy::write_with_newline)]
mod fmt {
  use ::std::fmt::Display;

  use super::*;

  impl<'c> Display for Return<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match &self.expression {
        Some(expr) => write!(f, "return {}", expr),
        None => write!(f, "return"),
      }
    }
  }

  impl<'c> Display for If<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "if {} {}", self.condition, self.then_branch)?;
      if let Some(else_branch) = &self.else_branch {
        write!(f, " else {}", else_branch)?;
      }
      Ok(())
    }
  }

  impl<'c> Display for Compound<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{{\n")?;
      for stmt in self.statements {
        write!(f, "  {}\n", stmt)?;
      }
      write!(f, "}}")
    }
  }

  impl<'c> Display for While<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "while {} {}", self.condition, self.body)
    }
  }

  impl<'c> Display for DoWhile<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "do {} while {}", self.body, self.condition)
    }
  }

  impl<'c> Display for For<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "for ({}; {}; {}) {}",
        match &self.initializer {
          Some(init) => format!("{}", init),
          None => String::from(""),
        },
        match &self.condition {
          Some(cond) => format!("{}", cond),
          None => String::from(""),
        },
        match &self.increment {
          Some(inc) => format!("{}", inc),
          None => String::from(""),
        },
        self.body
      )
    }
  }

  impl Display for Break {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "break;")
    }
  }

  impl Display for Continue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "continue;")
    }
  }

  impl<'c> Display for Default<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "default: {}",
        self
          .body
          .iter()
          .map(|s| format!("{}", s))
          .collect::<Vec<_>>()
          .join("\n")
      )
    }
  }

  impl<'c> Display for Label<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}: {}", self.name, self.statement)
    }
  }

  impl<'c> Display for Case<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "case {}:{}{}",
        self.value,
        if self.body.len() > 1 { "\n" } else { " " },
        self
          .body
          .iter()
          .map(|s| format!("{}", s))
          .collect::<Vec<_>>()
          .join("\n")
      )
    }
  }

  impl<'c> Display for Switch<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "switch ({}) {{\n", self.condition)?;
      for case in self.cases {
        write!(f, "  {}\n", case)?;
      }
      if let Some(default) = &self.default {
        write!(f, "  {}\n", default)?;
      }
      write!(f, "}}")
    }
  }

  impl<'c> Display for Goto<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "goto {};", self.label)
    }
  }

  impl<'c> Display for Statement<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      ::rcc_utils::static_dispatch!(
        self,
        |variant| variant.fmt(f) =>
        Empty Return Expression Declaration Compound If While DoWhile For Switch Goto Label Break Continue
      )
    }
  }
}
