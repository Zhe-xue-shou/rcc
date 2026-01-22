use ::rc_utils::interconvert;

use crate::{
  common::{Keyword, Literal, Storage},
  parser::{expression::Expression, statement::Compound},
  types::{FunctionSpecifier, Qualifiers},
};

#[derive(Debug, Default)]
pub struct Program {
  pub declarations: Vec<Declaration>,
}
/// declaration:
///       - declaration-specifiers init-declarator-list_opt ;
///       - attribute-specifier-sequence declaration-specifiers init-declarator-list ; (don't care)
///       - static_assert-declaration (don't care)
///       - attribute-declaration (don't care)
#[derive(Debug)]
pub enum Declaration {
  Function(Function),
  Variable(VarDef),
}

interconvert!(Function, Declaration);
interconvert!(VarDef, Declaration, Variable);

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
  /// declarator with no name. sizeof, typeof, cast, etc.
  Abstract,
  /// declarator with name. variable/function decl/def
  Named,
  /// indeterminate
  Maybe,
}
/// declarator:
///     pointer_opt direct-declarator
#[derive(Debug)]
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
#[derive(Debug)]
pub enum Modifier {
  Pointer(Qualifiers),
  Array(ArrayModifier),
  Function(FunctionSignature),
}
#[derive(Debug)]
pub struct Member {
  pub specifiers: Vec<TypeSpecifier>,
  pub qualifiers: Qualifiers,
  pub modifiers: Vec<Modifier>,
  pub declarator: Option<Declarator>,
  pub bit_width: Option<Expression>,
}
#[derive(Debug)]
pub struct Parameter {
  pub declspecs: DeclSpecs,
  pub declarator: Declarator,
}
#[derive(Debug)]
pub struct Struct {
  pub name: Option<String>,
  pub members: Vec<Member>,
}
/// type-specifier
#[derive(Debug)]
pub enum TypeSpecifier {
  Nullptr,
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

impl TypeSpecifier {
  pub fn sort_key(&self) -> u8 {
    match self {
      TypeSpecifier::Void => 0,
      TypeSpecifier::Unsigned => 1,
      TypeSpecifier::Signed => 2,
      TypeSpecifier::Char => 3,
      TypeSpecifier::Short => 4,
      TypeSpecifier::Long => 5,
      TypeSpecifier::Int => 6,
      TypeSpecifier::Float => 7,
      TypeSpecifier::Double => 8,
      TypeSpecifier::Bool => 9,
      TypeSpecifier::Complex => 10,
      TypeSpecifier::Typedef(_) => 11,
      TypeSpecifier::Nullptr => 12,
      TypeSpecifier::Struct(_) => 13,
      TypeSpecifier::Union(_) => 14,
      TypeSpecifier::Enum(_) => 15,
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
///    - declaration-specifier attribute-specifier-sequence_opt (don't care)
///    - declaration-specifier declaration-specifiers
///
/// declaration-specifier:
///    - storage-class-specifier
///    - type-specifier-qualifier
///    - function-specifier
#[derive(Debug)]
pub struct DeclSpecs {
  pub function_specifiers: FunctionSpecifier,
  pub storage_class: Option<Storage>,
  pub qualifiers: Qualifiers,
  pub type_specifiers: Vec<TypeSpecifier>,
}
#[derive(Debug)]
pub struct Function {
  pub declspecs: DeclSpecs,
  pub declarator: Declarator,
  pub body: Option<Compound>,
}
#[derive(Debug)]
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
#[derive(Debug)]
pub struct ArrayModifier {
  pub qualifiers: Qualifiers,
  pub is_static: bool,
  pub bound: ArrayBound,
}
#[derive(Debug)]
pub enum ArrayBound {
  Constant(usize),
  Variable(Expression),
  Incomplete,
}
/// function-declarator:
///     - direct-declarator ( parameter-type-list_opt )
#[derive(Debug)]
pub struct FunctionSignature {
  pub parameters: Vec<Parameter>,
  pub is_variadic: bool,
}
#[derive(Debug)]
pub enum Initializer {
  Expression(Box<Expression>),
  List(Vec<InitializerListEntry>),
}
#[derive(Debug)]
pub struct InitializerListEntry {
  pub designators: Vec<Designator>,
  pub value: Box<Initializer>,
}
#[derive(Debug)]
pub enum Designator {
  Member(String),
  Index(Expression),
}
#[derive(Debug)]
pub struct EnumSpecifier {
  pub name: Option<String>,
  pub enumerators: Vec<Enumerator>,
}
#[derive(Debug)]
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
  pub fn new(
    declspecs: DeclSpecs,
    declarator: Declarator,
    body: Option<Compound>,
  ) -> Self {
    Self {
      declspecs,
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
      parameters: Vec::default(),
      is_variadic: false,
    }
  }
}
impl Parameter {
  pub fn new(declspecs: DeclSpecs, declarator: Declarator) -> Self {
    Self {
      declspecs,
      declarator,
    }
  }
}
impl Program {
  pub fn new() -> Self {
    Self {
      declarations: Vec::default(),
    }
  }
}
impl Declarator {
  pub fn new(name: Option<String>) -> Self {
    Self {
      name,
      modifiers: Vec::default(),
    }
  }

  pub fn decltype(&self) -> DeclaratorType {
    match &self.name {
      Some(_) => DeclaratorType::Named,
      None => DeclaratorType::Abstract,
    }
  }
}
impl ::core::default::Default for DeclSpecs {
  fn default() -> Self {
    Self {
      function_specifiers: FunctionSpecifier::empty(),
      storage_class: None,
      qualifiers: Qualifiers::empty(),
      type_specifiers: Vec::default(),
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

  use ::std::fmt::Display;

  use super::{
    DeclSpecs, Declaration, Declarator, EnumSpecifier, Function,
    FunctionSignature, Modifier, Program, Struct, TypeSpecifier, VarDef,
  };

  impl Display for Declaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Declaration::Function(func) => <Function as Display>::fmt(func, f),
        Declaration::Variable(var) => <VarDef as Display>::fmt(var, f),
      }
    }
  }

  impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      self
        .declarations
        .iter()
        .try_for_each(|decl| writeln!(f, "{}", decl))
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
        },
        Modifier::Array(_) => todo!(),
        Modifier::Function(function_signature) =>
          <FunctionSignature as Display>::fmt(function_signature, f),
      }
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
            .declspecs
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
            Some(_) => " = <initializer>".to_string(),
            None => "".to_string(),
          }
        )?;
        write!(f, ";")
      }
    }
  }
  impl Display for DeclSpecs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{} ", self.function_specifiers)?;
      if let Some(storage) = &self.storage_class {
        write!(f, "{} ", storage)?;
      }
      write!(
        f,
        "{}",
        self
          .type_specifiers
          .iter()
          .map(|s| s.to_string())
          .collect::<Vec<_>>()
          .join(" ")
      )
    }
  }
  impl Display for Declarator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "{} {}",
        match &self.name {
          Some(name) => name,
          None => "<anonymous>",
        },
        self
          .modifiers
          .iter()
          .map(|m| m.to_string())
          .collect::<Vec<_>>()
          .join(" ")
      )
    }
  }
  impl Display for TypeSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        TypeSpecifier::Nullptr => write!(f, "nullptr"),
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
}
