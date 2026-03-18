use ::rcc_utils::{shared_ptr, weak_ptr};
use ::std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

use super::{Storage, StrRef};
use crate::types::QualifiedType;

pub type SymbolRef<'c> = shared_ptr<Symbol<'c>>;
pub type WeakSymbolRef<'c> = weak_ptr<Symbol<'c>>;

/// a lexical scope.
type ScopeAssoc<'c, T> = HashMap<StrRef<'c>, shared_ptr<T>>;
/// A lexical scope stack.
#[derive(Debug)]
pub struct Scope<'c, T> {
  scopes: Vec<ScopeAssoc<'c, T>>,
}
/// only tracks names
#[derive(Debug, Default)]
pub struct UnitScope<'c> {
  scopes: Vec<HashSet<StrRef<'c>>>,
}
#[derive(Debug, Eq, PartialEq, Clone, Copy, ::strum_macros::Display)]
pub enum VarDeclKind {
  /// declaration:
  ///   - file-scope: without initializer, with storage-class specifier(extern/static)
  ///   - block-scope: without initializer, with `extern` specifier (initializer is not allowed); functionproto
  Declaration,
  /// complete definition
  ///   - file-scope: with initializer, regardless of the presence of storage-class specifier
  ///   - block-scope: variable declaration without `extern` specifier
  Definition,
  /// tentative definition - no initializer, no storage-class specifier, and in file scope(**block scope is not allowed**)
  /// ```c
  /// int a; // tentative definition
  /// extern int a; // declaration
  /// int a = 0; // complete definition
  /// static int a; // ok, still tentative definition
  /// extern int a; // ok, still declaration
  /// // int a = 1; // error: redefinition
  /// ```
  /// Tentative declaration is C only, C++ has ODR. Multiple tentative definitions are allowed.
  ///
  /// if no complete definition is found, the tentative definition is treated as a complete definition uninitialized (initialized to zero)
  Tentative,
}
impl VarDeclKind {
  pub fn merge(lhs: Self, rhs: Self) -> Self {
    match (lhs, rhs) {
      (Self::Tentative, Self::Tentative) => Self::Tentative,
      (Self::Definition, _) | (_, Self::Definition) => Self::Definition,
      _ => Self::Declaration,
    }
  }
}
#[derive(Debug)]
pub struct Symbol<'c> {
  pub qualified_type: QualifiedType<'c>,
  /// for global variable, if the [`VarDeclKind`] is [`VarDeclKind::Definition`]
  /// and the [`Symbol::storage_class`] is [`Storage::Extern`], the [`Storage::Extern`] has no effect
  /// -- chossing not to wrap it into [`Option`] is just for convenience and now it's hard to change,
  ///
  /// That being said, during TAC gen,
  /// - if the global vardef has both [`Storage::Extern`] and [`VarDeclKind::Definition`]
  ///   or one [`VarDeclKind::Tentative`] (one tantative counts as definition), add it as definition
  /// - else if only has [`Storage::Extern`] and [`VarDeclKind::Declaration`], it's declaration and let linker handle it.
  pub storage_class: Storage,
  pub name: StrRef<'c>,
  pub declkind: VarDeclKind,
}
#[derive(Debug, Default)]
pub struct Environment<'c> {
  symbols: Scope<'c, Symbol<'c>>,
  cache: RefCell<HashMap<StrRef<'c>, WeakSymbolRef<'c>>>,
}
impl<'c> Environment<'c> {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn is_global(&self) -> bool {
    self.symbols.is_top_level()
  }

  // does NOT clear cache
  pub fn enter(&mut self) {
    self.symbols.push_scope();
  }

  pub fn exit(&mut self) {
    // do we still needs to clear cache although weak refs are used?
    // self.cache.borrow_mut().clear();
    self.symbols.pop_scope();
  }

  /// look up symbol and potentially cache it
  pub fn find(&self, name: StrRef<'c>) -> Option<SymbolRef<'c>> {
    if let Some(sym) = self.cache.borrow().get(name) {
      // upgrade weak ref, this already handles whether the ref is dead or not
      return sym.upgrade();
    }

    let sym = self.symbols.get(name);
    if let Some(s) = &sym {
      self.cache.borrow_mut().insert(name, Rc::downgrade(s));
    }
    sym
  }

  // we dont provide cache layer for shallow find, since it'll contain non-local symbols
  pub fn shallow_find(
    &self,
    name: StrRef<'c>,
  ) -> Option<SymbolRef<'c>> {
    let sym = self.symbols.shallow_get(name);
    if let Some(s) = &sym {
      self.cache.borrow_mut().insert(name, Rc::downgrade(s));
    }
    sym
  }

