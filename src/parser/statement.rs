use crate::parser::declaration::Declaration;
use crate::parser::expression::Expression;

pub enum Statement {
  Empty(),
  Return(Return),
  If(If),
  // here only vardef, funcdef only permitted in top-level declarations hence it's handled there
  Declaration(Declaration),
  Expression(Expression),
  Compound(Compound),
  While(While),
  For(For),
  DoWhile(DoWhile),
  Switch(Switch),

  Case(Case),
  Label(Label),

  Default(Default),
  Break(SingleLabel),
  Continue(SingleLabel),
}
pub fn new_loop_dummy_identifier(str: &'static str) -> String {
  static LOOP_LABEL_COUNTER: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
  let id = LOOP_LABEL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  format!("{}_{}", str, id)
}
pub struct SingleLabel {
  pub label: String,
}

pub type Break = SingleLabel;
pub type Continue = SingleLabel;
pub struct Return {
  pub expression: Option<Expression>,
}

pub struct If {
  pub condition: Expression,
  pub if_branch: Box<Statement>,
  pub else_branch: Option<Box<Statement>>,
}

pub struct While {
  pub condition: Expression,
  pub body: Box<Statement>,
  label: String,
}
pub struct DoWhile {
  pub body: Box<Statement>,
  pub condition: Expression,
  label: String,
}

pub struct For {
  pub initializer: Option<Box<Statement>>,
  pub condition: Option<Expression>,
  pub increment: Option<Expression>,
  pub body: Box<Statement>,
  label: String,
}

pub struct Switch {
  pub expression: Expression,
  pub body: Box<Statement>, // Usually a Block
}

pub struct Case {
  pub value: Expression, // Must be constant integer expression
  pub body: Box<Statement>,
}

pub struct Default {
  pub body: Box<Statement>,
}

pub struct Label {
  pub name: String,
  pub statement: Box<Statement>,
}
impl SingleLabel {
  pub fn new(label: String) -> Self {
    Self { label }
  }
  pub fn get_label(&self) -> &str {
    &self.label
  }
}
impl Return {
  pub fn new(expression: Option<Expression>) -> Self {
    Self {
      expression: expression,
    }
  }
}

impl If {
  pub fn new(condition: Expression, if_branch: Statement, else_branch: Option<Statement>) -> Self {
    Self {
      condition,
      if_branch: Box::new(if_branch),
      else_branch: else_branch.map(Box::new),
    }
  }
}
pub struct Compound {
  pub statements: Vec<Statement>,
}

impl Compound {
  pub fn new() -> Self {
    Self {
      statements: Vec::new(),
    }
  }
}

impl While {
  pub fn new(condition: Expression, body: Statement, label: String) -> Self {
    Self {
      condition,
      body: Box::new(body),
      label,
    }
  }
  pub fn get_label(&self) -> &str {
    &self.label
  }
}
impl DoWhile {
  pub fn new(body: Statement, condition: Expression, label: String) -> Self {
    Self {
      body: Box::new(body),
      condition,
      label,
    }
  }
  pub fn get_label(&self) -> &str {
    &self.label
  }
}
impl For {
  pub fn new(
    initializer: Option<Statement>,
    condition: Option<Expression>,
    increment: Option<Expression>,
    body: Statement,
    label: String,
  ) -> Self {
    Self {
      initializer: initializer.map(Box::new),
      condition,
      increment,
      body: Box::new(body),
      label,
    }
  }
  pub fn get_label(&self) -> &str {
    &self.label
  }
}
mod fmt {
  use crate::parser::{
    declaration::Declaration,
    expression::Expression,
    statement::{Compound, DoWhile, For, If, Return, SingleLabel, Statement, While},
  };
  use std::fmt::{Debug, Display};

  impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Statement::Return(ret) => <Return as Display>::fmt(ret, f),
        Statement::If(if_stmt) => <If as Display>::fmt(if_stmt, f),
        Statement::Declaration(decl) => <Declaration as Display>::fmt(decl, f),
        Statement::Expression(expr) => <Expression as Display>::fmt(expr, f),
        Statement::Compound(c) => <Compound as Display>::fmt(c, f),
        Statement::Break(_) => write!(f, "break;"),
        Statement::Continue(_) => write!(f, "continue;"),
        Statement::Empty() => write!(f, ";"),
        Statement::While(while_stmt) => <While as Display>::fmt(while_stmt, f),
        Statement::DoWhile(dowhile_stmt) => <DoWhile as Display>::fmt(dowhile_stmt, f),
        Statement::For(for_stmt) => <For as Display>::fmt(for_stmt, f),
        _ => write!(f, "<unimplemented statement fmt>"),
      }
    }
  }

  impl Debug for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for Return {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match &self.expression {
        Some(expr) => write!(f, "return {}", expr),
        None => write!(f, "return"),
      }
    }
  }

  impl Debug for Return {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for If {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "if {} {}", self.condition, self.if_branch)?;
      if let Some(else_branch) = &self.else_branch {
        write!(f, " else {}", else_branch)?;
      }
      Ok(())
    }
  }

  impl Debug for If {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for Compound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{{\n")?;
      for stmt in &self.statements {
        write!(f, "  {}\n", stmt)?;
      }
      write!(f, "}}")
    }
  }

  impl Debug for Compound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for While {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "while {} {}", self.condition, self.body)
    }
  }
  impl Debug for While {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for DoWhile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "do {} while {}", self.body, self.condition)
    }
  }
  impl Debug for DoWhile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for For {
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
  impl Debug for For {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for SingleLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.label)
    }
  }
  impl Debug for SingleLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
}
