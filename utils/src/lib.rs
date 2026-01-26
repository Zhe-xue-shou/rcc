mod macros;

use ::std::{
  cell::RefCell,
  rc::{Rc, Weak},
};

pub type SmallString = compact_str::CompactString;
/// as someone who came from C++, I'd more prefer to call it shared_ptr rather than Rc/RefCell or whatever. :p
#[allow(non_camel_case_types)]
pub type shared_ptr<T> = Rc<RefCell<T>>;
#[allow(non_camel_case_types)]
pub type weak_ptr<T> = Weak<RefCell<T>>;

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
/// A trait for creating dummy instances of types.
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
/// to wrap every single type in Option or Result just to represent a dummy value.
///
/// Notable use case from myself:
/// - use [`Dummy`] to create placeholder
/// expressions/statements (just returns an empty node)
/// to indicate errors during semantic analysis.
/// - [`SourceSpan`] to represent an invalid source location,
/// ususally for unconstructed spans/nodes, or representing a error node's location.
///
pub trait Dummy {
  fn dummy() -> Self;
}
impl<T: Dummy> Dummy for shared_ptr<T> {
  fn dummy() -> Self {
    Rc::new(RefCell::new(T::dummy()))
  }
}

impl Dummy for u32 {
  #[inline]
  fn dummy() -> Self {
    u32::MAX
  }
}
impl Dummy for usize {
  #[inline]
  fn dummy() -> Self {
    usize::MAX
  }
}
