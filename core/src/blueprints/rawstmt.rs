use ::rc_utils::Dummy;

use super::Placeholder as Empty;
use crate::common::SourceSpan;

#[::enum_dispatch::enum_dispatch]
#[derive(Debug)]
pub enum RawStmt<StmtTy, DeclTy, ExprTy, ExprCaseTy = ExprTy> {
  Empty(Empty),
  Return(RawReturn<ExprTy>),
  // here only vardef, funcdef only permitted in top-level declarations hence it's handled there
  Declaration(DeclTy),
  Expression(ExprTy),
  Compound(RawCompound<StmtTy>),
  If(RawIf<StmtTy, ExprTy>),
  While(RawWhile<StmtTy, ExprTy>),
  For(RawFor<StmtTy, ExprTy>),
  DoWhile(RawDoWhile<StmtTy, ExprTy>),
  Switch(RawSwitch<StmtTy, ExprTy, ExprCaseTy>),
  Goto(RawGoto),
  Label(RawLabel<StmtTy>),
  Break(RawBreak),
  Continue(RawContinue),
}

#[macro_export(local_inner_macros)]
macro_rules! type_alias_stmt {
  ($stmtty:ident,$declty:ident,$exprty:ident) => {
    $crate::type_alias_stmt!($stmtty, $declty, $exprty, $exprty);
  };
  ($stmtty:ident,$declty:ident,$exprty:ident,$exprcasety:ident) => {
    #[allow(dead_code)]
    pub type RawStmt = $crate::blueprints::RawStmt<$stmtty, $declty, $exprty>;
    #[allow(dead_code)]
    pub type Empty = $crate::blueprints::Placeholder;
    pub type Return = $crate::blueprints::RawReturn<$exprty>;
    pub type If = $crate::blueprints::RawIf<$stmtty, $exprty>;
    pub type While = $crate::blueprints::RawWhile<$stmtty, $exprty>;
    pub type DoWhile = $crate::blueprints::RawDoWhile<$stmtty, $exprty>;
    pub type For = $crate::blueprints::RawFor<$stmtty, $exprty>;
    pub type Switch =
      $crate::blueprints::RawSwitch<$stmtty, $exprty, $exprcasety>;
    pub type Case = $crate::blueprints::RawCase<$stmtty, $exprcasety>;
    pub type Default = $crate::blueprints::RawDefault<$stmtty>;
    pub type Label = $crate::blueprints::RawLabel<$stmtty>;
    pub type Goto = $crate::blueprints::RawGoto;
    pub type Compound = $crate::blueprints::RawCompound<$stmtty>;
    pub type Break = $crate::blueprints::RawBreak;
    pub type Continue = $crate::blueprints::RawContinue;
  };
}

