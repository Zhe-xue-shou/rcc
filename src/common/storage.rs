use strum_macros::Display;

use crate::common::{error::Error, keyword::Keyword, token::Literal};

/// storage-class-specifier
#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum Storage {
  /// variables that declared in block scope without any storage-class specifier
  /// are considered to have automatic storage duration.
  #[strum(serialize = "auto")]
  Automatic,
  #[strum(serialize = "register")]
  Register,
  /// - Function declarations with no storage-class specifier are always handled
  ///     as though they include an extern specifier
  /// - if variable declarations appear at file scope, they have external linkage
  /// - use extern to declare an identifier that’s already visible.
  /// ```c
  /// static int a;
  /// extern int a; // this is valid and a has internal linkage
  /// extern int b;
  /// static int b = 0; // this is also valid... (internal linkage)
  /// ```
  #[strum(serialize = "extern")]
  Extern,
  /// - At file scope, the static specifier indicates that a function or variable
  ///     has internal linkage.
  /// - At block scope(i.e., for variables), the static specifier controls storage duration, not linkage.
  #[strum(serialize = "static")]
  Static,
  /// according to standard, `typedef` is categorized as a storage-class specifier for **syntactic convenience only**.
  #[strum(serialize = "typedef")]
  Typedef,
  /// the variable is allocated when the thread is created
  #[strum(serialize = "thread_local")]
  ThreadLocal, // I won't care about this now
  /// C23, `#define VAR value` is the same `constexpr TYPE VAR = value;` with fewer name collisions
  #[strum(serialize = "constexpr")]
  Constexpr, // ditto
}

impl From<&Keyword> for Storage {
  fn from(kw: &Keyword) -> Self {
    match kw {
      Keyword::Auto => Storage::Automatic,
      Keyword::Register => Storage::Register,
      Keyword::Extern => Storage::Extern,
      Keyword::Static => Storage::Static,
      Keyword::Typedef => Storage::Typedef,
      Keyword::ThreadLocal => Storage::ThreadLocal,
      Keyword::Constexpr => Storage::Constexpr,
      _ => panic!("cannot convert {:?} to Storage", kw),
    }
  }
}

impl From<&Literal> for Storage {
  fn from(literal: &Literal) -> Self {
    match literal {
      Literal::Keyword(kw) => Storage::from(kw),
      _ => panic!("cannot convert {:?} to Storage", literal),
    }
  }
}

impl Storage {
  pub fn try_merge(lhs: &Storage, rhs: &Storage) -> Result<Storage, Error> {
    match (lhs, rhs) {
      (lhs, rhs) if lhs == rhs => Ok(lhs.clone()),
      (Storage::Constexpr, _) | (_, Storage::Constexpr) => Err(()), // unimplemented
      (Storage::Typedef, _) | (_, Storage::Typedef) => Err(()), // unmergeable
      (Storage::Extern, other) | (other, Storage::Extern) => Ok(other.clone()), // extern is compatible with any other storage class
      _ => Err(()),
    }
  }

  // pub fn try_merge_with_funcspecs(
  //   lhs: (
  //     Option<&Storage>,
  //     &FunctionSpecifier,
  //     bool, /* is definition */
  //   ),
  //   rhs: (
  //     Option<&Storage>,
  //     &FunctionSpecifier,
  //     bool, /* is definition */
  //   ),
  // ) -> Result<(Storage, FunctionSpecifier), Error> {
  //   assert_eq!(
  //     lhs.2 && rhs.2,
  //     false,
  //     "both cannot be definitions and should be handled before calling this"
  //   );
  //   type FS = FunctionSpecifier;
  //   let merged_storage = match (lhs.0, rhs.0) {
  //     (Some(l), Some(r)) => Some(Storage::try_merge(l, r)?),
  //     (Some(l), None) => Some(l.clone()),
  //     (None, Some(r)) => Some(r.clone()),
  //     (None, None) => None,
  //   };
  //   let merged_funcspecs = *lhs.1 | *rhs.1;
  //   match (merged_storage, merged_funcspecs) {
  //     (None, funcspecs) => Ok((Storage::Extern, funcspecs)), // default storage class for functions is extern
  //     (Some(storage), funcspecs) => Ok((storage, funcspecs)),
  //   }
  // }
}
