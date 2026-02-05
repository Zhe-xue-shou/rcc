use ::rc_utils::interconvert;

use super::{declaration::Declaration, expression::Expression};
use crate::type_alias_stmt;

#[derive(Debug)]
pub enum Statement {
  Empty(Empty),
  Return(Return),
  Expression(Expression),
  Declaration(Declaration),
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

type_alias_stmt!(Statement, Declaration, Expression);
interconvert!(Declaration, Statement);
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

impl Statement {
  pub(super) fn new_loop_dummy_identifier(str: &'static str) -> String {
    RawStmt::new_loop_dummy_identifier(str)
  }
}
impl ::std::default::Default for Statement {
  fn default() -> Self {
    Self::Empty(Empty::default())
  }
}
mod fmt {
  use ::rc_utils::static_dispatch;
  use ::std::fmt::Display;

  use super::Statement;

  impl Display for Statement {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      static_dispatch!(
        self.fmt(f),
        Empty Return Expression Declaration Compound If While DoWhile For Switch Goto
        Label Break Continue
      )
    }
  }
}
