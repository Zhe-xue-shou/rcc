#[derive(Debug)]
pub enum RawStmt<StmtTy, DeclTy, ExprTy> {
  Empty,
  Return(Return<ExprTy>),
  // here only vardef, funcdef only permitted in top-level declarations hence it's handled there
  Declaration(DeclTy),
  Expression(ExprTy),
  Compound(Compound<StmtTy>),
  If(If<StmtTy, ExprTy>),
  While(While<StmtTy, ExprTy>),
  For(For<StmtTy, ExprTy>),
  DoWhile(DoWhile<StmtTy, ExprTy>),
  Switch(Switch<StmtTy, ExprTy>),
  Goto(Goto),
  Label(Label<StmtTy>),
  Break(SingleLabel),
  Continue(SingleLabel),
}

#[macro_export(local_inner_macros)]
macro_rules! type_alias_stmt {
  ($stmtty:ident,$declty:ident,$exprty:ident) => {
    pub type RawStmt =
      crate::common::rawstmt::RawStmt<$stmtty, $declty, $exprty>;
    pub type Return = crate::common::rawstmt::Return<$exprty>;
    pub type If = crate::common::rawstmt::If<$stmtty, $exprty>;
    pub type While = crate::common::rawstmt::While<$stmtty, $exprty>;
    pub type DoWhile = crate::common::rawstmt::DoWhile<$stmtty, $exprty>;
    pub type For = crate::common::rawstmt::For<$stmtty, $exprty>;
    pub type Switch = crate::common::rawstmt::Switch<$stmtty, $exprty>;
    pub type Case = crate::common::rawstmt::Case<$stmtty, $exprty>;
    pub type Default = crate::common::rawstmt::Default<$stmtty>;
    pub type Label = crate::common::rawstmt::Label<$stmtty>;
    pub type Goto = crate::common::rawstmt::Goto;
    pub type Compound = crate::common::rawstmt::Compound<$stmtty>;
    pub type SingleLabel = crate::common::rawstmt::SingleLabel;
    pub type Break = crate::common::rawstmt::SingleLabel;
    pub type Continue = crate::common::rawstmt::SingleLabel;
  };
}

#[derive(Debug)]
pub struct Return<ExprTy> {
  pub expression: Option<ExprTy>,
}

#[derive(Debug)]
pub struct If<StmtTy, ExprTy> {
  pub condition: ExprTy,
  pub then_branch: Box<StmtTy>,
  pub else_branch: Option<Box<StmtTy>>,
}

#[derive(Debug)]
pub struct While<StmtTy, ExprTy> {
  pub condition: ExprTy,
  pub body: Box<StmtTy>,
  pub label: String,
}

#[derive(Debug)]
pub struct DoWhile<StmtTy, ExprTy> {
  pub body: Box<StmtTy>,
  pub condition: ExprTy,
  pub label: String,
}

#[derive(Debug)]
pub struct For<StmtTy, ExprTy> {
  pub initializer: Option<Box<StmtTy>>,
  pub condition: Option<ExprTy>,
  pub increment: Option<ExprTy>,
  pub body: Box<StmtTy>,
  pub label: String,
}

#[derive(Debug)]
pub struct Switch<StmtTy, ExprTy> {
  pub condition: ExprTy,
  pub cases: Vec<Case<StmtTy, ExprTy>>,
  pub default: Option<Default<StmtTy>>,
}
#[derive(Debug)]
pub struct Case<StmtTy, ExprTy> {
  pub value: ExprTy, // Must be constant integer expression
  pub body: Vec<StmtTy>,
}

#[derive(Debug)]
pub struct Default<StmtTy> {
  pub body: Vec<StmtTy>,
}

#[derive(Debug)]
pub struct Label<StmtTy> {
  pub name: String,
  pub statement: Box<StmtTy>,
}

#[derive(Debug)]
pub struct Goto {
  pub label: String,
}

#[derive(Debug)]
pub struct Compound<StmtTy> {
  pub statements: Vec<StmtTy>,
}

#[derive(Debug)]
pub struct SingleLabel {
  pub name: String,
}

impl Goto {
  pub fn new(label: String) -> Self {
    Self { label }
  }
}

