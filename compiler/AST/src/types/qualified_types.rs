use ::rcc_shared::{Keyword, Literal};
use ::rcc_utils::{IntoWith, RefEq, ensure_is_pod};

use super::{TypeRef, TypeRefMut};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct QualifiedType<'c> {
  pub qualifiers: Qualifiers,
  pub unqualified_type: TypeRef<'c>,
}

ensure_is_pod!(QualifiedType);

::bitflags::bitflags! {
/// type-specifier-qualifier:
/// -    type-specifier
/// -    type-qualifier
/// -    alignment-specifier (don't care)
///
/// specifier would be merged into `Type` directly, so here only have qualifiers
  #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
  pub struct Qualifiers: u8 {
    const Const = 0x01;
    const Volatile = 0x02;
    const Restrict = 0x04;
    const Atomic = 0x08; // ignore for now
  }
}
::bitflags::bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub struct FunctionSpecifier : u8 {
    const Inline = 0x01;
    const Noreturn = 0x10;
  }
}

impl<'c> QualifiedType<'c> {
  pub const fn new(
    qualifiers: Qualifiers,
    unqualified_type: TypeRef<'c>,
  ) -> Self {
    Self {
      qualifiers,
      unqualified_type,
    }
  }

  pub const fn new_unqualified(unqualified_type: TypeRef<'c>) -> Self {
    Self {
      qualifiers: Qualifiers::empty(),
      unqualified_type,
    }
  }
}
impl<'c> ::std::ops::Deref for QualifiedType<'c> {
  type Target = TypeRef<'c>;

  fn deref(&self) -> &Self::Target {
    &self.unqualified_type
  }
}

impl<'c> QualifiedType<'c> {
  pub fn with_qualifiers(mut self, qualifiers: Qualifiers) -> Self {
    self.qualifiers |= qualifiers;
    self
  }

  pub fn is_modifiable(&self) -> bool {
    self.unqualified_type.is_modifiable()
      && !self.qualifiers.contains(Qualifiers::Const)
  }

  pub fn is_void(&self) -> bool {
    self.unqualified_type.is_void()
  }

  pub fn destructure(self) -> (Qualifiers, TypeRef<'c>) {
    (self.qualifiers, self.unqualified_type)
  }

  pub fn ref_eq_same(lhs: &QualifiedType, rhs: &QualifiedType) -> bool {
    RefEq::ref_eq(lhs.unqualified_type, rhs.unqualified_type)
      && lhs.qualifiers == rhs.qualifiers
  }
}
impl<'c> From<TypeRef<'c>> for QualifiedType<'c> {
  #[inline]
  fn from(value: TypeRef<'c>) -> Self {
    Self {
      qualifiers: Qualifiers::empty(),
      unqualified_type: value,
    }
  }
}

impl<'c> From<TypeRefMut<'c>> for QualifiedType<'c> {
  #[inline(always)]
  fn from(inner: TypeRefMut<'c>) -> Self {
    Self::new_unqualified(inner)
  }
}
impl<'a> IntoWith<Qualifiers, QualifiedType<'a>> for TypeRef<'a> {
  fn into_with(self, with: Qualifiers) -> QualifiedType<'a> {
    QualifiedType::new(with, self)
  }
}

impl<'c> From<&Literal<'c>> for Qualifiers {
  fn from(literal: &Literal) -> Self {
    match literal {
      Literal::Keyword(kw) => match kw {
        Keyword::Const => Qualifiers::Const,
        Keyword::Volatile => Qualifiers::Volatile,
        Keyword::Restrict => Qualifiers::Restrict,
        Keyword::Atomic => Qualifiers::Atomic,
        _ => panic!("cannot convert {:?} to Qualifier", kw),
      },
      _ => panic!("cannot convert {:?} to Qualifier", literal),
    }
  }
}
impl TryFrom<&Keyword> for FunctionSpecifier {
  type Error = ();

  fn try_from(kw: &Keyword) -> Result<Self, Self::Error> {
    match kw {
      Keyword::Inline => Ok(FunctionSpecifier::Inline),
      Keyword::Noreturn => Ok(FunctionSpecifier::Noreturn),
      _ => Err(()),
    }
  }
}

impl<'c> TryFrom<&Literal<'c>> for FunctionSpecifier {
  type Error = ();

  fn try_from(literal: &Literal) -> Result<Self, Self::Error> {
    match literal {
      Literal::Keyword(kw) => FunctionSpecifier::try_from(kw),
      _ => Err(()),
    }
  }
}