#[derive(Debug)]
pub struct RawReturn<ExprTy> {
  pub expression: Option<ExprTy>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawIf<StmtTy, ExprTy> {
  pub condition: ExprTy,
  pub then_branch: Box<StmtTy>,
  pub else_branch: Option<Box<StmtTy>>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawWhile<StmtTy, ExprTy> {
  pub condition: ExprTy,
  pub body: Box<StmtTy>,
  pub tag: String,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawDoWhile<StmtTy, ExprTy> {
  pub body: Box<StmtTy>,
  pub condition: ExprTy,
  pub tag: String,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawFor<StmtTy, ExprTy> {
  pub initializer: Option<Box<StmtTy>>,
  pub condition: Option<ExprTy>,
  pub increment: Option<ExprTy>,
  pub body: Box<StmtTy>,
  pub tag: String,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawSwitch<StmtTy, ExprTy, ExprCaseTy = ExprTy> {
  pub condition: ExprTy,
  pub cases: Vec<RawCase<StmtTy, ExprCaseTy>>,
  pub default: Option<RawDefault<StmtTy>>,
  pub tag: String,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawCase<StmtTy, ExprCaseTy> {
  /// [`Expression`](crate::parser::expression::Expression) in parser,
  /// [`ConstantLiteral`](crate::types::Constant) in analyzer
  /// and IT SHALL be of integral type.
  pub value: ExprCaseTy,
  pub body: Vec<StmtTy>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawDefault<StmtTy> {
  pub body: Vec<StmtTy>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawLabel<StmtTy> {
  pub name: String,
  pub statement: Box<StmtTy>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawGoto {
  pub label: String,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawCompound<StmtTy> {
  pub statements: Vec<StmtTy>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawBreak {
  pub tag: String,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawContinue {
  pub tag: String,
  pub span: SourceSpan,
}

impl RawGoto {
  pub fn new(label: String, span: SourceSpan) -> Self {
    Self { label, span }
  }
}

impl<StmtTy> RawCompound<StmtTy> {
  pub fn new(statements: Vec<StmtTy>, span: SourceSpan) -> Self {
    Self { statements, span }
  }
}
impl<StmtTy> ::core::default::Default for RawCompound<StmtTy> {
  fn default() -> Self {
    Self {
      statements: Vec::default(),
      span: SourceSpan::dummy(),
    }
  }
}

impl<StmtTy, ExprTy, ExprCaseTy> RawSwitch<StmtTy, ExprTy, ExprCaseTy> {
  pub fn new(
    condition: ExprTy,
    cases: Vec<RawCase<StmtTy, ExprCaseTy>>,
    default: Option<RawDefault<StmtTy>>,
    tag: String,
    span: SourceSpan,
  ) -> Self {
    Self {
      condition,
      cases,
      default,
      tag,
      span,
    }
  }
}

impl<StmtTy> RawLabel<StmtTy> {
  pub fn new(name: String, statement: StmtTy, span: SourceSpan) -> Self {
    Self {
      name,
      statement: Box::new(statement),
      span,
    }
  }
}

impl<StmtTy, ExprCaseTy> RawCase<StmtTy, ExprCaseTy> {
  pub fn new(value: ExprCaseTy, body: Vec<StmtTy>, span: SourceSpan) -> Self {
    Self { value, body, span }
  }
}

impl<StmtTy, ExprTy> RawIf<StmtTy, ExprTy> {
  pub fn new(
    condition: ExprTy,
    then_branch: Box<StmtTy>,
    else_branch: Option<Box<StmtTy>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      condition,
      then_branch,
      else_branch,
      span,
    }
  }
}

impl<ExprTy> RawReturn<ExprTy> {
  pub fn new(expression: Option<ExprTy>, span: SourceSpan) -> Self {
    Self { expression, span }
  }
}

impl<StmtTy, ExprTy> RawWhile<StmtTy, ExprTy> {
  pub fn new(
    condition: ExprTy,
    body: Box<StmtTy>,
    tag: String,
    span: SourceSpan,
  ) -> Self {
    Self {
      condition,
      body,
      tag,
      span,
    }
  }
}

impl<StmtTy, ExprTy> RawDoWhile<StmtTy, ExprTy> {
  pub fn new(
    body: Box<StmtTy>,
    condition: ExprTy,
    tag: String,
    span: SourceSpan,
  ) -> Self {
    Self {
      body,
      condition,
      tag,
      span,
    }
  }
}

impl<StmtTy, ExprTy> RawFor<StmtTy, ExprTy> {
  pub fn new(
    initializer: Option<Box<StmtTy>>,
    condition: Option<ExprTy>,
    increment: Option<ExprTy>,
    body: Box<StmtTy>,
    tag: String,
    span: SourceSpan,
  ) -> Self {
    Self {
      initializer,
      condition,
      increment,
      body,
      tag,
      span,
    }
  }
}

impl<StmtTy> RawDefault<StmtTy> {
  pub fn new(body: Vec<StmtTy>, span: SourceSpan) -> Self {
    Self { body, span }
  }
}

impl RawBreak {
  pub fn new(tag: String, span: SourceSpan) -> Self {
    Self { tag, span }
  }
}

impl RawContinue {
  pub fn new(tag: String, span: SourceSpan) -> Self {
    Self { tag, span }
  }
}

impl<StmtTy, DeclTy, ExprTy> RawStmt<StmtTy, DeclTy, ExprTy> {
  pub fn new_loop_dummy_identifier(str: &'static str) -> String {
    static LOOP_LABEL_COUNTER: ::std::sync::atomic::AtomicUsize =
      ::std::sync::atomic::AtomicUsize::new(0);
    let id =
      LOOP_LABEL_COUNTER.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
    format!("{}_{}", str, id)
  }
}
#[allow(clippy::write_with_newline)]
mod fmt {
  use ::std::fmt::Display;

  use super::*;

  impl<StmtTy: Display, DeclTy: Display, ExprTy: Display> Display
    for RawStmt<StmtTy, DeclTy, ExprTy>
  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        RawStmt::Empty(_) => write!(f, ";"),
        RawStmt::Return(ret) => write!(f, "{}", ret),
        RawStmt::If(if_stmt) => write!(f, "{}", if_stmt),
        RawStmt::Declaration(decl) => write!(f, "{}", decl),
        RawStmt::Expression(expr) => write!(f, "{}", expr),
        RawStmt::Compound(compound) => write!(f, "{}", compound),
        RawStmt::While(while_stmt) => write!(f, "{}", while_stmt),
        RawStmt::For(for_stmt) => write!(f, "{}", for_stmt),
        RawStmt::DoWhile(dowhile_stmt) => write!(f, "{}", dowhile_stmt),
        RawStmt::Switch(switch_stmt) => write!(f, "{}", switch_stmt),
        RawStmt::Goto(goto) => write!(f, "{}", goto),
        RawStmt::Label(label) => write!(f, "{}", label),
        RawStmt::Break(break_stmt) => write!(f, "{}", break_stmt),
        RawStmt::Continue(continue_stmt) => write!(f, "{}", continue_stmt),
      }
    }
  }

  impl<ExprTy: Display> Display for RawReturn<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match &self.expression {
        Some(expr) => write!(f, "return {}", expr),
        None => write!(f, "return"),
      }
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for RawIf<StmtTy, ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "if {} {}", self.condition, self.then_branch)?;
      if let Some(else_branch) = &self.else_branch {
        write!(f, " else {}", else_branch)?;
      }
      Ok(())
    }
  }

  impl<StmtTy: Display> Display for RawCompound<StmtTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{{\n")?;
      for stmt in &self.statements {
        write!(f, "  {}\n", stmt)?;
      }
      write!(f, "}}")
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for RawWhile<StmtTy, ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "while {} {}", self.condition, self.body)
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for RawDoWhile<StmtTy, ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "do {} while {}", self.body, self.condition)
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for RawFor<StmtTy, ExprTy> {
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

  impl Display for RawBreak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "break {};", self.tag)
    }
  }

  impl Display for RawContinue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "continue {};", self.tag)
    }
  }

  impl<StmtTy: Display> Display for RawDefault<StmtTy> {
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

  impl<StmtTy: Display> Display for RawLabel<StmtTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}: {}", self.name, self.statement)
    }
  }

  impl<StmtTy: Display, ExprCaseTy: Display> Display
    for RawCase<StmtTy, ExprCaseTy>
  {
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

  impl<StmtTy: Display, ExprTy: Display, ExprCaseTy: Display> Display
    for RawSwitch<StmtTy, ExprTy, ExprCaseTy>
  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "switch ({}) {{\n", self.condition)?;
      for case in &self.cases {
        write!(f, "  {}\n", case)?;
      }
      if let Some(default) = &self.default {
        write!(f, "  {}\n", default)?;
      }
      write!(f, "}}")
    }
  }

  impl Display for RawGoto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "goto {};", self.label)
    }
  }
}
