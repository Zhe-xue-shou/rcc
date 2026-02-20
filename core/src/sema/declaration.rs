use ::std::{cell::Ref, collections::HashSet};

use super::{expression::Expression, statement::Compound};
use crate::{
  common::{SourceSpan, StrRef, SymbolRef},
  types::{FunctionSpecifier, QualifiedType, Type},
};

#[derive(Debug)]
pub struct TranslationUnit<'context> {
  pub declarations: Vec<ExternalDeclaration<'context>>,
}
#[derive(Debug)]
pub enum ExternalDeclaration<'context> {
  Function(Function<'context>),
  Variable(VarDef<'context>),
}

#[derive(Debug)]
pub struct Function<'context> {
  /// contains name, storage, definition flag, and full QualifiedType.
  pub symbol: SymbolRef<'context>, // function type is included in symbol's qualified_type
  pub parameters: Vec<Parameter<'context>>, // some duplication with symbol's qualified_type, but we need this for param names
  pub specifier: FunctionSpecifier,
  pub body: Option<Compound<'context>>,
  pub labels: HashSet<StrRef<'context>>, // just holds a name
  pub gotos: HashSet<StrRef<'context>>,  // just holds a name
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct VarDef<'context> {
  pub symbol: SymbolRef<'context>,
  pub initializer: Option<Initializer<'context>>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Parameter<'context> {
  /// If the parameter is named, point to the symbol; otherwise the name was set to `<unnamed_n>`.
  pub symbol: SymbolRef<'context>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub enum Initializer<'context> {
  /// Simple scalar initialization: `int x = val;`
  Scalar(Expression<'context>),
  /// Aggregate initialization: `int arr[] = { 1, 2, 3 };`
  /// unimplemented: todo.
  Aggregate(Vec<Initializer<'context>>),
}
impl<'context> TranslationUnit<'context> {
  pub fn new(declarations: Vec<ExternalDeclaration<'context>>) -> Self {
    Self { declarations }
  }
}
impl<'context> Function<'context> {
  pub fn new(
    symbol: SymbolRef<'context>,
    parameters: Vec<Parameter<'context>>,
    specifier: FunctionSpecifier,
    body: Option<Compound<'context>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      symbol,
      parameters,
      specifier,
      body,
      labels: HashSet::new(),
      gotos: HashSet::new(),
      span,
    }
  }

  #[inline(always)]
  pub fn is_declaration(&self) -> bool {
    !self.is_definition()
  }

  #[inline(always)]
  pub fn is_definition(&self) -> bool {
    self.body.is_some()
  }

  #[inline]
  pub fn proto(&self) -> Ref<'_, QualifiedType<'_>> {
    Ref::map(self.symbol.borrow(), |sym| &sym.qualified_type)
  }

  #[inline]
  pub fn proto_unqual(&self) -> Ref<'_, Type<'_>> {
    Ref::map(self.symbol.borrow(), |sym| {
      sym.qualified_type.unqualified_type
    })
  }
}
impl<'context> VarDef<'context> {
  pub fn new(
    symbol: SymbolRef<'context>,
    initializer: Option<Initializer<'context>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      symbol,
      initializer,
      span,
    }
  }
}

impl<'context> Parameter<'context> {
  pub fn new(symbol: SymbolRef<'context>, span: SourceSpan) -> Self {
    Self { symbol, span }
  }
}

mod fmt {

  use ::std::fmt::Display;

  use super::{
    ExternalDeclaration, Function, Initializer, Parameter, TranslationUnit,
    VarDef,
  };
  use crate::types::Type;

  impl<'context> Display for TranslationUnit<'context> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      self
        .declarations
        .iter()
        .try_for_each(|decl| writeln!(f, "{}", decl))
    }
  }

  impl<'context> Display for ExternalDeclaration<'context> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      ::rcc_utils::static_dispatch!(
        self.fmt(f),
        Function Variable
      )
    }
  }

  impl<'context> Display for Function<'context> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      let sym = self.symbol.borrow();

      // For functions, sym.qualified_type is Type::FunctionProto
      // We want: return_type name(params)
      match *sym.qualified_type.unqualified_type {
        Type::FunctionProto(proto) => {
          write!(f, "{} {}(", proto.return_type, sym.name)?;
          for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
              write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
          }
          write!(f, ")")?;
        },
        _ => {
          // Fallback for non-function types (shouldn't happen)
          write!(f, "{} {}", sym.qualified_type, sym.name)?;
        },
      }

      if let Some(body) = &self.body {
        write!(f, " {}", body)
      } else {
        write!(f, ";")
      }
    }
  }

  impl<'context> Display for Parameter<'context> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      // Show only the type for brevity; names are optional.
      write!(f, "{}", self.symbol.borrow().qualified_type)
    }
  }

  impl<'context> Display for VarDef<'context> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      let sym = self.symbol.borrow();
      write!(
        f,
        "{} {} {}",
        sym.storage_class, sym.qualified_type, sym.name
      )?;
      if let Some(initializer) = &self.initializer {
        write!(f, " = {}", initializer)?;
      }
      write!(f, ";")
    }
  }

  impl<'context> Display for Initializer<'context> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      match self {
        Initializer::Scalar(expr) => write!(f, "{}", expr),
        Initializer::Aggregate(_) => todo!(),
      }
    }
  }
}
