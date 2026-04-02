use ::rcc_ast::types::{FunctionSpecifier, Qualifiers};
use ::rcc_shared::{Keyword, Literal, SourceSpan, Storage};
use ::rcc_utils::{StrRef, interconvert};

use crate::{expression::Expression, statement::Compound};

#[derive(Debug, Default)]
pub struct Program<'c> {
  pub declarations: Vec<Declaration<'c>>,
}
/// declaration:
///       - declaration-specifiers init-declarator-list_opt ;
///       - attribute-specifier-sequence declaration-specifiers init-declarator-list ; (don't care)
///       - static_assert-declaration (don't care)
///       - attribute-declaration (don't care)
#[derive(Debug)]
pub enum Declaration<'c> {
  Function(Function<'c>),
  Variable(VarDef<'c>),
}

interconvert!(Function, Declaration, 'c);
interconvert!(VarDef, Declaration, 'c, Variable);

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
pub struct Declarator<'c> {
  pub name: Option<StrRef<'c>>,
  pub modifiers: Vec<Modifier<'c>>, // pointer, array, function
  pub span: SourceSpan,
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
/// ignore attrs for now
///
/// this is flatten structure, so the order of `Vec<Modifier>` in `Declarator` matters
/// and usually applied in reverse order
#[derive(Debug)]
pub enum Modifier<'c> {
  Pointer(Qualifiers),
  Array(ArrayModifier<'c>),
  Function(FunctionSignature<'c>),
}
#[derive(Debug)]
pub struct Member<'c> {
  pub specifiers: Vec<TypeSpecifier<'c>>,
  pub qualifiers: Qualifiers,
  pub modifiers: Vec<Modifier<'c>>,
  pub declarator: Option<Declarator<'c>>,
  pub bit_width: Option<Expression<'c>>,
}
#[derive(Debug)]
pub struct Parameter<'c> {
  pub declspecs: DeclSpecs<'c>,
  pub declarator: Declarator<'c>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct Struct<'c> {
  pub name: Option<StrRef<'c>>,
  pub members: Vec<Member<'c>>,
}
/// type-specifier
#[derive(Debug)]
pub enum TypeSpecifier<'c> {
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
  Typedef(StrRef<'c>),
  // vvv below should be wrong, but now don't care
  Struct(Struct<'c>),
  Union(Struct<'c>),
  Enum(EnumSpecifier<'c>),
  /// there's a dedicated class
  /// [`AutoType`](https://github.com/llvm/llvm-project/blob/4f9d5a8bc85431b722e6f90744f3683adffc17b4/clang/include/clang/AST/TypeBase.h#L7155)
  /// in clang's frontend, while im just only holds it as placeholder since the logic i implemented is simple.
  AutoType,
}

impl<'c> TypeSpecifier<'c> {
  pub fn sort_key(&self) -> u8 {
    use TypeSpecifier::*;
    match self {
      Void => 0,
      Unsigned => 1,
      Signed => 2,
      Char => 3,
      Short => 4,
      Long => 5,
      Int => 6,
      Float => 7,
      Double => 8,
      Bool => 9,
      Complex => 10,
      Typedef(_) => 11,
      Nullptr => 12,
      Struct(_) => 13,
      Union(_) => 14,
      Enum(_) => 15,
      AutoType => 16,
    }
  }

  /// builtin type specifier(i.e., keyword types) can combine with each other,
  /// typedef-ed type, struct, union, enum cannot.
  pub fn is_builtin(&self) -> bool {
    use TypeSpecifier::*;
    !matches!(self, Typedef(_) | Struct(_) | Union(_) | Enum(_))
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
pub struct DeclSpecs<'c> {
  pub function_specifiers: FunctionSpecifier,
  pub storage_class: Option<Storage>,
  pub qualifiers: Qualifiers,
  pub type_specifiers: Vec<TypeSpecifier<'c>>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct Function<'c> {
  pub declspecs: DeclSpecs<'c>,
  pub declarator: Declarator<'c>,
  pub body: Option<Compound<'c>>,
  pub span: SourceSpan,
}
#[derive(Debug)]
pub struct VarDef<'c> {
  pub declspecs: DeclSpecs<'c>,
  pub declarator: Declarator<'c>,
  pub initializer: Option<Initializer<'c>>,
  pub span: SourceSpan,
}
/// array-declarator:
///     - direct-declarator \[ type-qualifier-list_opt assignment-expression_opt \]
///     - direct-declarator \[ static type-qualifier-list_opt assignment-expression \]
///     - direct-declarator \[ type-qualifier-list static assignment-expression \]
///     - direct-declarator \[ type-qualifier-list_opt * \]
#[derive(Debug)]
pub struct ArrayModifier<'c> {
  pub qualifiers: Qualifiers,
  pub is_static: bool,
  pub bound: Option<Expression<'c>>,
  pub span: SourceSpan,
}
/// function-declarator:
///     - direct-declarator ( parameter-type-list_opt )
#[derive(Debug, Default)]
pub struct FunctionSignature<'c> {
  pub parameters: Vec<Parameter<'c>>,
  pub is_variadic: bool,
}
/// braced-initializer:
///     - { }
///     - { initializer-list   }
///     - { initializer-list , }
///
/// initializer:
///     - assignment-expression
///     - braced-initializer
#[derive(Debug)]
pub enum Initializer<'c> {
  Expression(Expression<'c>),
  InitializerList(InitializerList<'c>),
}
/// initializer-list:
///     - designation_opt initializer
///     - initializer-list , designation_opt initializer
#[derive(Debug)]
pub struct InitializerList<'c> {
  pub entries: Vec<InitializerListEntry<'c>>,
  pub span: SourceSpan,
}

::rcc_utils::interconvert!(Expression, Initializer, 'c);
::rcc_utils::interconvert!(InitializerList, Initializer, 'c);

impl<'c> InitializerList<'c> {
  pub fn new(entries: Vec<InitializerListEntry<'c>>, span: SourceSpan) -> Self {
    Self { entries, span }
  }
}
#[derive(Debug)]
pub enum InitializerListEntry<'c> {
  Designated(Designated<'c>),
  Initializer(Initializer<'c>),
}

::rcc_utils::interconvert!(Designated, InitializerListEntry, 'c);
::rcc_utils::interconvert!(Initializer, InitializerListEntry, 'c);
/// designation:
///     - designator-list =
///
/// designator-list:
///     - designator
///     - designator-list designator
#[derive(Debug)]
pub struct Designated<'c> {
  pub designators: Vec<Designator<'c>>,
  pub initializer: Initializer<'c>,
  /// from `.` or `[` to `=`.
  pub designator_sloc: SourceSpan,
}

impl<'c> Designated<'c> {
  pub fn new(
    designators: Vec<Designator<'c>>,
    initializer: Initializer<'c>,
    designator_sloc: SourceSpan,
  ) -> Self {
    Self {
      designators,
      initializer,
      designator_sloc,
    }
  }
}

/// designator:
///     - \[ constant-expression \]
///     - . identifier
#[derive(Debug)]
pub enum Designator<'c> {
  Field(StrRef<'c>),
  Index(Expression<'c>),
}
#[derive(Debug)]
pub struct EnumSpecifier<'c> {
  pub name: Option<StrRef<'c>>,
  pub enumerators: Vec<Enumerator<'c>>,
}
#[derive(Debug)]
pub struct Enumerator<'c> {
  pub name: StrRef<'c>,
  pub value: Option<Expression<'c>>,
}
impl<'c> Enumerator<'c> {
  pub fn new(name: StrRef<'c>, value: Option<Expression<'c>>) -> Self {
    Self { name, value }
  }
}
impl<'c> EnumSpecifier<'c> {
  pub fn new(
    name: Option<StrRef<'c>>,
    enumerators: Vec<Enumerator<'c>>,
  ) -> Self {
    Self { name, enumerators }
  }
}
impl<'c> ArrayModifier<'c> {
  pub fn new(
    qualifiers: Qualifiers,
    is_static: bool,
    bound: Option<Expression<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      qualifiers,
      is_static,
      bound,
      span,
    }
  }
}
impl<'c> Function<'c> {
  pub fn new(
    declspecs: DeclSpecs<'c>,
    declarator: Declarator<'c>,
    body: Option<Compound<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      declspecs,
      declarator,
      body,
      span,
    }
  }
}
impl<'c> FunctionSignature<'c> {
  pub fn new(parameters: Vec<Parameter<'c>>, is_variadic: bool) -> Self {
    Self {
      parameters,
      is_variadic,
    }
  }
}
impl<'c> Parameter<'c> {
  pub fn new(
    declspecs: DeclSpecs<'c>,
    declarator: Declarator<'c>,
    span: SourceSpan,
  ) -> Self {
    Self {
      declspecs,
      declarator,
      span,
    }
  }
}
impl<'c> Program<'c> {
  pub fn new() -> Self {
    Self {
      declarations: Vec::default(),
    }
  }
}
impl<'c> DeclSpecs<'c> {
  pub fn new(
    storage_class: Option<Storage>,
    qualifiers: Qualifiers,
    type_specifiers: Vec<TypeSpecifier<'c>>,
    function_specifiers: FunctionSpecifier,
    span: SourceSpan,
  ) -> Self {
    Self {
      storage_class,
      qualifiers,
      type_specifiers,
      function_specifiers,
      span,
    }
  }
}
impl<'c> Declarator<'c> {
  pub fn new(
    name: Option<StrRef<'c>>,
    modifiers: Vec<Modifier<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      name,
      modifiers,
      span,
    }
  }

  pub fn decltype(&self) -> DeclaratorType {
    match &self.name {
      Some(_) => DeclaratorType::Named,
      None => DeclaratorType::Abstract,
    }
  }
}
impl<'c> VarDef<'c> {
  pub fn new(
    declspecs: DeclSpecs<'c>,
    declarator: Declarator<'c>,
    initializer: Option<Initializer<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      declspecs,
      declarator,
      initializer,
      span,
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
mod cvt {
  use super::*;

  impl<'c> TryFrom<&Keyword> for TypeSpecifier<'c> {
    type Error = ();

    fn try_from(kw: &Keyword) -> Result<Self, Self::Error> {
      use TypeSpecifier::*;
      match kw {
        Keyword::Void => Ok(Void),
        Keyword::Char => Ok(Char),
        Keyword::Short => Ok(Short),
        Keyword::Int => Ok(Int),
        Keyword::Long => Ok(Long),
        Keyword::Float => Ok(Float),
        Keyword::Double => Ok(Double),
        Keyword::Signed => Ok(Signed),
        Keyword::Unsigned => Ok(Unsigned),
        Keyword::Bool => Ok(Bool),
        Keyword::AutoType => Ok(AutoType),
        _ => Err(()),
      }
    }
  }
  impl<'c> TryFrom<&Literal<'c>> for TypeSpecifier<'c> {
    type Error = ();

    fn try_from(literal: &Literal) -> Result<Self, Self::Error> {
      match literal {
        Literal::Keyword(kw) => TypeSpecifier::try_from(kw),
        _ => Err(()),
      }
    }
  }
}
mod fmt {

  use ::std::fmt::Display;

  use super::{
    ArrayModifier, DeclSpecs, Declaration, Declarator, Designated, Designator,
    EnumSpecifier, Function, FunctionSignature, Initializer, InitializerList,
    InitializerListEntry, Modifier, Program, Struct, TypeSpecifier, VarDef,
  };

  impl<'c> Display for Declaration<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      match self {
        Declaration::Function(func) => <Function as Display>::fmt(func, f),
        Declaration::Variable(var) => <VarDef as Display>::fmt(var, f),
      }
    }
  }

  impl<'c> Display for Program<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      self
        .declarations
        .iter()
        .try_for_each(|decl| writeln!(f, "{}", decl))
    }
  }
  impl<'c> Display for Function<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
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

  impl<'c> Display for Modifier<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      match self {
        Modifier::Pointer(qualifiers) => {
          write!(
            f,
            "{}*",
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
        Modifier::Array(array_modifier) =>
          <ArrayModifier as Display>::fmt(array_modifier, f),
        Modifier::Function(function_signature) =>
          <FunctionSignature as Display>::fmt(function_signature, f),
      }
    }
  }

  impl<'c> Display for ArrayModifier<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      write!(f, "[")?;
      if self.is_static {
        write!(f, "static ")?;
      }
      if !self.qualifiers.is_empty() {
        write!(
          f,
          "{} ",
          self
            .qualifiers
            .iter()
            .map(|q| q.to_string())
            .collect::<Vec<_>>()
            .join(" ")
        )?;
      }
      if let Some(bound) = &self.bound {
        write!(f, "{}", bound)?;
      } else {
        write!(f, "*")?;
      }
      write!(f, "]")
    }
  }

  impl<'c> Display for FunctionSignature<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
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
  impl<'c> Display for VarDef<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      write!(f, "{} {}", self.declspecs, self.declarator)?;
      match self.initializer {
        Some(ref initializer) => {
          write!(f, " = ")?;
          initializer.fmt(f)
        },
        None => Ok(()),
      }
    }
  }
  impl<'c> Display for DeclSpecs<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      if !self.function_specifiers.is_empty() {
        write!(f, "{} ", self.function_specifiers)?;
      }
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
  impl<'c> Display for Declarator<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      write!(
        f,
        "{} {}",
        self.name.unwrap_or("<anonymous>"),
        self
          .modifiers
          .iter()
          .rev()
          .map(|m| m.to_string())
          .collect::<Vec<_>>()
          .join(" ")
      )
    }
  }
  impl<'c> Display for TypeSpecifier<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      use TypeSpecifier::*;
      match self {
        Nullptr => write!(f, "nullptr"),
        Void => write!(f, "void"),
        Char => write!(f, "char"),
        Short => write!(f, "short"),
        Int => write!(f, "int"),
        Long => write!(f, "long"),
        Float => write!(f, "float"),
        Double => write!(f, "double"),
        Signed => write!(f, "signed"),
        Unsigned => write!(f, "unsigned"),
        Bool => write!(f, "bool"),
        Complex => write!(f, "complex"),
        Typedef(name) => write!(f, "{}", name),
        Struct(s) => write!(f, "struct {}", s),
        Union(s) => write!(f, "union {}", s),
        Enum(e) => write!(f, "enum {}", e),
        AutoType => write!(f, "__auto_type"),
      }
    }
  }
  impl<'c> Display for Struct<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
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
  impl<'c> Display for EnumSpecifier<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
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

  impl<'c> Display for Initializer<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      ::rcc_utils::static_dispatch!(
        self,
        |variant| variant.fmt(f) =>
        Expression InitializerList
      )
    }
  }
  impl<'c> Display for InitializerList<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{{ ")?;
      for (i, entry) in self.entries.iter().enumerate() {
        if i > 0 {
          write!(f, ", ")?;
        }
        entry.fmt(f)?;
      }
      write!(f, " }}")
    }
  }
  impl<'c> Display for InitializerListEntry<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      ::rcc_utils::static_dispatch!(
        self,
        |variant| variant.fmt(f) =>
        Designated Initializer
      )
    }
  }
  impl<'c> Display for Designated<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      self.designators.iter().try_for_each(|d| d.fmt(f))?;
      write!(f, " = ")?;
      self.initializer.fmt(f)
    }
  }
  impl<'c> Display for Designator<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Designator::Field(name) => write!(f, ".{}", name),
        Designator::Index(expr) => write!(f, "[{}]", expr),
      }
    }
  }
}
