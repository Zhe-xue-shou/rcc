use ::rcc_utils::{SmallString, interconvert};

use super::{declaration::Declaration, expression::Expression};
use crate::blueprints::type_alias_stmt;

#[derive(Debug)]
pub enum Statement<'context> {
  Empty(Empty),
  Return(Return<'context>),
  Expression(Expression<'context>),
  Declaration(Declaration<'context>),
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
  Declaration<'context>,
  Expression<'context>
);
interconvert!(Declaration, Statement, 'context);
interconvert!(Expression, Statement, 'context);
interconvert!(Return, Statement, 'context);
interconvert!(Compound, Statement, 'context);
interconvert!(If, Statement, 'context);
interconvert!(While, Statement, 'context);
interconvert!(DoWhile, Statement, 'context);
interconvert!(For, Statement, 'context);
interconvert!(Switch, Statement, 'context);
interconvert!(Goto, Statement, 'context);
interconvert!(Label, Statement, 'context);
interconvert!(Break, Statement, 'context);
interconvert!(Continue, Statement, 'context);

impl<'context> Statement<'context> {
  pub fn new_loop_dummy_identifier(str: &'static str) -> SmallString {
    static LOOP_LABEL_COUNTER: ::std::sync::atomic::AtomicUsize =
      ::std::sync::atomic::AtomicUsize::new(0);
    let id =
      LOOP_LABEL_COUNTER.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
    SmallString::from(format!("{}_{}", str, id))
  }
}
impl<'context> ::std::default::Default for Statement<'context> {
  fn default() -> Self {
    Self::Empty(Empty::default())
  }
}
mod fmt {
  use ::rcc_utils::static_dispatch;
  use ::std::fmt::Display;

  use super::Statement;

  impl Display for Statement<'_> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      static_dispatch!(
        self.fmt(f),
        Empty Return Expression Declaration Compound If While DoWhile For Switch Goto
        Label Break Continue
      )
    }
  }
}
