use ::rc_utils::{shared_ptr, weak_ptr};
use ::std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

use super::Storage;
use crate::types::QualifiedType;

pub type SymbolRef = shared_ptr<Symbol>;
pub type WeakSymbolRef = weak_ptr<Symbol>;

/// a lexical scope.
type ScopeAssoc<T> = HashMap<String, shared_ptr<T>>;
/// A lexical scope stack.
#[derive(Debug)]
pub struct Scope<T> {
  scopes: Vec<ScopeAssoc<T>>,
}
/// only tracks names
#[derive(Debug, Default)]
pub struct UnitScope {
  scopes: Vec<HashSet<String>>,
}
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
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
  /// multiple tentative definitions are allowed.
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
pub struct Symbol {
  pub qualified_type: QualifiedType,
  pub storage_class: Storage,
  pub name: String,
  /// declaration or definition
  pub declkind: VarDeclKind,
}
#[derive(Debug, Default)]
pub struct Environment {
  symbols: Scope<Symbol>,
  cache: RefCell<HashMap<String, WeakSymbolRef>>,
}
impl Environment {
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
  pub fn find(&self, name: &str) -> Option<SymbolRef> {
    if let Some(sym) = self.cache.borrow().get(name) {
      // upgrade weak ref, this already handles whether the ref is dead or not
      return sym.upgrade();
    }

    let sym = self.symbols.get(name);
    if let Some(s) = &sym {
      self
        .cache
        .borrow_mut()
        .insert(name.to_string(), Rc::downgrade(s));
    }
    sym
  }

  // we dont provide cache layer for shallow find, since it'll contain non-local symbols
  pub fn shallow_find(&self, name: &str) -> Option<SymbolRef> {
    let sym = self.symbols.shallow_get(name);
    if let Some(s) = &sym {
      self
        .cache
        .borrow_mut()
        .insert(name.to_string(), Rc::downgrade(s));
    }
    sym
  }

  /// note: if the symbol already exists, it'll be updated.
  pub fn declare_symbol(
    &mut self,
    name: String,
    symbol: SymbolRef,
  ) -> SymbolRef {
    // overwrite cache
    self
      .cache
      .borrow_mut()
      .insert(name.clone(), Rc::downgrade(&symbol));
    self.symbols.declare(name, symbol.clone())
  }
}
impl Symbol {
  #[inline]
  pub fn is_typedef(&self) -> bool {
    self.storage_class.is_typedef()
  }

  #[inline]
  pub fn is_constexpr(&self) -> bool {
    self.storage_class.is_constexpr()
  }

  pub fn new(
    qualified_type: QualifiedType,
    storage_class: Storage,
    name: String,
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
    qualified_type: QualifiedType,
    storage_class: Storage,
    name: String,
  ) -> SymbolRef {
    Self::new_ref(Self::new(
      qualified_type,
      storage_class,
      name,
      VarDeclKind::Declaration,
    ))
  }

  pub fn def(
    qualified_type: QualifiedType,
    storage_class: Storage,
    name: String,
  ) -> SymbolRef {
    Self::new_ref(Self::new(
      qualified_type,
      storage_class,
      name,
      VarDeclKind::Definition,
    ))
  }

  pub fn tentative(
    qualified_type: QualifiedType,
    storage_class: Storage,
    name: String,
  ) -> SymbolRef {
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
impl UnitScope {
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

  pub fn declare(&mut self, name: String) {
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
impl<T> Default for Scope<T> {
  fn default() -> Self {
    Self {
      scopes: Vec::default(),
    }
  }
}
impl<T> Scope<T> {
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

  pub fn declare(&mut self, name: String, val: shared_ptr<T>) -> shared_ptr<T> {
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

  impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}: {}", self.name, self.qualified_type)
    }
  }
}
