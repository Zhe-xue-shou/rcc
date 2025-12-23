use crate::common::keyword::Keyword;
use crate::common::token::Literal;
use crate::common::types::Qualifiers;
use crate::parser::expression::Expression;
use crate::parser::statement::Compound;
use ::strum_macros::Display;

pub struct Program {
  pub declarations: Vec<Declaration>,
}
/// declaration:
///        declaration-specifiers init-declarator-listopt ;
///        attribute-specifier-sequence declaration-specifiers init-declarator-list ; (don't care)
///        static_assert-declaration (don't care)
///        attribute-declaration (don't care)
pub enum Declaration {
  Function(Function),
  Variable(VarDef),
}
/// storage-class-specifier
#[derive(Display, PartialEq, Eq)]
pub enum Storage {
  /// variables that declared in block scope without any storage-class specifier
  /// are considered to have automatic storage duration.
  #[strum(serialize = "auto")]
  Automatic,
  #[strum(serialize = "register")]
  Register,
  /// - Function declarations with no storage-class specifier are always handled
  /// as though they include an extern specifier
  /// - if variable declarations appear at file scope, they have external linkage
  /// - use extern to declare an identifier that’s already visible.
  /// ```c
  /// static int a;
  /// extern int a; // this is valid and a has internal linkage
  /// extern int b;
  /// static int b = 0; // this is also valid... (internal linkage)
  /// ```
  #[strum(serialize = "extern")]
  Extern,
  /// - At file scope, the static specifier indicates that a function or variable
  /// has internal linkage.
  /// - At block scope(i.e., for variables), the static specifier controls storage duration, not linkage.
  #[strum(serialize = "static")]
  Static,
  /// according to standard, `typedef` is categorized as a storage-class specifier for **syntactic convenience only**.
  #[strum(serialize = "typedef")]
  Typedef,
  /// the variable is allocated when the thread is created
  #[strum(serialize = "thread_local")]
  ThreadLocal, // I won't care about this now
  /// C23, `#define VAR value` is the same `constexpr TYPE VAR = value;` with fewer name collisions
  #[strum(serialize = "constexpr")]
  Constexpr, // ditto
}
impl From<&Keyword> for Storage {
  fn from(kw: &Keyword) -> Self {
    match kw {
      Keyword::Auto => Storage::Automatic,
      Keyword::Register => Storage::Register,
      Keyword::Extern => Storage::Extern,
      Keyword::Static => Storage::Static,
      Keyword::Typedef => Storage::Typedef,
      Keyword::ThreadLocal => Storage::ThreadLocal,
      Keyword::Constexpr => Storage::Constexpr,
      _ => panic!("cannot convert {:?} to Storage", kw),
    }
  }
}
impl From<&Literal> for Storage {
  fn from(literal: &Literal) -> Self {
    match literal {
      Literal::Keyword(kw) => Storage::from(kw),
      _ => panic!("cannot convert {:?} to Storage", literal),
    }
  }
}

