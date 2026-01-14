#![allow(unused)]

use ::std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

use crate::common::{storage::Storage, types::QualifiedType};

/// as someone who came from C++, I'd more prefer to call it shared_ptr rather than Rc/RefCell or whatever. :p
#[allow(non_camel_case_types)]
pub type shared_ptr<T> = Rc<RefCell<T>>;
pub type SymbolRef = shared_ptr<Symbol>;

type ScopeAssoc<T> = HashMap<String, shared_ptr<T>>;
#[derive(Debug)]
pub struct Scope<T> {
  scopes: Vec<ScopeAssoc<T>>,
}
/// only tracks names
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
  /// multiple tentative definitions are allowed
  /// if no complete definition is found, the tentative definition is treated as a complete definition uninitialized (initialized to zero)
  Tentative,
}
#[derive(Debug)]
pub struct Symbol {
  pub qualified_type: QualifiedType,
  pub storage_class: Storage,
  pub name: String,
  /// declaration or definition
  pub declkind: VarDeclKind,
}
#[derive(Debug)]
pub struct Environment {
  pub symbols: Scope<Symbol>,
}
impl Environment {
  pub fn new() -> Self {
    Self {
      symbols: Scope::new(),
    }
  }

  pub fn is_global(&self) -> bool {
    self.symbols.is_top_level()
  }

  pub fn enter(&mut self) {
    self.symbols.push_scope();
  }

  pub fn exit(&mut self) {
    self.symbols.pop_scope();
  }

  pub fn find(&self, name: &str) -> Option<shared_ptr<Symbol>> {
    self.symbols.get(name)
  }
}
impl Symbol {
  pub fn is_typedef(&self) -> bool {
    matches!(self.storage_class, Storage::Typedef)
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
    Self { scopes: Vec::new() }
  }

  pub fn push_scope(&mut self) {
    self.scopes.push(HashSet::new());
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
impl<T> Scope<T> {
  pub fn new() -> Self {
    Self { scopes: Vec::new() }
  }

  pub fn push_scope(&mut self) {
    self.scopes.push(ScopeAssoc::new());
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
    current.unwrap().insert(name, val.clone());
    val
  }

  pub fn is_top_level(&self) -> bool {
    self.scopes.len() == 1
  }
}
mod fmt {
  use ::std::fmt::Display;

  use super::Symbol;

  impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}: {}", self.name, self.qualified_type)
    }
  }
}
