#[derive(Debug)]
pub enum RawStmt<StmtTy, DeclTy, ExprTy> {
  Empty,
  Return(RawReturn<ExprTy>),
  // here only vardef, funcdef only permitted in top-level declarations hence it's handled there
  Declaration(DeclTy),
  Expression(ExprTy),
  Compound(RawCompound<StmtTy>),
  If(RawIf<StmtTy, ExprTy>),
  While(RawWhile<StmtTy, ExprTy>),
  For(RawFor<StmtTy, ExprTy>),
  DoWhile(RawDoWhile<StmtTy, ExprTy>),
  Switch(RawSwitch<StmtTy, ExprTy>),
  Goto(RawGoto),
  Label(RawLabel<StmtTy>),
  Break(RawBreak),
  Continue(RawContinue),
}

#[macro_export(local_inner_macros)]
macro_rules! type_alias_stmt {
  ($stmtty:ident,$declty:ident,$exprty:ident) => {
    #[allow(dead_code)]
    pub type RawStmt =
      $crate::common::rawstmt::RawStmt<$stmtty, $declty, $exprty>;
    pub type Return = $crate::common::rawstmt::RawReturn<$exprty>;
    pub type If = $crate::common::rawstmt::RawIf<$stmtty, $exprty>;
    pub type While = $crate::common::rawstmt::RawWhile<$stmtty, $exprty>;
    pub type DoWhile = $crate::common::rawstmt::RawDoWhile<$stmtty, $exprty>;
    pub type For = $crate::common::rawstmt::RawFor<$stmtty, $exprty>;
    pub type Switch = $crate::common::rawstmt::RawSwitch<$stmtty, $exprty>;
    pub type Case = $crate::common::rawstmt::RawCase<$stmtty, $exprty>;
    pub type Default = $crate::common::rawstmt::RawDefault<$stmtty>;
    pub type Label = $crate::common::rawstmt::RawLabel<$stmtty>;
    pub type Goto = $crate::common::rawstmt::RawGoto;
    pub type Compound = $crate::common::rawstmt::RawCompound<$stmtty>;
    pub type Break = $crate::common::rawstmt::RawBreak;
    pub type Continue = $crate::common::rawstmt::RawContinue;
  };
}

#[derive(Debug)]
pub struct RawReturn<ExprTy> {
  pub expression: Option<ExprTy>,
}

#[derive(Debug)]
pub struct RawIf<StmtTy, ExprTy> {
  pub condition: ExprTy,
  pub then_branch: Box<StmtTy>,
  pub else_branch: Option<Box<StmtTy>>,
}

#[derive(Debug)]
pub struct RawWhile<StmtTy, ExprTy> {
  pub condition: ExprTy,
  pub body: Box<StmtTy>,
  pub tag: String,
}

#[derive(Debug)]
pub struct RawDoWhile<StmtTy, ExprTy> {
  pub body: Box<StmtTy>,
  pub condition: ExprTy,
  pub tag: String,
}

#[derive(Debug)]
pub struct RawFor<StmtTy, ExprTy> {
  pub initializer: Option<Box<StmtTy>>,
  pub condition: Option<ExprTy>,
  pub increment: Option<ExprTy>,
  pub body: Box<StmtTy>,
  pub tag: String,
}

#[derive(Debug)]
pub struct RawSwitch<StmtTy, ExprTy> {
  pub condition: ExprTy,
  pub cases: Vec<RawCase<StmtTy, ExprTy>>,
  pub default: Option<RawDefault<StmtTy>>,
  pub tag: String,
}
#[derive(Debug)]
pub struct RawCase<StmtTy, ExprTy> {
  pub value: ExprTy, // Must be constant integer expression
  pub body: Vec<StmtTy>,
}

#[derive(Debug)]
pub struct RawDefault<StmtTy> {
  pub body: Vec<StmtTy>,
}

#[derive(Debug)]
pub struct RawLabel<StmtTy> {
  pub name: String,
  pub statement: Box<StmtTy>,
}

