use std::{marker::PhantomData, ptr::NonNull};

use rcc_ast::{Context, VarDeclKind, types::QualifiedType};
use rcc_shared::Storage;
use rcc_utils::StrRef;

#[derive(Debug)]
pub struct DeclNode<'c> {
  qualified_type: QualifiedType<'c>,
  storage_class: Storage,
  name: StrRef<'c>,
  /// for global variable, if the [`VarDeclKind`] is [`VarDeclKind::Definition`]
  /// and the [`Self::storage_class`] is [`Storage::Extern`], the [`Storage::Extern`] has no effect
  ///
  /// That being said, during TAC gen,
  /// - if the global vardef has both [`Storage::Extern`] and [`VarDeclKind::Definition`]
  ///   or one [`VarDeclKind::Tentative`] (one tantative counts as definition), add it as definition
  /// - else if only has [`Storage::Extern`] and [`VarDeclKind::Declaration`], it's declaration and let linker handle it.
  declkind: VarDeclKind,
  /// Shall only be [`None`] if the current one is canonical.
  previous_decl: Option<DeclRef<'c>>,
  /// The nopde points to the eearlist appeared node.
  canonical_decl: DeclRef<'c>,
  /// The node points to the `definition` node.
  ///
  /// # Directly access this field to judge whether a definition exists is wrong.
  /// only canonical one would be updated.
  definition: Option<DeclRef<'c>>,
}
/// SAFETY: this struct is safe as long as the [`DeclNode`] it points to are located inside the Arena, so does itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeclRef<'c> {
  ptr: NonNull<DeclNode<'c>>,
  marker: PhantomData<&'c DeclNode<'c>>,
}

impl<'c> DeclRef<'c> {
  fn from_ptr(ptr: *mut DeclNode<'c>) -> Self {
    Self {
      ptr: NonNull::new(ptr).expect("declaration pointer shall not be null"),
      marker: PhantomData,
    }
  }

  // fn as_ptr(self) -> *mut DeclNode<'c> {
  //   self.ptr.as_ptr()
  // }

  fn as_decl(self) -> &'c DeclNode<'c> {
    unsafe { self.ptr.as_ref() }
  }

  fn as_decl_mut(self) -> &'c mut DeclNode<'c> {
    unsafe { &mut *self.ptr.as_ptr() }
  }

  #[inline]
  pub fn qualified_type(self) -> QualifiedType<'c> {
    self.as_decl().qualified_type
  }

  #[inline]
  pub fn name(self) -> StrRef<'c> {
    self.as_decl().name
  }

  #[inline]
  pub fn storage_class(self) -> Storage {
    self.as_decl().storage_class
  }

  #[inline]
  pub fn declkind(self) -> VarDeclKind {
    self.as_decl().declkind
  }

  #[inline]
  pub fn previous_decl(self) -> Option<DeclRef<'c>> {
    self.as_decl().previous_decl
  }

  #[inline]
  pub fn canonical_decl(self) -> DeclRef<'c> {
    self.as_decl().canonical_decl
  }

  #[inline]
  pub fn definition(self) -> Option<DeclRef<'c>> {
    self.canonical_decl().as_decl().definition
  }

  #[inline]
  pub fn is_typedef(self) -> bool {
    self.storage_class().is_typedef()
  }

  #[inline]
  pub fn is_constexpr(self) -> bool {
    self.storage_class().is_constexpr()
  }

  pub fn is_address_constant(self) -> bool {
    self.qualified_type().is_functionproto()
      || matches!(
        self.storage_class(),
        Storage::Static | Storage::Extern | Storage::Constexpr
      )
  }

  /// tecnically speaking a node is created and shall never change except for the `definition` pointer,
  /// but here in my sema i didnt merge first then create node, but backpatching them, so here it serves as a workaround.
  #[inline]
  pub(super) fn set_qualified_type(self, qualified_type: QualifiedType<'c>) {
    self.as_decl_mut().qualified_type = qualified_type;
  }

  /// ditto.
  #[inline]
  pub(super) fn set_storage_class(self, storage_class: Storage) {
    self.as_decl_mut().storage_class = storage_class;
  }

  // pub(super) fn set_declkind(self, declkind: VarDeclKind) {
  //   {
  //     let decl = self.as_decl_mut();
  //     decl.declkind = declkind;
  //     if matches!(declkind, VarDeclKind::Definition) {
  //       decl.definition = Some(self);
  //     }
  //   }
  // }

  #[inline]
  fn set_definition(self, definition: Option<DeclRef<'c>>) {
    self.as_decl_mut().definition = definition;
  }
}

