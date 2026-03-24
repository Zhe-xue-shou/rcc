#![feature(adt_const_params)]

pub mod declaration;
pub mod expression;
mod parser;
pub mod statement;

use ::rcc_utils::Unbox;

pub use self::parser::Parser;

impl Unbox for expression::Expression<'_> {
  type Output = Self;

  #[inline(always)]
  fn unbox(self) -> Self::Output
  where
    Self::Output: Unbox<Output = Self::Output>,
  {
    self
  }
}
impl Unbox for statement::Statement<'_> {
  type Output = Self;

  #[inline(always)]
  fn unbox(self) -> Self::Output
  where
    Self::Output: Unbox<Output = Self::Output>,
  {
    self
  }
}
