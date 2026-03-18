use ::rcc_utils::{SmallString, interconvert};

use super::{declaration::Declaration, expression::Expression};
use crate::blueprints::type_alias_stmt;

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

type_alias_stmt!(
  Statement<'c>,
  Declaration<'c>,
  Expression<'c>
);
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

impl<'c> Statement<'c> {
  pub fn new_loop_dummy_identifier(str: &'static str) -> SmallString {
    static LOOP_LABEL_COUNTER: ::std::sync::atomic::AtomicUsize =
      ::std::sync::atomic::AtomicUsize::new(0);
    let id =
      LOOP_LABEL_COUNTER.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
    SmallString::from(format!("{}_{}", str, id))
  }
}
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
