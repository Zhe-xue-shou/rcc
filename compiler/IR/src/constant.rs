#![allow(unused)]

use ::rcc_adt::{Floating, Integral};
use ::rcc_utils::StrRef;

/// TODO: shold replace the old constant ALL PLACE NOT ONLY IR STAGE.
enum NewConstant<'c> {
  Nullptr(),
  Integral(Integral),
  Floating(Floating),
  Aggregate(Aggregate<'c>),
  Global(StrRef<'c>),
}
enum Aggregate<'c> {
  String(StrRef<'c>),
  Sequential(&'c [&'c NewConstant<'c>]),
}