impl<StmtTy> Compound<StmtTy> {
  pub fn new(statements: Vec<StmtTy>) -> Self {
    Self { statements }
  }
}
impl<StmtTy> ::core::default::Default for Compound<StmtTy> {
  fn default() -> Self {
    Self { statements: vec![] }
  }
}

impl<StmtTy, ExprTy> Switch<StmtTy, ExprTy> {
  pub fn new(
    condition: ExprTy,
    cases: Vec<Case<StmtTy, ExprTy>>,
    default: Option<Default<StmtTy>>,
  ) -> Self {
    Self {
      condition,
      cases,
      default,
    }
  }
}

impl<StmtTy> Label<StmtTy> {
  pub fn new(name: String, statement: StmtTy) -> Self {
    Self {
      name,
      statement: Box::new(statement),
    }
  }
}

impl<StmtTy, ExprTy> Case<StmtTy, ExprTy> {
  pub fn new(value: ExprTy, body: Vec<StmtTy>) -> Self {
    Self { value, body }
  }
}

impl<StmtTy, ExprTy> If<StmtTy, ExprTy> {
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

impl<ExprTy> Return<ExprTy> {
  pub fn new(expression: Option<ExprTy>) -> Self {
    Self { expression }
  }
}

impl<StmtTy, ExprTy> While<StmtTy, ExprTy> {
  pub fn new(condition: ExprTy, body: Box<StmtTy>, label: String) -> Self {
    Self {
      condition,
      body,
      label,
    }
  }

  pub fn get_label(&self) -> &str {
    &self.label
  }
}

impl<StmtTy, ExprTy> DoWhile<StmtTy, ExprTy> {
  pub fn new(body: Box<StmtTy>, condition: ExprTy, label: String) -> Self {
    Self {
      body,
      condition,
      label,
    }
  }

  pub fn get_label(&self) -> &str {
    &self.label
  }
}

impl<StmtTy, ExprTy> For<StmtTy, ExprTy> {
  pub fn new(
    initializer: Option<Box<StmtTy>>,
    condition: Option<ExprTy>,
    increment: Option<ExprTy>,
    body: Box<StmtTy>,
    label: String,
  ) -> Self {
    Self {
      initializer,
      condition,
      increment,
      body,
      label,
    }
  }

  pub fn get_label(&self) -> &str {
    &self.label
  }
}

impl<StmtTy> Default<StmtTy> {
  pub fn new(body: Vec<StmtTy>) -> Self {
    Self { body }
  }
}

impl SingleLabel {
  pub fn new(name: String) -> Self {
    Self { name }
  }

  pub fn get_label(&self) -> &str {
    &self.name
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
  use std::fmt::Display;

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
        RawStmt::Break(_) => write!(f, "break;"),
        RawStmt::Continue(_) => write!(f, "continue;"),
      }
    }
  }

  impl<ExprTy: Display> Display for Return<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match &self.expression {
        Some(expr) => write!(f, "return {}", expr),
        None => write!(f, "return"),
      }
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for If<StmtTy, ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "if {} {}", self.condition, self.then_branch)?;
      if let Some(else_branch) = &self.else_branch {
        write!(f, " else {}", else_branch)?;
      }
      Ok(())
    }
  }

  impl<StmtTy: Display> Display for Compound<StmtTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{{\n")?;
      for stmt in &self.statements {
        write!(f, "  {}\n", stmt)?;
      }
      write!(f, "}}")
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for While<StmtTy, ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "while {} {}", self.condition, self.body)
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for DoWhile<StmtTy, ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "do {} while {}", self.body, self.condition)
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for For<StmtTy, ExprTy> {
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

  impl Display for SingleLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name)
    }
  }

  impl<StmtTy: Display> Display for Default<StmtTy> {
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

  impl<StmtTy: Display> Display for Label<StmtTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}: {}", self.name, self.statement)
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for Case<StmtTy, ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "case {}: {}",
        self.value,
        self
          .body
          .iter()
          .map(|s| format!("{}", s))
          .collect::<Vec<_>>()
          .join("\n")
      )
    }
  }

  impl<StmtTy: Display, ExprTy: Display> Display for Switch<StmtTy, ExprTy> {
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

  impl Display for Goto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "goto {};", self.label)
    }
  }
}
