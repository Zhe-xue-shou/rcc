use ::rcc_utils::interconvert;

use super::{
  declaration::ExternalDeclaration,
  expression::{ConstantLiteral, Expression},
};
use crate::blueprints::type_alias_stmt;
// no additional info like that one we do in Expression?
// alright, just repeat the same structure again -- stop abstracting here
#[derive(Debug)]
pub enum Statement<'c> {
  Empty(Empty),
  Return(Return<'c>),
  Expression(Expression<'c>),
  Declaration(ExternalDeclaration<'c>),
  Compound(Compound<'c>),
  If(If<'c>),
  While(While<'c>),
  DoWhile(DoWhile<'c>),
  For(For<'c>),
  Switch(Switch<'c>),
  Goto(Goto<'c>),
  Label(Label<'c>),
  Break(Break<'c>),
  Continue(Continue<'c>),
}

type_alias_stmt!(
  Statement<'c>,
  ExternalDeclaration<'c>,
  Expression<'c>,
  ConstantLiteral<'c>
);
interconvert!(ExternalDeclaration, Statement, 'c, Declaration);
interconvert!(Expression, Statement,'c);
interconvert!(Return, Statement,'c);
interconvert!(Compound, Statement,'c);
interconvert!(If, Statement,'c);
interconvert!(While, Statement,'c);
interconvert!(DoWhile, Statement,'c);
interconvert!(For, Statement,'c);
interconvert!(Switch, Statement,'c);
interconvert!(Goto, Statement,'c);
interconvert!(Label, Statement,'c);
interconvert!(Break, Statement,'c);
interconvert!(Continue, Statement,'c);

impl<'c> ::std::default::Default for Statement<'c> {
  fn default() -> Self {
    Statement::Empty(Empty::default())
  }
}

mod fmt {
  use ::std::fmt::Display;

  use super::Statement;

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
