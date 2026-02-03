use crate::{
  analyzer::{
    declaration::ExternalDeclaration,
    expression::{ConstantLiteral, Expression},
  },
  type_alias_stmt,
};
// no additional info like that one we do in Expression?
// alright, just repeat the same structure again -- stop abstracting here
#[derive(Debug)]
pub enum Statement {
  Empty(),
  Return(Return),
  Expression(Expression),
  Declaration(ExternalDeclaration),
  Compound(Compound),
  If(If),
  While(While),
  DoWhile(DoWhile),
  For(For),
  Switch(Switch),
  Goto(Goto),
  Label(Label),
  Break(Break),
  Continue(Continue),
}

type_alias_stmt!(Statement, ExternalDeclaration, Expression, ConstantLiteral);

impl ::std::default::Default for Statement {
  fn default() -> Self {
    Statement::Empty()
  }
}

mod fmt {
  use ::std::fmt::Display;

  use super::Statement;

  impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Statement::Empty() => write!(f, "<noop>"),
        Statement::Return(ret) => write!(f, "{}", ret),
        Statement::Expression(expr) => write!(f, "{};", expr),
        Statement::Declaration(decl) => write!(f, "{}", decl),
        Statement::Compound(compound) => write!(f, "{}", compound),
        Statement::If(if_stmt) => write!(f, "{}", if_stmt),
        Statement::While(while_stmt) => write!(f, "{}", while_stmt),
        Statement::DoWhile(do_while_stmt) => write!(f, "{}", do_while_stmt),
        Statement::For(for_stmt) => write!(f, "{}", for_stmt),
        Statement::Switch(switch_stmt) => write!(f, "{}", switch_stmt),
        Statement::Goto(goto_stmt) => write!(f, "{}", goto_stmt),
        Statement::Label(label_stmt) => write!(f, "{}", label_stmt),
        Statement::Break(break_stmt) => write!(f, "{}", break_stmt),
        Statement::Continue(continue_stmt) => write!(f, "{}", continue_stmt),
      }
    }
  }
}
