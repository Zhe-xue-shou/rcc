use crate::parser::ast::Block;
use crate::parser::expression::Expression;

pub enum Statement {
  Return(Return),
  If(If),
  Declaration(VarDef),
  Expression(Expression),
}

pub struct Return {
  pub(crate) expression: Option<Expression>,
}

pub struct If {
  pub(crate) condition: Expression,
  pub(crate) if_branch: Block,
  pub(crate) else_branch: Block,
}

pub struct VarDef {
  pub(crate) name: String,
  pub(crate) initializer: Option<Expression>,
  // type: QualifiedType,
}

impl Return {
  pub fn new(expression: Option<Expression>) -> Self {
    Self {
      expression: expression,
    }
  }
}
impl VarDef {
  pub fn new(name: String, initializer: Option<Expression>) -> Self {
    Self { name, initializer }
  }
}
