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
  /// fixme: dont do [`ExprRef`] here but store a real expr so that we have cache locality.
  Scalar(ExprRef<'c>),
  List(InitializerList<'c>),
}
::rcc_utils::interconvert!(ExprRef, Initializer, 'c, Scalar);
::rcc_utils::interconvert!(InitializerList, Initializer, 'c, List);
#[derive(Debug)]
pub struct InitializerList<'c> {
  pub entries: &'c [InitializerListEntry<'c>],
  pub span: SourceSpan,
}

#[derive(Debug)]
pub struct InitializerListEntry<'c> {
  pub designators: &'c [Designator<'c>],
  pub initializer: Initializer<'c>,
  pub is_implicit: bool,
}

impl<'c> InitializerListEntry<'c> {
  pub fn new(
    designators: &'c [Designator<'c>],
    initializer: Initializer<'c>,
    is_implicit: bool,
  ) -> Self {
    Self {
      designators,
      initializer,
      is_implicit,
    }
  }
}

#[derive(Debug)]
pub enum Designator<'c> {
  Array(usize),
  Field(
    /* Field iterator or so.. */ ::std::marker::PhantomData<&'c u8>,
  ),
}

impl Designator<'_> {
  /// as error node. you can't use [`usize::MAX`] in subscript
  /// since it would require the array has [`usize::MAX`] + 1 length, which is impossible.
  /// Also nobody in SANE would allocate such a huge array...
  ///
  /// error node that:
  ///   1. ignore overrides once there's at least one [`Self::SENTINAL`] in [`Designator`]s.
  ///   2. during AST Dump, put the index as recovery/invalid.
  ///   3. which means every calc w.r.t. [`Designator`] shall use [`usize::saturating_add`]-like calc,
  ///      or checking whether the current operand is [`usize::MAX`].
  #[allow(non_upper_case_globals)]
  pub const npos: usize = usize::MAX;
  // #[allow(non_upper_case_globals)]
  // pub const nofield...
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
    Self {
      declarations: declarations
        .into_iter()
        .collect_in::<ArenaVec<_>>(context.arena())
        .into_bump_slice(),
    }
  }
}

impl<'c> Function<'c> {
  pub fn alloc(
    context: &'c Context<'c>,
    function: Function<'c>,
  ) -> FunctionRef<'c> {
    context.arena().alloc(function)
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
impl<'c> Initializer<'c> {
  pub fn span(&self) -> SourceSpan {
    ::rcc_utils::static_dispatch!(
      self,
      |variant| variant.span() =>
      Scalar List
    )
  }
}
impl<'c> InitializerList<'c> {
  pub fn new(
    entries: &'c [InitializerListEntry<'c>],
    span: SourceSpan,
  ) -> Self {
    Self { entries, span }
  }

  fn span(&self) -> SourceSpan {
    self.span
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
      ::rcc_utils::static_dispatch!(
        self,
        |variant| variant.fmt(f) =>
        Scalar List
      )
    }
  }

  impl<'c> Display for InitializerList<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{{")?;
      if !self.entries.is_empty() {
        write!(f, " ")?;
      }

      for (i, entry) in self.entries.iter().enumerate() {
        if i > 0 {
          write!(f, ", ")?;
        }

        for designator in entry.designators.iter() {
          match designator {
            Designator::Array(index) => write!(f, "[{}]", index)?,
            Designator::Field(_) => write!(f, ".<field>")?,
          }
        }
        write!(f, " = {}", entry.initializer)?;
      }

      if !self.entries.is_empty() {
        write!(f, " ")?;
      }
      write!(f, "}}")
    }
  }
}
