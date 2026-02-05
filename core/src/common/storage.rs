use ::rc_utils::IntoWith;

use super::{Keyword, Literal};
use crate::diagnosis::{DiagData, DiagMeta, Severity};

/// storage-class-specifier
#[derive(Debug, ::strum_macros::Display, PartialEq, Eq, Clone, Copy)]
pub enum Storage {
  /// variables that declared in block scope without any storage-class specifier
  /// are considered to have automatic storage duration.
  #[strum(serialize = "auto")]
  Automatic,
  #[strum(serialize = "register")]
  Register,
  /// - Function declarations with no storage-class specifier are always handled
  ///   as though they include an extern specifier
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
  ///   has internal linkage.
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

use Storage::*;

impl From<&Keyword> for Storage {
  fn from(kw: &Keyword) -> Self {
    match kw {
      Keyword::Auto => Automatic,
      Keyword::Register => Register,
      Keyword::Extern => Extern,
      Keyword::Static => Static,
      Keyword::Typedef => Typedef,
      Keyword::ThreadLocal => ThreadLocal,
      Keyword::Constexpr => Constexpr,
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
  pub fn try_merge(lhs: &Storage, rhs: &Storage) -> Result<Storage, DiagMeta> {
    match (lhs, rhs) {
      (lhs, rhs) if lhs == rhs => Ok(*lhs),
      (Constexpr, _) | (_, Constexpr) => Err(
        DiagData::UnsupportedFeature("Constexpr unimplemented yet".to_string())
          .into_with(Severity::Error),
      ),
      (Typedef, _) | (_, Typedef) => Err(
        DiagData::StorageSpecsUnmergeable(*lhs, *rhs)
          .into_with(Severity::Error),
      ),
      (Extern, other) | (other, Extern) => Ok(*other), // extern is compatible with any other storage class
      _ => Err(
        DiagData::StorageSpecsUnmergeable(*lhs, *rhs)
          .into_with(Severity::Error),
      ),
    }
  }

  #[inline]
  pub fn is_static(&self) -> bool {
    matches!(self, Static)
  }

  #[inline]
  pub fn is_extern(&self) -> bool {
    matches!(self, Extern)
  }

  #[inline]
  pub fn is_thread_local(&self) -> bool {
    matches!(self, ThreadLocal)
  }

  #[inline]
  pub fn is_constexpr(&self) -> bool {
    matches!(self, Constexpr)
  }

  #[inline]
  pub fn is_typedef(&self) -> bool {
    matches!(self, Typedef)
  }

  #[inline]
  pub fn is_automatic(&self) -> bool {
    matches!(self, Automatic)
  }

  #[inline]
  pub fn is_register(&self) -> bool {
    matches!(self, Register)
  }
}

impl PartialEq<Storage> for &Storage {
  fn eq(&self, other: &Storage) -> bool {
    **self == *other
  }
}
