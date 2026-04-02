use ::rcc_utils::StrRef;
use ::std::collections::HashSet;

/// only tracks names.
#[derive(Debug, Default)]
pub struct UnitScope<'c> {
  scopes: Vec<HashSet<StrRef<'c>>>,
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
