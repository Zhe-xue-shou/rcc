use ::rcc_shared::{Keyword, Literal};
use ::rcc_utils::{IntoWith, RefEq, concat_static_str as css, ensure_is_pod};

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
#[allow(non_upper_case_globals)]
const Pad: &str = " ";
impl Qualifiers {
  #[allow(non_upper_case_globals)]
  pub(crate) const MetaStaticStr: [&'static str; 4] =
    ["const", "volatile", "restrict", "_Atomic"];
  #[allow(non_upper_case_globals)]
  pub const StaticStr: [&'static str; 16] = [
    css!(),                                                      // 0x00
    css!(QMeta[0]),                                              // 0x01
    css!(QMeta[1]),                                              // 0x02
    css!(QMeta[0], Pad, QMeta[1]),                               // 0x03
    css!(QMeta[2]),                                              // 0x04
    css!(QMeta[0], Pad, QMeta[2]),                               // 0x05
    css!(QMeta[1], Pad, QMeta[2]),                               // 0x06
    css!(QMeta[0], Pad, QMeta[1], Pad, QMeta[2]),                // 0x07
    css!(QMeta[3]),                                              // 0x08
    css!(QMeta[0], Pad, QMeta[3]),                               // 0x09
    css!(QMeta[1], Pad, QMeta[3]),                               // 0x0A
    css!(QMeta[0], Pad, QMeta[1], Pad, QMeta[3]),                // 0x0B
    css!(QMeta[2], Pad, QMeta[3]),                               // 0x0C
    css!(QMeta[0], Pad, QMeta[2], Pad, QMeta[3]),                // 0x0D
    css!(QMeta[1], Pad, QMeta[2], Pad, QMeta[3]),                // 0x0E
    css!(QMeta[0], Pad, QMeta[1], Pad, QMeta[2], Pad, QMeta[3]), // 0x0F
  ];

  #[inline(always)]
  pub const fn into_static_str(self) -> &'static str {
    Self::StaticStr[self.bits() as usize]
  }
}
#[allow(non_upper_case_globals)]
const QMeta: [&str; 4] = Qualifiers::MetaStaticStr;
::bitflags::bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub struct FunctionSpecifier : u8 {
    const Inline = 0x01;
    const Noreturn = 0x10;
  }
}
impl FunctionSpecifier {
  #[allow(non_upper_case_globals)]
  pub(crate) const MetaStaticStr: [&'static str; 2] = ["inline", "_Noreturn"];
  #[allow(non_upper_case_globals)]
  pub const StaticStr: [&'static str; 4] = [
    css!(),                        // 0x00
    css!(FMeta[0]),                // 0x01
    css!(FMeta[1]),                // 0x10
    css!(FMeta[0], Pad, FMeta[1]), // 0x11
  ];

  #[inline(always)]
  pub const fn into_static_str(self) -> &'static str {
    Self::StaticStr[self.bits() as usize]
  }
}

#[allow(non_upper_case_globals)]
const FMeta: [&str; 2] = FunctionSpecifier::MetaStaticStr;

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
