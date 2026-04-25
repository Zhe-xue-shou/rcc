use ::rcc_utils::StrRef;

use crate::{
    ConstantData as Data,
    global::{Function, Variable},
};
#[derive(Debug)]
pub enum Global<'ir> {
  Function(Function<'ir>),
  Variable(Variable<'ir>),
}

impl<'ir> Global<'ir> {
  pub fn name(&self) -> StrRef<'ir> {
    ::rcc_utils::static_dispatch!(
      Global : self,
      |variant| variant.name =>
      Function Variable
    )
  }
}

#[derive(Debug)]
pub enum Constant<'ir> {
  /// (mostly) immediate.
  Data(Data<'ir>),
  /// Address constant.
  Global(Global<'ir>),
}

mod cvt {
  use ::rcc_utils::{interconvert, make_trio_for};

  use super::*;
  interconvert!(Function, Global,'ir);
  interconvert!(Variable, Global,'ir);
  make_trio_for!(Function, Global, 'ir);
  make_trio_for!(Variable, Global, 'ir);

  interconvert!(Data, Constant,'ir);
  interconvert!(Global, Constant,'ir);
  make_trio_for!(Data, Constant, 'ir);
  make_trio_for!(Global, Constant, 'ir);
}

mod fmt {
  use ::std::fmt::{Display, Formatter, Result};

  use super::*;
  impl<'ir> Display for Constant<'ir> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
      ::rcc_utils::static_dispatch!(
        Constant : self,
        |variant| Display::fmt(variant, f) =>
        Data Global
      )
    }
  }

  impl<'ir> Display for Global<'ir> {
    fn fmt(&self, _f: &mut Formatter<'_>) -> Result {
      unreachable!("when is it possible to reach here?")
    }
  }
}
