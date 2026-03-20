pub mod declaration;
pub mod expression;
pub mod statement;

mod conversion;
mod dump;
mod folding;
mod semantics;
mod testing;

pub use self::{
  dump::ASTDumper,
  folding::{Folding, FoldingResult},
  semantics::Sema,
};

pub trait Unbox {
  type Output;
  #[must_use]
  fn unbox(self) -> Self::Output
  where
    Self::Output: Unbox<Output = Self::Output>;
}
// impl<T> Unbox for T {
//   default type Output = T;

//   #[inline(always)]
//   default fn unbox(self) -> Self::Output {
//     self
//     // unsafe {
//     //   let result = ::std::ptr::read
//     //     (&self as *const _ as *const Self::Output);
//     //   ::std::mem::forget(self);
//     //   result
//     // }
//   }
// }
impl<T> Unbox for Box<T> {
  type Output = T;

  #[inline(always)]
  fn unbox(self) -> Self::Output
  where
    Self::Output: Unbox<Output = Self::Output>,
  {
    *self
  }
}
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

// impl<'c> Unbox for Box<expression::Expression<'c>> {
//   type Output = expression::Expression<'c>;

//   #[inline(always)]
//   fn unbox(self) -> Self::Output {
//     *self
//   }
// }

// impl<'c> Unbox for Box<statement::Statement<'c>> {
//   type Output = statement::Statement<'c>;

//   fn unbox(self) -> Self::Output {
//     *self
//   }
// }
