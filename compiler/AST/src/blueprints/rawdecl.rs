#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash, ::strum_macros::Display)]
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
    use VarDeclKind::*;
    match (lhs, rhs) {
      (Tentative, Tentative) => Tentative,
      (Definition, _) | (_, Definition) => Definition,
      _ => Self::Declaration,
    }
  }
}
