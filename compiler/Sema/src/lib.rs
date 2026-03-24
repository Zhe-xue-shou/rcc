pub mod declaration;
pub mod expression;
pub mod statement;

mod conversion;
mod folding;
mod semantics;

use ::rcc_utils::Unbox;

pub use self::semantics::Sema;

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
