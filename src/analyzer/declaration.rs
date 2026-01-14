use ::std::cell::Ref;

use crate::{
  analyzer::{expression::Expression, statement::Compound},
  common::{
    environment::SymbolRef,
    rawdecl::FunctionSpecifier,
    types::{QualifiedType, Type},
  },
};

#[derive(Debug)]
pub struct TranslationUnit {
  pub declarations: Vec<Declaration>,
}
#[derive(Debug)]
pub enum Declaration {
  Function(Function),
  Variable(VarDef),
}

#[derive(Debug)]
pub struct Function {
  /// contains name, storage, definition flag, and full QualifiedType.
  pub symbol: SymbolRef, // function type is included in symbol's qualified_type
  pub parameters: Vec<Parameter>, // some duplication with symbol's qualified_type, but we need this for param names
  pub specifier: FunctionSpecifier,
  pub body: Option<Compound>,
  // todo: static declarations inside function body, return path checking, goto stmt, irgen related info
  // pub gotos: Vec<Goto>,
  // pub labels: Vec<Label>,
}

#[derive(Debug)]
pub struct VarDef {
  pub symbol: SymbolRef,
  pub initializer: Option<Initializer>,
}

#[derive(Debug)]
pub struct Parameter {
  /// If the parameter is named, point to the symbol; otherwise None (abstract/unnamed parameter).
  pub symbol: Option<SymbolRef>,
}

#[derive(Debug)]
pub enum Initializer {
  /// Simple scalar initialization: `int x = val;`
  Scalar(Expression),
  /// Aggregate initialization: `int arr[] = { 1, 2, 3 };`
  /// unimplemented: todo.
  Aggregate(Vec<Initializer>),
}
impl TranslationUnit {
  pub fn new(declarations: Vec<Declaration>) -> Self {
    Self { declarations }
  }
}
impl Function {
  pub fn new(
    symbol: SymbolRef,
    parameters: Vec<Parameter>,
    specifier: FunctionSpecifier,
    body: Option<Compound>,
  ) -> Self {
    Self {
      symbol,
      parameters,
      specifier,
      body,
    }
  }

  #[inline]
  pub fn is_declaration(&self) -> bool {
    !self.is_definition()
  }

  #[inline]
  pub fn is_definition(&self) -> bool {
    self.body.is_some()
  }

  pub fn proto(&self) -> Ref<'_, QualifiedType> {
    Ref::map(self.symbol.borrow(), |sym| &sym.qualified_type)
  }

  pub fn proto_unqual(&self) -> Ref<'_, Type> {
    Ref::map(self.symbol.borrow(), |sym| {
      &sym.qualified_type.unqualified_type
    })
  }
}
impl VarDef {
  pub fn new(symbol: SymbolRef, initializer: Option<Initializer>) -> Self {
    Self {
      symbol,
      initializer,
    }
  }
}

impl Parameter {
  pub fn new(symbol: Option<SymbolRef>) -> Self {
    Self { symbol }
  }
}

mod fmt {

  use ::std::fmt::Display;

  use super::{
    Declaration, Function, Initializer, Parameter, TranslationUnit, VarDef,
  };
  use crate::common::types::Type;

  impl Display for TranslationUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      for decl in &self.declarations {
        writeln!(f, "{}", decl)?;
      }
      Ok(())
    }
  }

  impl Display for Declaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Declaration::Function(func) => <Function as Display>::fmt(func, f),
        Declaration::Variable(var) => <VarDef as Display>::fmt(var, f),
      }
    }
  }

  impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let sym = self.symbol.borrow();

      // For functions, sym.qualified_type is Type::FunctionProto
      // We want: return_type name(params)
      match &sym.qualified_type.unqualified_type {
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

  impl Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      // Show only the type for brevity; names are optional.
      write!(
        f,
        "{}",
        match &self.symbol {
          Some(sym) => sym.borrow().qualified_type.to_string(),
          None => "<unnamed>".to_string(),
        }
      )
    }
  }

  impl Display for VarDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let sym = self.symbol.borrow();
      write!(f, "{} {}", sym.qualified_type, sym.name)?;
      if let Some(initializer) = &self.initializer {
        write!(f, " = {}", initializer)?;
      }
      write!(f, ";")
    }
  }

  impl Display for Initializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Initializer::Scalar(expr) => write!(f, "{}", expr),
        Initializer::Aggregate(_) => todo!(),
      }
    }
  }
}
