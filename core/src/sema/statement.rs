use ::rcc_utils::interconvert;

use super::{
  declaration::ExternalDeclaration,
  expression::{ConstantLiteral, Expression},
};
use crate::blueprints::type_alias_stmt;
// no additional info like that one we do in Expression?
// alright, just repeat the same structure again -- stop abstracting here
#[derive(Debug)]
pub enum Statement<'context> {
  Empty(Empty),
  Return(Return<'context>),
  Expression(Expression<'context>),
  Declaration(ExternalDeclaration<'context>),
  Compound(Compound<'context>),
  If(If<'context>),
  While(While<'context>),
  DoWhile(DoWhile<'context>),
  For(For<'context>),
  Switch(Switch<'context>),
  Goto(Goto<'context>),
  Label(Label<'context>),
  Break(Break<'context>),
  Continue(Continue<'context>),
}

type_alias_stmt!(
  Statement<'context>,
  ExternalDeclaration<'context>,
  Expression<'context>,
  ConstantLiteral<'context>
);
interconvert!(ExternalDeclaration, Statement, 'context, Declaration);
interconvert!(Expression, Statement,'context);
interconvert!(Return, Statement,'context);
interconvert!(Compound, Statement,'context);
interconvert!(If, Statement,'context);
interconvert!(While, Statement,'context);
interconvert!(DoWhile, Statement,'context);
interconvert!(For, Statement,'context);
interconvert!(Switch, Statement,'context);
interconvert!(Goto, Statement,'context);
interconvert!(Label, Statement,'context);
interconvert!(Break, Statement,'context);
interconvert!(Continue, Statement,'context);

impl<'context> ::std::default::Default for Statement<'context> {
  fn default() -> Self {
    Statement::Empty(Empty::default())
  }
}

mod fmt {
  use ::rcc_utils::static_dispatch;
  use ::std::fmt::Display;

  use super::Statement;

  impl<'context> Display for Statement<'context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      static_dispatch!(
        self.fmt(f),
        Empty Return Expression Declaration Compound If While DoWhile For Switch Goto Label Break Continue
      )
    }
  }
}
