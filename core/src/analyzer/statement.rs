use ::rc_utils::interconvert;

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
  Empty(Empty),
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
interconvert!(ExternalDeclaration, Statement, Declaration);
interconvert!(Expression, Statement);
interconvert!(Return, Statement);
interconvert!(Compound, Statement);
interconvert!(If, Statement);
interconvert!(While, Statement);
interconvert!(DoWhile, Statement);
interconvert!(For, Statement);
interconvert!(Switch, Statement);
interconvert!(Goto, Statement);
interconvert!(Label, Statement);
interconvert!(Break, Statement);
interconvert!(Continue, Statement);

impl ::std::default::Default for Statement {
  fn default() -> Self {
    Statement::Empty(Empty::default())
  }
}

mod fmt {
  use ::rc_utils::static_dispatch;
  use ::std::fmt::Display;

  use super::Statement;

  impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      static_dispatch!(
        self.fmt(f),
        Empty Return Expression Declaration Compound If While DoWhile For Switch Goto Label Break Continue
      )
    }
  }
}
