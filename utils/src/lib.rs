#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(negative_impls)]
#![feature(const_trait_impl)]
#![feature(const_destruct)]
#![feature(const_cmp)]
#![feature(const_ops)]
#![feature(const_type_name)]
#![feature(const_eval_select)]
mod macros;
mod num_traits;

use ::std::{cell::RefCell, rc::Rc};

pub use self::num_traits::*;

pub type SmallString = ::compact_str::CompactString;

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
/// A trait for creating dummy instances of types during testing.
///
/// This is useful for situations where a placeholder value is needed,
/// such as during testing or when initializing data structures,
/// but their actual values do not matter.
///
/// The difference between this and the [`Default`] trait is that Dummy
/// instances are often invalid or nonsensical in a real context,
/// whereas [`Default`] instances are expected to be valid and meaningful.
///
/// In other words, [`Dummy`] targets for ppl who read and write the code.
///
/// Why not use [`Option<T>`] or [`Result<T, E>`]? -- there's no point
/// to wrap every single type in Option or Result just cater for unittest.
#[cfg(debug_assertions)]
pub trait Dummy {
  fn dummy() -> Self;
}
#[cfg(debug_assertions)]
impl<T: Dummy> Dummy for Rc<RefCell<T>> {
  fn dummy() -> Self {
    Rc::new(RefCell::new(T::dummy()))
  }
}

#[cfg(debug_assertions)]
impl Dummy for u32 {
  #[inline]
  fn dummy() -> Self {
    u32::MAX
  }
}
#[cfg(debug_assertions)]
impl Dummy for usize {
  #[inline]
  fn dummy() -> Self {
    usize::MAX
  }
}

pub type StrRef<'c> = &'c str;
impl RefEq for StrRef<'_> {}

/// Intern helper trait to both use [`::std::ptr::eq`] but also [`debug_assert`]
/// the actual equality of the two values in debug mode, to catch potential bugs
/// where two different instances with the same content are compared by pointer address.
///
/// # Important Note
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
// impl<T: ?Sized> !RefEq for &T {}
impl<T: ?Sized> !RefEq for &mut T {}

#[track_caller]
#[inline]
pub const fn static_assert(cond: bool, _: &str) {
  assert!(cond, "static assertion failed");
}
#[track_caller]
#[inline]
pub fn debug_assertion(cond: bool, msg: &str) {
  debug_assert!(cond, "debug assertion failed: {}", msg);
}

use ::std::{fmt::Debug, marker::Destruct, ops::Add};
#[allow(non_camel_case_types)]
pub struct const_pre;

impl<T: [const] PartialEq + [const] Destruct + Debug> const Add<(T, T)>
  for const_pre
{
  type Output = ();

  fn add(self, (lhs, rhs): (T, T)) {
    const_assert_eq!(lhs, rhs)
  }
}
impl<T: [const] PartialEq + [const] Destruct + Debug> const Add<(T, T, &str)>
  for const_pre
{
  type Output = ();

  fn add(self, (lhs, rhs, msg): (T, T, &str)) {
    const_assert_eq!(lhs, rhs, msg)
  }
}

mod opaque;
pub use opaque::Opaque;
