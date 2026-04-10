#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(negative_impls)]
#![feature(const_trait_impl)]
#![feature(const_destruct)]
#![feature(const_cmp)]
#![feature(const_ops)]
#![feature(const_type_name)]
#![feature(const_eval_select)]
#![feature(likely_unlikely)]
mod concat;
mod macros;
mod num_traits;
mod opaque;

mod fwd {
  pub use ::paste::paste;
  pub type SmallString = ::compact_str::CompactString;
}
pub use self::{concat::StaticStr, fwd::*, num_traits::*, opaque::Opaque};

/// A handy trait for converting between types with additional context.
pub trait IntoWith<With, To> {
  fn into_with(self, with: With) -> To;
}
pub trait DisplayWith<'a, With, To: ::std::fmt::Display> {
  fn display_with(&'a self, with: &'a With) -> To;
}
pub trait FromWith<With, From>: Sized {
  fn from_with(from: From, with: With) -> Self;
}
pub trait TryFromWith<With, From>: Sized {
  type Error;
  fn try_from_with(from: From, with: With) -> Result<Self, Self::Error>;
}

pub type StrRef<'c> = &'c str;
// impl RefEq for StrRef<'_> {}

/// Intern helper trait to both use [`::std::ptr::eq`] but also [`debug_assert`]
/// the actual equality of the two values in debug mode, to catch potential bugs
/// where two different instances with the same content are compared by pointer address.
///
/// ### Important Note
/// Should never impl this [`RefEq`] w.r.t. ref-type, such as `&'a MyRef`,
/// otherwise it would cause double reference problem:
/// i.e., comparing `&&MyRef` which probably resides on the stack;
/// even if both points the same object,
/// the temporary variable would always have different address.
///
/// For example, if you found two addresses are exactly 8 bytes apart,
/// which is the size of a pointer on a 64-bit architecture -- this prob means
/// you are accidentally comparing the stack addresses of the local variables.
///
/// ### [`&str`](StrRef) is an exception
/// since it's a fat pointer and comparing by pointer address is actually
/// comparing the content of the string, and thus is (probably) safe.
///
/// ### Negative impls for `&T` and `&mut T` has not yet finished
/// Rust's template specialization and negative impls are fairly limited
/// and incomplete, i havent devise up an method to combine them all.
pub trait RefEq {
  #[inline]
  #[must_use]
  fn ref_eq(lhs: &Self, rhs: &Self) -> bool
  where
    Self: PartialEq + ::std::fmt::Debug,
  {
    Self::ref_eq_impl(lhs, rhs, "")
  }
  fn ref_eq_impl(lhs: &Self, rhs: &Self, msg: &'static str) -> bool
  where
    Self: PartialEq + ::std::fmt::Debug,
  {
    let ref_eq = ::std::ptr::eq(lhs, rhs);
    if const { cfg!(debug_assertions) } {
      let actual_eq = lhs == rhs;
      if ref_eq != actual_eq {
        eprintln!(
          "INTERNAL ERROR: comparing by pointer address result did not match 
          the actual result: {:p}: {:?} and {:p}: {:?}. {}\n If you find the \
           actual addresses are offseted by n * sizeof(void*) -- then see \
           docs of `RefEq` for more information.
        ",
          lhs, lhs, rhs, rhs, msg
        );
      }
      return actual_eq;
    }
    ref_eq
  }
}
impl<T: ?Sized> !RefEq for &T {}
impl<T: ?Sized> !RefEq for &mut T {}

pub trait PtrEq {
  #[inline(always)]
  #[must_use]
  fn ptr_eq(lhs: &Self, rhs: &Self) -> bool {
    ::std::ptr::eq(lhs, rhs)
  }
}
impl<T: ?Sized> !PtrEq for &T {}
impl<T: ?Sized> !PtrEq for &mut T {}

/// internal implementation used for [`const_assert`] and [`const_assert_eq`].
///
/// # Invoke it directly is wrong.
#[track_caller]
#[inline(always)]
pub const fn _static_assert_impl_(cond: bool, _: &str) {
  assert!(cond, "static assertion failed");
}
/// internal implementation used for [`const_assert`] and [`const_assert_eq`].
///
/// # Invoke it directly has no additional effect than [`debug_assert`].
#[track_caller]
#[inline(always)]
pub fn _debug_assertion_impl_(cond: bool, msg: &str) {
  debug_assert!(cond, "debug assertion failed: {}", msg);
}

use ::std::{fmt::Debug, marker::Destruct, ops::Add};
#[allow(non_camel_case_types)]
pub struct const_pre;

impl<T: [const] PartialEq + [const] Destruct + Debug> const Add<(T, T)>
  for const_pre
{
  type Output = ();

  #[inline(always)]
  fn add(self, (lhs, rhs): (T, T)) {
    const_assert_eq!(lhs, rhs)
  }
}
impl<T: [const] PartialEq + [const] Destruct + Debug> const Add<(T, T, &str)>
  for const_pre
{
  type Output = ();

  #[inline(always)]
  fn add(self, (lhs, rhs, msg): (T, T, &str)) {
    const_assert_eq!(lhs, rhs, msg)
  }
}

/// exists just to avoid polluting unstable feature switches in the rest of the codebase.
#[inline(always)]
pub fn likely(b: bool) -> bool {
  ::std::hint::likely(b)
}

/// exists just to avoid polluting unstable feature switches in the rest of the codebase.
#[inline(always)]
pub fn unlikely(b: bool) -> bool {
  ::std::hint::unlikely(b)
}