impl From<&Literal> for Qualifiers {
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

/// abstract declarator: no variable name/identifier
///
/// used in parsing
#[derive(::std::marker::ConstParamTy, PartialEq, Eq)]
pub enum DeclaratorType {
  Abstract,
  Named,
  Maybe,
}
/// declarator:
///     pointer_opt direct-declarator
pub struct Declarator {
  pub name: Option<String>,
  pub modifiers: Vec<Modifier>, // pointer, array, function
}
/// direct-declarator:
///     - ( declarator )
///     - identifier attribute-specifier-sequence_opt
///     - array-declarator attribute-specifier-sequence_opt
///     - function-declarator attribute-specifier-sequence_opt
///
/// pointer:
///     - \* attribute-specifier-sequenceopt type-qualifier-list_opt
///     - \* attribute-specifier-sequenceopt type-qualifier-list_opt pointer
///
/// won't care about attribute-specifier-sequence for now
///
/// this is flatten structure, so the order of `Vec<Modifier>` in `Declarator` matters
pub enum Modifier {
  Pointer(Qualifiers),
  Array(ArrayModifier),
  Function(FunctionSignature),
}
pub struct Member {
  pub specifiers: Vec<TypeSpecifier>,
  pub qualifiers: Qualifiers,
  pub modifiers: Vec<Modifier>,
  pub declarator: Option<Declarator>,
  pub bit_width: Option<Expression>,
}
pub struct Parameter {
  pub specifications: DeclSpecs,
  pub declarator: Declarator,
}
pub struct Struct {
  pub name: Option<String>,
  pub members: Vec<Member>,
}
/// type-specifier
pub enum TypeSpecifier {
  Void,
  Char,
  Short,
  Int,
  Long,
  Float,
  Double,
  Signed,
  Unsigned,
  Bool,
  Complex,
  Typedef(String),
  // vvv below should be wrong, but now don't care
  Struct(Struct),
  Union(Struct),
  Enum(EnumSpecifier),
}

pub enum FunctionSpecifier {
  Inline,
  Noreturn,
}

impl TryFrom<&Keyword> for FunctionSpecifier {
  type Error = ();
  fn try_from(kw: &Keyword) -> Result<Self, Self::Error> {
    match kw {
      Keyword::Inline => Ok(FunctionSpecifier::Inline),
      Keyword::_Noreturn => Ok(FunctionSpecifier::Noreturn),
      _ => Err(()),
    }
  }
}

impl TryFrom<&Literal> for FunctionSpecifier {
  type Error = ();
  fn try_from(literal: &Literal) -> Result<Self, Self::Error> {
    match literal {
      Literal::Keyword(kw) => FunctionSpecifier::try_from(kw),
      _ => Err(()),
    }
  }
}

impl TryFrom<&Keyword> for TypeSpecifier {
  type Error = ();
  fn try_from(kw: &Keyword) -> Result<Self, Self::Error> {
    match kw {
      Keyword::Void => Ok(TypeSpecifier::Void),
      Keyword::Char => Ok(TypeSpecifier::Char),
      Keyword::Short => Ok(TypeSpecifier::Short),
      Keyword::Int => Ok(TypeSpecifier::Int),
      Keyword::Long => Ok(TypeSpecifier::Long),
      Keyword::Float => Ok(TypeSpecifier::Float),
      Keyword::Double => Ok(TypeSpecifier::Double),
      Keyword::Signed => Ok(TypeSpecifier::Signed),
      Keyword::Unsigned => Ok(TypeSpecifier::Unsigned),
      Keyword::Bool => Ok(TypeSpecifier::Bool),
      _ => Err(()),
    }
  }
}
impl TryFrom<&Literal> for TypeSpecifier {
  type Error = ();
  fn try_from(literal: &Literal) -> Result<Self, Self::Error> {
    match literal {
      Literal::Keyword(kw) => TypeSpecifier::try_from(kw),
      _ => Err(()),
    }
  }
}
/// declaration-specifiers:
///    declaration-specifier attribute-specifier-sequenceopt (don't care)
///    declaration-specifier declaration-specifiers
/// declaration-specifier:
///    storage-class-specifier
///    type-specifier-qualifier
///    function-specifier
pub struct DeclSpecs {
  pub function_specifiers: Vec<FunctionSpecifier>,
  pub storage_class: Option<Storage>,
  pub qualifiers: Qualifiers,
  pub type_specifiers: Vec<TypeSpecifier>,
}
pub struct Function {
  pub declspecs: DeclSpecs,
  pub declarator: Declarator,
  pub body: Option<Compound>,
}
pub struct VarDef {
  pub declspecs: DeclSpecs,
  pub declarator: Declarator,
  pub initializer: Option<Initializer>,
}
/// array-declarator:
///     - direct-declarator \[ type-qualifier-list_opt assignment-expression_opt \]
///     - direct-declarator \[ static type-qualifier-list_opt assignment-expression \]
///     - direct-declarator \[ type-qualifier-list static assignment-expression \]
///     - direct-declarator \[ type-qualifier-list_opt * \]
pub struct ArrayModifier {
  pub qualifiers: Qualifiers,
  pub is_static: bool,
  pub bound: ArrayBound,
}
pub enum ArrayBound {
  Constant(usize),
  Variable(Expression),
  Incomplete,
}
/// function-declarator:
///     - direct-declarator ( parameter-type-list_opt )
pub struct FunctionSignature {
  pub parameters: Vec<Parameter>,
  pub is_variadic: bool,
}
pub enum Initializer {
  Expression(Box<Expression>),
  List(Vec<InitializerListEntry>),
}
pub struct InitializerListEntry {
  pub designators: Vec<Designator>,
  pub value: Box<Initializer>,
}
pub enum Designator {
  Member(String),
  Index(Expression),
}
pub struct EnumSpecifier {
  pub name: Option<String>,
  pub enumerators: Vec<Enumerator>,
}
pub struct Enumerator {
  pub name: String,
  pub value: Option<Expression>,
}
impl Enumerator {
  pub fn new(name: String, value: Option<Expression>) -> Self {
    Self { name, value }
  }
}
impl EnumSpecifier {
  pub fn new(name: Option<String>, enumerators: Vec<Enumerator>) -> Self {
    Self { name, enumerators }
  }
}
impl Function {
  pub fn new(declspec: DeclSpecs, declarator: Declarator, body: Option<Compound>) -> Self {
    Self {
      declspecs: declspec,
      declarator,
      body,
    }
  }
}
impl FunctionSignature {
  pub fn new(parameters: Vec<Parameter>, is_variadic: bool) -> Self {
    Self {
      parameters,
      is_variadic,
    }
  }
}
impl ::core::default::Default for FunctionSignature {
  fn default() -> Self {
    Self {
      parameters: Vec::new(),
      is_variadic: false,
    }
  }
}
impl Parameter {
  pub fn new(specifications: DeclSpecs, declarator: Declarator) -> Self {
    Self {
      specifications,
      declarator,
    }
  }
}
impl Program {
  pub fn new() -> Self {
    Self {
      declarations: Vec::new(),
    }
  }
}
impl Declarator {
  pub fn new(name: Option<String>) -> Self {
    Self {
      name,
      modifiers: Vec::new(),
    }
  }
}
impl ::core::default::Default for DeclSpecs {
  fn default() -> Self {
    Self {
      function_specifiers: Vec::new(),
      storage_class: None,
      qualifiers: Qualifiers::empty(),
      type_specifiers: Vec::new(),
    }
  }
}
impl VarDef {
  pub fn new(
    declspecs: DeclSpecs,
    declarator: Declarator,
    initializer: Option<Initializer>,
  ) -> Self {
    Self {
      declspecs,
      declarator,
      initializer,
    }
  }
  pub fn is_typedef(&self) -> bool {
    let maybe = matches!(self.declspecs.storage_class, Some(Storage::Typedef));
    if maybe {
      debug_assert!(
        self.initializer.is_none(),
        "typedef variable cannot have initializer"
      );
    }
    maybe
  }
  pub fn is_vardef(&self) -> bool {
    !self.is_typedef()
  }
}
mod fmt {

