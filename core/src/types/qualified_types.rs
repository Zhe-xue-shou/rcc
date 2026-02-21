use ::rcc_utils::{IntoWith, ensure_is_pod};

use super::{Type, TypeRef};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct QualifiedType<'context> {
  pub qualifiers: Qualifiers,
  pub unqualified_type: TypeRef<'context>,
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

impl<'context> QualifiedType<'context> {
  pub const fn new(
    qualifiers: Qualifiers,
    unqualified_type: TypeRef<'context>,
  ) -> Self {
    Self {
      qualifiers,
      unqualified_type,
    }
  }

  pub const fn new_unqualified(unqualified_type: TypeRef<'context>) -> Self {
    Self {
      qualifiers: Qualifiers::empty(),
      unqualified_type,
    }
  }
}
impl<'context> ::std::ops::Deref for QualifiedType<'context> {
  type Target = TypeRef<'context>;

  fn deref(&self) -> &Self::Target {
    &self.unqualified_type
  }
}

impl<'context> QualifiedType<'context> {
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

  pub fn destructure(self) -> (Qualifiers, TypeRef<'context>) {
    (self.qualifiers, self.unqualified_type)
  }
}
impl<'context> const From<TypeRef<'context>> for QualifiedType<'context> {
  #[inline]
  fn from(value: TypeRef<'context>) -> Self {
    Self {
      qualifiers: Qualifiers::empty(),
      unqualified_type: value,
    }
  }
}

impl<'context> From<&'context mut Type<'context>> for QualifiedType<'context> {
  #[inline(always)]
  fn from(inner: &'context mut Type<'context>) -> Self {
    Self::new_unqualified(inner)
  }
}
impl<'a> IntoWith<Qualifiers, QualifiedType<'a>> for TypeRef<'a> {
  fn into_with(self, with: Qualifiers) -> QualifiedType<'a> {
    QualifiedType::new(with, self)
  }
}