#[derive(Debug)]
pub struct RawGoto {
  pub label: String,
}

#[derive(Debug)]
pub struct RawCompound<StmtTy> {
  pub statements: Vec<StmtTy>,
}

#[derive(Debug)]
pub struct RawBreak {
  pub tag: String,
}

#[derive(Debug)]
pub struct RawContinue {
  pub tag: String,
}

impl RawGoto {
  pub fn new(label: String) -> Self {
    Self { label }
  }
}

impl<StmtTy> RawCompound<StmtTy> {
  pub fn new(statements: Vec<StmtTy>) -> Self {
    Self { statements }
  }
}
impl<StmtTy> ::core::default::Default for RawCompound<StmtTy> {
  fn default() -> Self {
    Self {
      statements: Vec::default(),
    }
  }
}

impl<StmtTy, ExprTy> RawSwitch<StmtTy, ExprTy> {
  pub fn new(
    condition: ExprTy,
    cases: Vec<RawCase<StmtTy, ExprTy>>,
    default: Option<RawDefault<StmtTy>>,
    tag: String,
  ) -> Self {
    Self {
      condition,
      cases,
      default,
      tag,
    }
  }
}

impl<StmtTy> RawLabel<StmtTy> {
  pub fn new(name: String, statement: StmtTy) -> Self {
    Self {
      name,
      statement: Box::new(statement),
    }
  }
}

impl<StmtTy, ExprTy> RawCase<StmtTy, ExprTy> {
  pub fn new(value: ExprTy, body: Vec<StmtTy>) -> Self {
    Self { value, body }
  }
}

impl<StmtTy, ExprTy> RawIf<StmtTy, ExprTy> {
  pub fn new(
    condition: ExprTy,
    then_branch: Box<StmtTy>,
    else_branch: Option<Box<StmtTy>>,
  ) -> Self {
    Self {
      condition,
      then_branch,
      else_branch,
    }
  }
}

impl<ExprTy> RawReturn<ExprTy> {
  pub fn new(expression: Option<ExprTy>) -> Self {
    Self { expression }
  }
}

impl<StmtTy, ExprTy> RawWhile<StmtTy, ExprTy> {
  pub fn new(condition: ExprTy, body: Box<StmtTy>, tag: String) -> Self {
    Self {
      condition,
      body,
      tag,
    }
  }
}

impl<StmtTy, ExprTy> RawDoWhile<StmtTy, ExprTy> {
  pub fn new(body: Box<StmtTy>, condition: ExprTy, tag: String) -> Self {
    Self {
      body,
      condition,
      tag,
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
  ) -> Self {
    Self {
      initializer,
      condition,
      increment,
      body,
      tag,
    }
  }
}

impl<StmtTy> RawDefault<StmtTy> {
  pub fn new(body: Vec<StmtTy>) -> Self {
    Self { body }
  }
}

impl RawBreak {
  pub fn new(tag: String) -> Self {
    Self { tag }
  }
}

impl RawContinue {
  pub fn new(tag: String) -> Self {
    Self { tag }
  }
}

impl<StmtTy, DeclTy, ExprTy> RawStmt<StmtTy, DeclTy, ExprTy> {
  pub fn new_loop_dummy_identifier(str: &'static str) -> String {
    static LOOP_LABEL_COUNTER: std::sync::atomic::AtomicUsize =
      std::sync::atomic::AtomicUsize::new(0);
    let id =
      LOOP_LABEL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}_{}", str, id)
  }
}

mod fmt {
  use ::std::fmt::Display;

  use super::*;

  impl<StmtTy: Display, DeclTy: Display, ExprTy: Display> Display
    for RawStmt<StmtTy, DeclTy, ExprTy>
  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        RawStmt::Empty => write!(f, ";"),
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

  impl<StmtTy: Display, ExprTy: Display> Display for RawCase<StmtTy, ExprTy> {
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

  impl<StmtTy: Display, ExprTy: Display> Display for RawSwitch<StmtTy, ExprTy> {
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