  /// note: if the symbol already exists, it'll be updated.
  pub fn declare_symbol(
    &mut self,
    name: StrRef<'c>,
    symbol: SymbolRef<'c>,
  ) -> SymbolRef<'c> {
    // overwrite cache
    self.cache.borrow_mut().insert(name, Rc::downgrade(&symbol));
    self.symbols.declare(name, symbol.clone())
  }
}
impl<'c> Symbol<'c> {
  #[inline]
  pub fn is_typedef(&self) -> bool {
    self.storage_class.is_typedef()
  }

  #[inline]
  pub fn is_constexpr(&self) -> bool {
    self.storage_class.is_constexpr()
  }

  pub fn new(
    qualified_type: QualifiedType<'c>,
    storage_class: Storage,
    name: StrRef<'c>,
    declkind: VarDeclKind,
  ) -> Self {
    Self {
      qualified_type,
      storage_class,
      declkind,
      name,
    }
  }

  pub fn decl(
    qualified_type: QualifiedType<'c>,
    storage_class: Storage,
    name: StrRef<'c>,
  ) -> SymbolRef<'c> {
    Self::new_ref(Self::new(
      qualified_type,
      storage_class,
      name,
      VarDeclKind::Declaration,
    ))
  }

  pub fn def(
    qualified_type: QualifiedType<'c>,
    storage_class: Storage,
    name: StrRef<'c>,
  ) -> SymbolRef<'c> {
    Self::new_ref(Self::new(
      qualified_type,
      storage_class,
      name,
      VarDeclKind::Definition,
    ))
  }

  pub fn tentative(
    qualified_type: QualifiedType<'c>,
    storage_class: Storage,
    name: StrRef<'c>,
  ) -> SymbolRef<'c> {
    Self::new_ref(Self::new(
      qualified_type,
      storage_class,
      name,
      VarDeclKind::Tentative,
    ))
  }

  pub fn new_ref(symbol: Symbol) -> SymbolRef {
    Rc::new(RefCell::new(symbol))
  }
}
impl<'c> UnitScope<'c> {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn push_scope(&mut self) {
    self.scopes.push(Default::default());
  }

  pub fn pop_scope(&mut self) {
    self.scopes.pop();
  }

  pub fn shallow_contains(&self, name: &str) -> bool {
    self.scopes.last().is_some_and(|scope| scope.contains(name))
  }

  pub fn contains(&self, name: &str) -> bool {
    for scope in self.scopes.iter().rev() {
      if scope.contains(name) {
        return true;
      }
    }
    false
  }

  pub fn declare(&mut self, name: StrRef<'c>) {
    let current = self.scopes.last_mut();
    assert!(
      current.is_some(),
      "No scope to declare variable `{}` in",
      name
    );
    current.unwrap().insert(name);
  }

  pub fn is_top_level(&self) -> bool {
    self.scopes.len() == 1
  }
}
impl<'c, T> Default for Scope<'c, T> {
  fn default() -> Self {
    Self {
      scopes: Vec::default(),
    }
  }
}
impl<'c, T> Scope<'c, T> {
  #[allow(unused)]
  pub fn new() -> Self {
    Self::default()
  }

  pub fn push_scope(&mut self) {
    self.scopes.push(Default::default());
  }

  pub fn pop_scope(&mut self) {
    self.scopes.pop();
  }

  pub fn shallow_get(&self, name: &str) -> Option<shared_ptr<T>> {
    self
      .scopes
      .last()
      .and_then(|scope| scope.get(name).cloned())
  }

  pub fn get(&self, name: &str) -> Option<shared_ptr<T>> {
    for scope in self.scopes.iter().rev() {
      if let Some(val) = scope.get(name) {
        return Some(val.clone());
      }
    }
    None
  }

  pub fn declare(
    &mut self,
    name: StrRef<'c>,
    val: shared_ptr<T>,
  ) -> shared_ptr<T> {
    let current = self.scopes.last_mut();
    assert!(
      current.is_some(),
      "No scope to declare variable `{}` in",
      name
    );
    _/* maybe old val, or None */ = current.unwrap().insert(name, val.clone());
    val
  }

  pub fn is_top_level(&self) -> bool {
    self.scopes.len() == 1
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::*;

  impl<'c> Display for Symbol<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}: {}", self.name, self.qualified_type)
    }
  }
}
