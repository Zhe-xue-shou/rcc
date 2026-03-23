use ::rcc_ast::type_alias_stmt;
use ::rcc_utils::interconvert;

use super::{declaration::Declaration, expression::Expression};

#[derive(Debug)]
pub enum Statement<'c> {
  Empty(Empty),
  Return(Return<'c>),
  Expression(Expression<'c>),
  Declaration(Declaration<'c>),
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

type_alias_stmt!(Statement<'c>, Declaration<'c>, Expression<'c>);
interconvert!(Declaration, Statement, 'c);
interconvert!(Expression, Statement, 'c);
interconvert!(Return, Statement, 'c);
interconvert!(Compound, Statement, 'c);
interconvert!(If, Statement, 'c);
interconvert!(While, Statement, 'c);
interconvert!(DoWhile, Statement, 'c);
interconvert!(For, Statement, 'c);
interconvert!(Switch, Statement, 'c);
interconvert!(Goto, Statement, 'c);
interconvert!(Label, Statement, 'c);
interconvert!(Break, Statement, 'c);
interconvert!(Continue, Statement, 'c);

impl<'c> ::std::default::Default for Statement<'c> {
  fn default() -> Self {
    Self::Empty(Empty::default())
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::Statement;

  impl Display for Statement<'_> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      ::rcc_utils::static_dispatch!(
        self,
        |variant| variant.fmt(f) =>
        Empty Return Expression Declaration Compound If While DoWhile For Switch Goto
        Label Break Continue
      )
    }
  }
}