impl<'c> DeclNode<'c> {
  pub fn alloc(
    context: &'c Context<'c>,
    qualified_type: QualifiedType<'c>,
    storage_class: Storage,
    name: StrRef<'c>,
    declkind: VarDeclKind,
    previous_decl: Option<DeclRef<'c>>,
  ) -> DeclRef<'c> {
    use ::std::mem::MaybeUninit;
    #[allow(clippy::uninit_assumed_init)]
    #[allow(invalid_value)]
    let node = context.arena().alloc(Self {
      qualified_type,
      storage_class,
      name,
      declkind,
      previous_decl,
      canonical_decl: unsafe { MaybeUninit::uninit().assume_init() },
      definition: None,
    });
    let this = DeclRef::from_ptr(&raw mut *node as *mut _);

    node.canonical_decl = previous_decl
      .map(|prev| prev.canonical_decl())
      .unwrap_or(this);

    node.definition = match declkind {
      VarDeclKind::Definition => Some(this),
      _ =>
        previous_decl.and_then(|previous: DeclRef<'_>| previous.definition()),
    };

    if matches!(declkind, VarDeclKind::Definition) {
      // If we are the definition, update the canonical node so all prior/future
      // nodes in the chain can find the definition.
      this.canonical_decl().set_definition(Some(this));
    }

    this
  }

  #[inline]
  pub fn decl(
    context: &'c Context<'c>,
    qualified_type: QualifiedType<'c>,
    storage_class: Storage,
    name: StrRef<'c>,
    previous_decl: Option<DeclRef<'c>>,
  ) -> DeclRef<'c> {
    Self::alloc(
      context,
      qualified_type,
      storage_class,
      name,
      VarDeclKind::Declaration,
      previous_decl,
    )
  }

  #[inline]
  pub fn def(
    context: &'c Context<'c>,
    qualified_type: QualifiedType<'c>,
    storage_class: Storage,
    name: StrRef<'c>,
    previous_decl: Option<DeclRef<'c>>,
  ) -> DeclRef<'c> {
    Self::alloc(
      context,
      qualified_type,
      storage_class,
      name,
      VarDeclKind::Definition,
      previous_decl,
    )
  }

  #[inline]
  pub fn tentative(
    context: &'c Context<'c>,
    qualified_type: QualifiedType<'c>,
    storage_class: Storage,
    name: StrRef<'c>,
    previous_decl: Option<DeclRef<'c>>,
  ) -> DeclRef<'c> {
    Self::alloc(
      context,
      qualified_type,
      storage_class,
      name,
      VarDeclKind::Tentative,
      previous_decl,
    )
  }
}
::rcc_utils::ensure_is_pod!(DeclNode<'_>);
::rcc_utils::ensure_is_pod!(DeclRef<'_>);

mod fmt {
  use ::std::fmt::{Display, Pointer};

  use super::*;

  impl<'c> Display for DeclNode<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      write!(f, "{}: {}", self.name, self.qualified_type)
    }
  }

  impl<'c> Pointer for DeclRef<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{:p}", self.as_decl())
    }
  }

  impl<'c> Display for DeclRef<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      self.as_decl().fmt(f)
    }
  }
}