  use super::{
    DeclSpecs, Declaration, EnumSpecifier, Function, FunctionSignature, Modifier, Program, Struct,
    TypeSpecifier, VarDef,
  };
  use ::std::fmt::{Debug, Display};

  impl Display for Declaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Declaration::Function(func) => <Function as Display>::fmt(func, f),
        Declaration::Variable(var) => <VarDef as Display>::fmt(var, f),
      }
    }
  }
  impl Debug for Declaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      self
        .declarations
        .iter()
        .try_for_each(|decl| write!(f, "{}\n", decl))
    }
  }
  impl Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "<{} {}: {} -> {}> {}",
        match &self.body {
          Some(_) => "function",
          None => "functiondecl",
        },
        match &self.declarator.name {
          Some(name) => name,
          None => "<anonymous>",
        },
        self
          .declarator
          .modifiers
          .iter()
          .map(|m| m.to_string())
          .collect::<Vec<_>>()
          .join(", "),
        self
          .declspecs
          .type_specifiers
          .iter()
          .map(|s| s.to_string())
          .collect::<Vec<_>>()
          .join(" "),
        match &self.body {
          Some(block) => format!("{}", block),
          None => ";".to_string(),
        }
      )
    }
  }
  impl Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Modifier::Pointer(qualifiers) => {
          write!(
            f,
            "*{}",
            if qualifiers.is_empty() {
              "".to_string()
            } else {
              format!(
                " {}",
                qualifiers
                  .iter()
                  .map(|q| q.to_string())
                  .collect::<Vec<_>>()
                  .join(" ")
              )
            }
          )
        }
        Modifier::Array(_) => todo!(),
        Modifier::Function(function_signature) => {
          <FunctionSignature as Display>::fmt(function_signature, f)
        }
      }
    }
  }
  impl Debug for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }

  impl Display for FunctionSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "(")?;
      for (i, param) in self.parameters.iter().enumerate() {
        if i > 0 {
          write!(f, ", ")?;
        }
        write!(
          f,
          "{}",
          param
            .specifications
            .type_specifiers
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join(" ")
        )?;
      }
      if self.is_variadic {
        if !self.parameters.is_empty() {
          write!(f, ", ")?;
        }
        write!(f, "...")?;
      }
      write!(f, ")")
    }
  }
  impl Debug for FunctionSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for VarDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      if self.is_typedef() {
        write!(
          f,
          "typedef {} {};",
          self
            .declspecs
            .type_specifiers
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(" "),
          match &self.declarator.name {
            Some(name) => name,
            None => "<anonymous>",
          },
        )
      } else {
        write!(
          f,
          "{} {}{}",
          self
            .declspecs
            .type_specifiers
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(" "),
          match &self.declarator.name {
            Some(name) => name,
            None => "<anonymous>",
          },
          match &self.initializer {
            Some(_) => format!(" = <initializer>"),
            None => "".to_string(),
          }
        )?;
        write!(f, ";")
      }
    }
  }
  impl Debug for VarDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for DeclSpecs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "<declaration specs>")
    }
  }
  impl Debug for DeclSpecs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for TypeSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        TypeSpecifier::Void => write!(f, "void"),
        TypeSpecifier::Char => write!(f, "char"),
        TypeSpecifier::Short => write!(f, "short"),
        TypeSpecifier::Int => write!(f, "int"),
        TypeSpecifier::Long => write!(f, "long"),
        TypeSpecifier::Float => write!(f, "float"),
        TypeSpecifier::Double => write!(f, "double"),
        TypeSpecifier::Signed => write!(f, "signed"),
        TypeSpecifier::Unsigned => write!(f, "unsigned"),
        TypeSpecifier::Bool => write!(f, "bool"),
        TypeSpecifier::Complex => write!(f, "complex"),
        TypeSpecifier::Typedef(name) => write!(f, "{}", name),
        TypeSpecifier::Struct(s) => write!(f, "struct {}", s),
        TypeSpecifier::Union(s) => write!(f, "union {}", s),
        TypeSpecifier::Enum(e) => write!(f, "enum {}", e),
      }
    }
  }
  impl Debug for TypeSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for Struct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "{}",
        match &self.name {
          Some(name) => name,
          None => "(unnamed)",
        }
      )
    }
  }
  impl Debug for Struct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
  impl Display for EnumSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "{}",
        match &self.name {
          Some(name) => name,
          None => "(unnamed)",
        }
      )
    }
  }
  impl Debug for EnumSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      <Self as Display>::fmt(self, f)
    }
  }
}
