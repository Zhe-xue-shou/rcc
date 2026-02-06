use ::std::rc::Rc;

use super::Type;

#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedType {
  qualifiers: Qualifiers,
  unqualified_type: Rc<Type>,
}

::bitflags::bitflags! {
/// type-specifier-qualifier:
/// -    type-specifier
/// -    type-qualifier
/// -    alignment-specifier (don't care)
///
/// specifier would be merged into `Type` directly, so here only have qualifiers
  #[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
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

impl QualifiedType {
  pub const fn new(qualifiers: Qualifiers, unqualified_type: Rc<Type>) -> Self {
    Self {
      qualifiers,
      unqualified_type,
    }
  }

  pub const fn new_unqualified(unqualified_type: Rc<Type>) -> Self {
    Self {
      qualifiers: Qualifiers::empty(),
      unqualified_type,
    }
  }

  pub fn void() -> Self {
    Self::new_unqualified(Type::void().into())
  }

  /// Currently it returns an [`Primitive::Int`] instead of [`Primitive::Bool`].
  pub fn bool_type() -> Self {
    Self::new_unqualified(Type::bool_type().into())
  }

  pub fn int() -> Self {
    Self::new_unqualified(Type::int().into())
  }

  pub fn float() -> Self {
    Self::new_unqualified(Type::float().into())
  }

  pub fn nullptr() -> Self {
    Self::new_unqualified(Type::nullptr().into())
  }

  pub fn uintptr() -> Self {
    Self::new_unqualified(Type::uintptr().into())
  }

  pub fn ptrdiff() -> Self {
    Self::new_unqualified(Type::ptrdiff().into())
  }

  pub fn char() -> Self {
    Self::new_unqualified(Type::char().into())
  }
}
impl ::std::ops::Deref for QualifiedType {
  type Target = Type;

  fn deref(&self) -> &Self::Target {
    &self.unqualified_type
  }
}

impl QualifiedType {
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

  pub fn qualifiers(&self) -> &Qualifiers {
    &self.qualifiers
  }

  pub fn unqualified_type(&self) -> &Type {
    &self.unqualified_type
  }

  pub fn destructure(self) -> (Qualifiers, Rc<Type>) {
    (self.qualifiers, self.unqualified_type)
  }
}
impl From<Type> for QualifiedType {
  #[inline]
  fn from(value: Type) -> Self {
    QualifiedType::new_unqualified(value.into())
  }
}
