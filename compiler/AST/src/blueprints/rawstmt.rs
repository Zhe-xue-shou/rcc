use ::rcc_shared::SourceSpan;
use ::rcc_utils::StrRef;

// #[derive(Debug)]
// pub enum RawStmt<'c, StmtTy, DeclTy, ExprTy, ExprCaseTy = ExprTy> {
//   Empty(Empty),
//   Return(RawReturn<ExprTy>),
//   // here only vardef, funcdef only permitted in top-level declarations hence it's handled there
//   Declaration(DeclTy),
//   Expression(ExprTy),
//   Compound(RawCompound<StmtTy>),
//   If(RawIf<StmtTy, ExprTy>),
//   While(RawWhile<StmtTy, ExprTy>),
//   For(RawFor<StmtTy, ExprTy>),
//   DoWhile(RawDoWhile<StmtTy, ExprTy>),
//   Switch(RawSwitch<StmtTy, ExprTy, ExprCaseTy>),
//   Goto(RawGoto<'c>),
//   Label(RawLabel<'c, StmtTy>),
//   Break(RawBreak),
//   Continue(RawContinue),
// }
#[macro_export]
macro_rules! type_alias_stmt {
  ($stmtty:ty, $declty:ty, $exprty:ty) => {
    type_alias_stmt!($stmtty, $declty, $exprty, $exprty);
  };
  ($stmtty:ty, $declty:ty, $exprty:ty, $exprcasety:ty) => {
    type_alias_stmt!(@impl $stmtty, $declty, $exprty, $exprcasety);
  };
  (@impl $stmtty:ty, $declty:ty, $exprty:ty, $exprcasety:ty) => {
    // #[allow(dead_code)]
    // pub type RawStmt<'c> = $crate::blueprints::RawStmt<'c, $stmtty, $declty, $exprty>;
    #[allow(dead_code)]
    pub type Empty = $crate::blueprints::Placeholder;
    pub type Return<'c> = $crate::blueprints::RawReturn<$exprty>;
    pub type If<'c> = $crate::blueprints::RawIf<$stmtty, $exprty>;
    pub type While<'c> = $crate::blueprints::RawWhile<$stmtty, $exprty>;
    pub type DoWhile<'c> = $crate::blueprints::RawDoWhile<$stmtty, $exprty>;
    pub type For<'c> = $crate::blueprints::RawFor<$stmtty, $exprty>;
    pub type Switch<'c> =
      $crate::blueprints::RawSwitch<$stmtty, $exprty, $exprcasety>;
    pub type Case<'c> = $crate::blueprints::RawCase<$stmtty, $exprcasety>;
    pub type Default<'c> = $crate::blueprints::RawDefault<$stmtty>;
    pub type Label<'c> = $crate::blueprints::RawLabel<'c, $stmtty>;
    pub type Goto<'c> = $crate::blueprints::RawGoto<'c>;
    pub type Compound <'c> = $crate::blueprints::RawCompound<$stmtty>;
    pub type Break<'c> = $crate::blueprints::RawBreak;
    pub type Continue<'c> = $crate::blueprints::RawContinue;
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

  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawDoWhile<StmtTy, ExprTy> {
  pub body: Box<StmtTy>,
  pub condition: ExprTy,

  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawFor<StmtTy, ExprTy> {
  pub initializer: Option<Box<StmtTy>>,
  pub condition: Option<ExprTy>,
  pub increment: Option<ExprTy>,
  pub body: Box<StmtTy>,

  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawSwitch<StmtTy, ExprTy, ExprCaseTy = ExprTy> {
  pub condition: ExprTy,
  pub cases: Vec<RawCase<StmtTy, ExprCaseTy>>,
  pub default: Option<RawDefault<StmtTy>>,

  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct RawCase<StmtTy, ExprCaseTy> {
  /// [`Expression`](crate::parse::expression::Expression) in parser,
  /// [`ConstantLiteral`](crate::common::Constant) in analyzer
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
pub struct RawLabel<'c, StmtTy> {
  pub name: StrRef<'c>,
  pub statement: Box<StmtTy>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawGoto<'c> {
  pub label: StrRef<'c>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawCompound<StmtTy> {
  pub statements: Vec<StmtTy>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawBreak {
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawContinue {
  pub span: SourceSpan,
}

impl<'c> RawGoto<'c> {
  pub fn new(label: StrRef<'c>, span: SourceSpan) -> Self {
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
      span: SourceSpan::default(),
    }
  }
}

impl<StmtTy, ExprTy, ExprCaseTy> RawSwitch<StmtTy, ExprTy, ExprCaseTy> {
  pub fn new(
    condition: ExprTy,
    cases: Vec<RawCase<StmtTy, ExprCaseTy>>,
    default: Option<RawDefault<StmtTy>>,

    span: SourceSpan,
  ) -> Self {
    Self {
      condition,
      cases,
      default,

      span,
    }
  }
}

impl<'c, StmtTy> RawLabel<'c, StmtTy> {
  pub fn new(name: StrRef<'c>, statement: StmtTy, span: SourceSpan) -> Self {
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
  pub fn new(condition: ExprTy, body: Box<StmtTy>, span: SourceSpan) -> Self {
    Self {
      condition,
      body,

      span,
    }
  }
}

impl<StmtTy, ExprTy> RawDoWhile<StmtTy, ExprTy> {
  pub fn new(body: Box<StmtTy>, condition: ExprTy, span: SourceSpan) -> Self {
    Self {
      body,
      condition,

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

    span: SourceSpan,
  ) -> Self {
    Self {
      initializer,
      condition,
      increment,
      body,

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
  pub fn new(span: SourceSpan) -> Self {
    Self { span }
  }
}

impl RawContinue {
  pub fn new(span: SourceSpan) -> Self {
    Self { span }
  }
}

#[allow(clippy::write_with_newline)]
mod fmt {
  use ::std::fmt::Display;

  use super::*;

  // impl<'c, StmtTy: Display, DeclTy: Display, ExprTy: Display> Display
  //   for RawStmt<'c, StmtTy, DeclTy, ExprTy>
  // {
  //   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
  //     match self {
  //       RawStmt::Empty(_) => write!(f, ";"),
  //       RawStmt::Return(ret) => write!(f, "{}", ret),
  //       RawStmt::If(if_stmt) => write!(f, "{}", if_stmt),
  //       RawStmt::Declaration(decl) => write!(f, "{}", decl),
  //       RawStmt::Expression(expr) => write!(f, "{}", expr),
  //       RawStmt::Compound(compound) => write!(f, "{}", compound),
  //       RawStmt::While(while_stmt) => write!(f, "{}", while_stmt),
  //       RawStmt::For(for_stmt) => write!(f, "{}", for_stmt),
  //       RawStmt::DoWhile(dowhile_stmt) => write!(f, "{}", dowhile_stmt),
  //       RawStmt::Switch(switch_stmt) => write!(f, "{}", switch_stmt),
  //       RawStmt::Goto(goto) => write!(f, "{}", goto),
  //       RawStmt::Label(label) => write!(f, "{}", label),
  //       RawStmt::Break(break_stmt) => write!(f, "{}", break_stmt),
  //       RawStmt::Continue(continue_stmt) => write!(f, "{}", continue_stmt),
  //     }
  //   }
  // }

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
      write!(f, "break;")
    }
  }

  impl Display for RawContinue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "continue;")
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

  impl<'c, StmtTy: Display> Display for RawLabel<'c, StmtTy> {
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

  impl<'c> Display for RawGoto<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "goto {};", self.label)
    }
  }
}
