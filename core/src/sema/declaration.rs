use ::std::{cell::Ref, collections::HashSet};

use super::{expression::Expression, statement::Compound};
use crate::{
  common::{SourceSpan, StrRef, SymbolRef},
  types::{FunctionSpecifier, QualifiedType, Type},
};

#[derive(Debug)]
pub struct TranslationUnit<'c> {
  pub declarations: Vec<ExternalDeclaration<'c>>,
}
#[derive(Debug)]
pub enum ExternalDeclaration<'c> {
  Function(Function<'c>),
  Variable(VarDef<'c>),
}

#[derive(Debug)]
pub struct Function<'c> {
  /// contains name, storage, definition flag, and full QualifiedType.
  pub symbol: SymbolRef<'c>, // function type is included in symbol's qualified_type
  pub parameters: Vec<Parameter<'c>>, // some duplication with symbol's qualified_type, but we need this for param names
  pub specifier: FunctionSpecifier,
  pub body: Option<Compound<'c>>,
  pub labels: HashSet<StrRef<'c>>, // just holds a name
  pub gotos: HashSet<StrRef<'c>>,  // just holds a name
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct VarDef<'c> {
  pub symbol: SymbolRef<'c>,
  pub initializer: Option<Initializer<'c>>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Parameter<'c> {
  /// If the parameter is named, point to the symbol; otherwise the name was set to `<unnamed_n>`.
  pub symbol: SymbolRef<'c>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub enum Initializer<'c> {
  /// Simple scalar initialization: `int x = val;`
  Scalar(Expression<'c>),
  /// Aggregate initialization: `int arr[] = { 1, 2, 3 };`
  /// unimplemented: todo.
  Aggregate(Vec<Initializer<'c>>),
}
impl<'c> TranslationUnit<'c> {
  pub fn new(declarations: Vec<ExternalDeclaration<'c>>) -> Self {
    Self { declarations }
  }
}
impl<'c> Function<'c> {
  pub fn new(
    symbol: SymbolRef<'c>,
    parameters: Vec<Parameter<'c>>,
    specifier: FunctionSpecifier,
    body: Option<Compound<'c>>,
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
impl<'c> VarDef<'c> {
  pub fn new(
    symbol: SymbolRef<'c>,
    initializer: Option<Initializer<'c>>,
    span: SourceSpan,
  ) -> Self {
    Self {
      symbol,
      initializer,
      span,
    }
  }
}

impl<'c> Parameter<'c> {
  pub fn new(symbol: SymbolRef<'c>, span: SourceSpan) -> Self {
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

  impl<'c> Display for TranslationUnit<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      self
        .declarations
        .iter()
        .try_for_each(|decl| writeln!(f, "{}", decl))
    }
  }

  impl<'c> Display for ExternalDeclaration<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      ::rcc_utils::static_dispatch!(
        self,
        |variant| variant.fmt(f) =>
        Function Variable
      )
    }
  }

  impl<'c> Display for Function<'c> {
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

  impl<'c> Display for Parameter<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      // Show only the type for brevity; names are optional.
      write!(f, "{}", self.symbol.borrow().qualified_type)
    }
  }

  impl<'c> Display for VarDef<'c> {
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

  impl<'c> Display for Initializer<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      match self {
        Initializer::Scalar(expr) => write!(f, "{}", expr),
        Initializer::Aggregate(_) => todo!(),
      }
    }
  }
}
