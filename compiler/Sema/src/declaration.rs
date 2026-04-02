use ::rcc_ast::{
  Context,
  types::{FunctionSpecifier, QualifiedType, Type, TypeRef},
};
use ::rcc_shared::{ArenaVec, CollectIn, SourceSpan};
use ::rcc_utils::StrRef;

pub use crate::declref::DeclRef;
use crate::{expression::ExprRef, statement::Compound};

#[derive(Debug)]
pub struct TranslationUnit<'c> {
  pub declarations: &'c [ExternalDeclarationRef<'c>],
}

pub type FunctionRef<'c> = &'c Function<'c>;
pub type VarDefRef<'c> = &'c VarDef<'c>;

#[derive(Debug)]
pub enum ExternalDeclarationRef<'c> {
  Function(FunctionRef<'c>),
  Variable(VarDefRef<'c>),
}

#[derive(Debug)]
pub struct Function<'c> {
  pub declaration: DeclRef<'c>,
  pub parameters: &'c [Parameter<'c>],
  pub specifier: FunctionSpecifier,
  pub body: Option<Compound<'c>>,
  pub labels: &'c [StrRef<'c>],
  pub gotos: &'c [StrRef<'c>],
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct VarDef<'c> {
  pub declaration: DeclRef<'c>,
  pub initializer: Option<Initializer<'c>>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct Parameter<'c> {
  pub declaration: DeclRef<'c>,
  pub span: SourceSpan,
}

#[derive(Debug)]
pub enum Initializer<'c> {
  Scalar(ExprRef<'c>),
  Aggregate(&'c [Initializer<'c>]),
}

::rcc_utils::ensure_is_pod!(Initializer<'_>);
::rcc_utils::ensure_is_pod!(VarDef<'_>);
::rcc_utils::ensure_is_pod!(Parameter<'_>);
::rcc_utils::ensure_is_pod!(Function<'_>);
::rcc_utils::ensure_is_pod!(ExternalDeclarationRef<'_>);
::rcc_utils::ensure_is_pod!(TranslationUnit<'_>);

impl<'c> TranslationUnit<'c> {
  pub fn new(
    context: &'c Context<'c>,
    declarations: impl IntoIterator<Item = ExternalDeclarationRef<'c>>,
  ) -> Self {
    let declarations = declarations
      .into_iter()
      .collect_in::<ArenaVec<_>>(context.arena())
      .into_bump_slice();
    Self { declarations }
  }
}

impl<'c> Function<'c> {
  pub fn alloc(
    context: &'c Context<'c>,
    function: Function<'c>,
  ) -> FunctionRef<'c> {
    let function = context.arena().alloc(function);
    &*function
  }

  pub fn new(
    context: &'c Context<'c>,
    declaration: DeclRef<'c>,
    parameters: impl IntoIterator<Item = Parameter<'c>>,
    specifier: FunctionSpecifier,
    body: Option<Compound<'c>>,
    span: SourceSpan,
  ) -> Self {
    let parameters = parameters
      .into_iter()
      .collect_in::<ArenaVec<_>>(context.arena())
      .into_bump_slice();
    Self {
      declaration,
      parameters,
      specifier,
      body,
      labels: &[],
      gotos: &[],
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
  pub fn proto(&self) -> QualifiedType<'c> {
    self.declaration.qualified_type()
  }

  #[inline]
  pub fn proto_unqual(&self) -> TypeRef<'c> {
    self.declaration.qualified_type().unqualified_type
  }
}

impl<'c> VarDef<'c> {
  pub fn new(
    context: &'c Context<'c>,
    declaration: DeclRef<'c>,
    initializer: Option<Initializer<'c>>,
    span: SourceSpan,
  ) -> VarDefRef<'c> {
    context.arena().alloc(Self {
      declaration,
      initializer,
      span,
    })
  }
}

impl<'c> Parameter<'c> {
  pub fn new(declaration: DeclRef<'c>, span: SourceSpan) -> Self {
    Self { declaration, span }
  }
}

mod fmt {
  use ::std::fmt::Display;

  use super::*;

  impl<'c> Display for TranslationUnit<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      self
        .declarations
        .iter()
        .try_for_each(|decl| writeln!(f, "{}", decl))
    }
  }

  impl<'c> Display for ExternalDeclarationRef<'c> {
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
      match self.declaration.qualified_type().unqualified_type {
        Type::FunctionProto(proto) => {
          write!(f, "{} {}(", proto.return_type, self.declaration.name())?;
          for (index, param) in self.parameters.iter().enumerate() {
            if index > 0 {
              write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
          }
          write!(f, ")")?;
        },
        _ => {
          write!(
            f,
            "{} {}",
            self.declaration.qualified_type(),
            self.declaration.name()
          )?;
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
      write!(f, "{}", self.declaration.qualified_type())
    }
  }

  impl<'c> Display for VarDef<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      let decl = self.declaration;
      write!(
        f,
        "{} {} {}",
        decl.storage_class(),
        decl.qualified_type(),
        decl.name()
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
