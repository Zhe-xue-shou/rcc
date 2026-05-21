use ::rcc_adt::Integral;
use ::rcc_ast::{
  Context, Session, SessionRef, VarDeclKind,
  types::{
    Array, ArraySize, CastType::*, Compatibility, FunctionProto,
    FunctionSpecifier, Pointer, Primitive, QualifiedType, Qualifiers, Type,
    TypeInfo, UnqualExt,
  },
};
use ::rcc_memory::{ArenaVec, CollectIn};
use ::rcc_parse::{declaration as pd, expression as pe, statement as ps};
use ::rcc_shared::{
  Diag,
  DiagData::{self, *},
  Diagnosis, Linkage, OpDiag, Operator, OperatorCategory, Severity, SourceSpan,
  Storage, StorageSpecifier as SS,
};
use ::rcc_utils::{
  Opaque, RefEq, StrRef, contract_violation, not_implemented_feature,
};
use ::std::collections::{HashMap, HashSet};

use super::{declaration as sd, expression as se, statement as ss};
use crate::{
  declaration::DeclRef, expression::ExprRef, folding::Folder,
  initialization::Initialization,
};

#[derive(Debug)]
pub(crate) enum ScopeContext {
  Function,
  Loop,
  Switch,
}

type DeclScopeAssoc<'c> = HashMap<StrRef<'c>, sd::DeclRef<'c>>;

#[derive(Debug, Default)]
pub(crate) struct DeclEnvironment<'c> {
  scopes: Vec<DeclScopeAssoc<'c>>,
}

impl<'c> DeclEnvironment<'c> {
  fn enter(&mut self) {
    self.scopes.push(Default::default());
  }

  fn exit(&mut self) {
    self.scopes.pop();
  }

  pub(crate) fn is_global(&self) -> bool {
    self.scopes.len() == 1
  }

  pub(crate) fn find(&self, name: StrRef<'c>) -> Option<sd::DeclRef<'c>> {
    for scope in self.scopes.iter().rev() {
      if let Some(&declaration) = scope.get(name) {
        return Some(declaration);
      }
    }
    None
  }

  fn shallow_find(&self, name: StrRef<'c>) -> Option<sd::DeclRef<'c>> {
    self
      .scopes
      .last()
      .and_then(|scope| scope.get(name).copied())
  }

  /// find declnode from root, excluding the last scope.
  fn extern_find(&self, name: StrRef<'c>) -> Option<sd::DeclRef<'c>> {
    debug_assert!(self.scopes.len() >= 2,);
    for scope in self.scopes.iter().take(self.scopes.len() - 1) {
      if let Some(&declaration) = scope.get(name) {
        return Some(declaration);
      }
    }
    None
  }

  fn declare(
    &mut self,
    name: StrRef<'c>,
    declaration: sd::DeclRef<'c>,
  ) -> Option<DeclRef<'c>> {
    self
      .scopes
      .last_mut()
      .shall_ok("No scope to declare symbol")
      .insert(name, declaration)
  }
}

pub struct Sema<'c> {
  pub(crate) environment: DeclEnvironment<'c>,
  current_function: Option<sd::DeclRef<'c>>,
  current_labels: HashSet<StrRef<'c>>,
  current_gotos: HashSet<StrRef<'c>>,
  scope_context: Vec<ScopeContext>,
  pub(crate) session: SessionRef<'c>,

  pub(crate) __empty_expr: se::ExprRef<'c>,
  pub(crate) __empty_stmt: ss::StmtRef<'c>,
}
impl<'a> ::std::ops::Deref for Sema<'a> {
  type Target = Session<'a, OpDiag<'a>>;

  fn deref(&self) -> &'a Self::Target {
    self.session
  }
}
impl<'c> Sema<'c> {
  pub fn new(session: SessionRef<'c>) -> Self {
    Self {
      session,
      environment: Default::default(),
      current_function: Default::default(),
      current_labels: Default::default(),
      current_gotos: Default::default(),
      scope_context: Default::default(),
      __empty_expr: se::Expression::new_error_node(
        session,
        session.int_type().into(),
      ),
      __empty_stmt: ss::Statement::alloc(session.ast(), Default::default()),
    }
  }

  pub fn add_diag(&self, diag: Diag<'c>) {
    self.diag().add_diag(diag);
  }

  pub fn add_error(&self, error: DiagData<'c>, span: SourceSpan) {
    self.diag().add_error(error, span);
  }

  pub fn add_warning(&self, warning: DiagData<'c>, span: SourceSpan) {
    self.diag().add_warning(warning, span);
  }

  pub fn analyze(
    mut self,
    program: pd::Program<'c>,
  ) -> sd::TranslationUnit<'c> {
    self.environment.enter();
    let translation_unit =
      sd::TranslationUnit::new(self.ast(), self.externaldecl(program));

    self.environment.exit();
    debug_assert!(self.environment.scopes.is_empty());

    translation_unit
  }
}
type ParsedDeclspecs<'c> = (FunctionSpecifier, SS, QualifiedType<'c>);

impl<'c> Sema<'c> {
  /// IMPORTANT: currently, caller shoould check:
  /// 1. whether the `restrict` is valid; (it's only valid for pointers and non-static local variable.)
  /// 2. the type is complete or not, via [`TypeInfo::size`].
  fn apply_modifiers_for_varty<I>(
    &self,
    mut qualified_type: QualifiedType<'c>,
    modifiers: I,
  ) -> QualifiedType<'c>
  where
    I: IntoIterator<Item = pd::Modifier<'c>>,
    I::IntoIter: DoubleEndedIterator,
  {
    // reverse order
    modifiers
      .into_iter()
      .rev()
      .for_each(|modifier| match modifier {
        pd::Modifier::Pointer(qualifiers) =>
          qualified_type =
            self.apply_pointer_modifier(qualified_type, qualifiers),
        pd::Modifier::Array(array_modifier) =>
          qualified_type =
            self.apply_array_modifier(qualified_type, array_modifier),
        pd::Modifier::Function(function_signature) =>
          qualified_type =
            self.apply_function_modifier(qualified_type, function_signature),
      });
    qualified_type
  }

  fn apply_pointer_modifier(
    &self,
    qualified_type: QualifiedType<'c>,
    qualifiers: Qualifiers,
  ) -> QualifiedType<'c> {
    QualifiedType::new(
      qualifiers,
      Type::Pointer(Pointer::new(qualified_type)).lookup(self),
    )
  }

  fn apply_function_modifier(
    &self,
    qualified_type: QualifiedType<'c>,
    function_signature: pd::FunctionSignature<'c>,
  ) -> QualifiedType<'c> {
    let pd::FunctionSignature {
      parameters,
      is_variadic,
    } = function_signature;
    let analyzed_parameter_types = self.parse_parameter_types(parameters);
    self
      .intern(FunctionProto::new(
        qualified_type,
        analyzed_parameter_types.into_bump_slice(),
        is_variadic,
      ))
      .into()
  }

  fn apply_array_modifier(
    &self,
    qualified_type: QualifiedType<'c>,
    array_modifier: pd::ArrayModifier<'c>,
  ) -> QualifiedType<'c> {
    let size = match array_modifier.bound {
      None => ArraySize::Incomplete,
      Some(expr) => {
        // check 1. it's a constant expression or not, 2. it's type should
        // be integer type 3. should be non-negative
        match self.expression(expr) {
          Ok(analyzed_expr) if analyzed_expr.qualified_type().is_scalar() =>
            match self.folder(false, analyzed_expr) {
              Some(value) =>
                if value.is_integer_constant() {
                  ArraySize::Constant(
                    value
                      .as_constant_unchecked()
                      .as_integral_unchecked()
                      .to_builtin::<usize>()
                      .into(),
                  )
                } else {
                  self.add_error(
                    NonIntegerInArraySubscript(value.to_string()),
                    value.span(),
                  );
                  ArraySize::ONE
                },
              None => match self.folder(true, analyzed_expr) {
                Some(value) =>
                  if value.is_integer_constant() {
                    self.add_warning(ConstVLAFolds, value.span());
                    ArraySize::Constant(
                      value
                        .as_constant_unchecked()
                        .as_integral_unchecked()
                        .to_builtin::<usize>()
                        .into(),
                    )
                  } else {
                    self.add_error(
                      NonIntegerInArraySubscript(value.to_string()),
                      value.span(),
                    );
                    ArraySize::ONE
                  },
                None =>
                  if self.environment.is_global() {
                    self.add_error(GlobalVLA, analyzed_expr.span());
                    ArraySize::ONE
                  } else {
                    self.add_error(
                      UnsupportedFeature(
                        "Expression could not be evaluated to a constant; VLA \
                         is not supported currently."
                          .to_string(),
                      ),
                      analyzed_expr.span(),
                    );
                    ArraySize::Variable(Opaque::new(analyzed_expr))
                  },
              },
            },
          Ok(analyzed_expr) => {
            self.add_error(
              NonIntegerInArraySubscript(analyzed_expr.to_string()),
              analyzed_expr.span(),
            );
            ArraySize::ONE // error case
          },
          Err(diag) => {
            self.add_diag(diag);
            ArraySize::ONE
          },
        }
      },
    };
    if !qualified_type.is_complete(self) {
      self.add_error(
        ArrayHasIncompleteType(qualified_type.to_string()),
        array_modifier.span,
      );
    }
    Type::Array(Array::new(qualified_type, size))
      .lookup(self)
      .into()
  }

  fn apply_modifiers_for_functiondecl(
    &self,
    base_return_type: QualifiedType<'c>,
    modifiers: Vec<pd::Modifier<'c>>,
  ) -> Result<
    (
      QualifiedType<'c>,
      /* parameters name and their type, here's some repetition
      parameter type had also been inside QualifiedType of the function */
      &'c [sd::DeclRef<'c>],
    ),
    Diag<'c>,
  > {
    let mut it = modifiers.into_iter();
    let Some(pd::Modifier::Function(function_signature)) = it.next() else {
      panic!("function declarator should have at least one modifier(function)")
    };

    let return_type = self.apply_modifiers_for_varty(base_return_type, it);
    // we need to build function type
    let parameters = self.parse_parameters(function_signature.parameters);
    let parameter_types = parameters
      .iter()
      .map(|param| {
        let qualified_type = param.qualified_type;
        if !qualified_type.is_complete(self) {
          self.add_error(
            VariableIncompleteType(param.name, qualified_type.to_string()),
            param.span,
          );
        }
        qualified_type
      })
      .collect_in::<ArenaVec<_>>(self.arena());
    Ok((
      self
        .make_function_proto(
          return_type,
          parameter_types.into_bump_slice(),
          function_signature.is_variadic,
        )
        .into(),
      parameters,
    ))
  }

  fn parse_parameter_types(
    &self,
    parameters: Vec<pd::Parameter<'c>>,
  ) -> ArenaVec<'c, QualifiedType<'c>> {
    parameters
      .into_iter()
      .map(|parameter| {
        let pd::Parameter {
          declarator,
          declspecs,
          span: _,
        } = parameter;
        let (_fs, storage_specs, base_type) = self.parse_declspecs(declspecs);
        ::std::assert_matches!(
          storage_specs,
          SS::Empty | SS::Auto | SS::Register,
          "pls report add_error()"
        );
        // strictly speaking the names shall be unique but it doesnt matter here really.
        let pd::Declarator {
          modifiers,
          name: _,
          span: _,
        } = declarator;
        self
          .apply_modifiers_for_varty(base_type, modifiers)
          .parameter_adjustment(self)
      })
      .collect_in(self.arena())
  }

  fn parse_parameters(
    &self,
    parameters: Vec<pd::Parameter<'c>>,
  ) -> &'c [sd::DeclRef<'c>] {
    parameters
      .into_iter()
      .map(|parameter| {
        let pd::Parameter {
          declarator,
          declspecs,
          span,
        } = parameter;
        let (_fs, storage_specs, base_type) = self.parse_declspecs(declspecs);

        ::std::assert_matches!(
          storage_specs,
          SS::Empty | SS::Auto | SS::Register,
          "pls report add_error()"
        );
        let pd::Declarator {
          modifiers,
          name,
          span: _,
        } = declarator;
        let qualified_type = self
          .apply_modifiers_for_varty(base_type, modifiers)
          .parameter_adjustment(self);
        sd::ExternalDeclaration::new_canonical(
          self.arena(),
          name.unwrap_or("<unnamed>"),
          qualified_type,
          Linkage::None,
          VarDeclKind::Declaration,
          sd::VarDef::decl(Storage::Automatic).into(),
          span,
        )
      })
      .collect_in::<ArenaVec<_>>(self.arena())
      .into_bump_slice()
  }

  fn parse_declspecs(
    &self,
    declspecs: pd::DeclSpecs<'c>,
  ) -> ParsedDeclspecs<'c> {
    let qualified_type = self
      .get_type(declspecs.type_specifiers)
      .handle_with(self, self.int_type().into())
      .with_qualifiers(declspecs.qualifiers, self);
    let storage_class = declspecs.storage_specifiers;
    let function_specifier = declspecs.function_specifiers;

    (function_specifier, storage_class, qualified_type)
  }

  fn get_type(
    &self,
    mut type_specifiers: Vec<pd::TypeSpecifier<'c>>,
  ) -> Result<QualifiedType<'c>, Diag<'c>> {
    assert!(!type_specifiers.is_empty());
    assert!(type_specifiers.len() <= 5); // unsigned long long int complex (integer complex not in standard) is the max
    type_specifiers.sort_by_key(pd::TypeSpecifier::sort_key);
    use pd::TypeSpecifier::*;
    // 6.7.3p1
    match type_specifiers.as_slice() {
      [AutoType] => Ok(self.auto_type().into()),
      [Nullptr] => Ok(self.nullptr_type().into()),
      [Void] => Ok(self.void_type().into()),

      [Bool] => Ok(self.i8_bool_type().into()),

      [Char] => Ok(self.char_type().into()),
      [Signed, Char] => Ok(self.schar_type().into()),
      [Unsigned, Char] => Ok(self.uchar_type().into()),

      [Short] | [Short, Int] | [Signed, Short] | [Signed, Short, Int] =>
        Ok(self.short_type().into()),
      [Unsigned, Short] | [Unsigned, Short, Int] =>
        Ok(self.ushort_type().into()),

      [Int] | [Signed] | [Signed, Int] => Ok(self.int_type().into()),
      [Unsigned] | [Unsigned, Int] => Ok(self.uint_type().into()),

      [Long] | [Long, Int] | [Signed, Long] | [Signed, Long, Int] =>
        Ok(self.long_type().into()),
      [Unsigned, Long] | [Unsigned, Long, Int] => Ok(self.ulong_type().into()),

      [Long, Long]
      | [Long, Long, Int]
      | [Signed, Long, Long]
      | [Signed, Long, Long, Int] => Ok(self.long_long_type().into()),
      [Unsigned, Long, Long] | [Unsigned, Long, Long, Int] =>
        Ok(self.ulong_long_type().into()),

      [Float] => Ok(self.float32_type().into()),
      [Double] => Ok(self.float64_type().into()),
      [Long, Double] =>
        Ok(Type::Primitive(Primitive::LongDouble).lookup(self).into()),

      [Float, Complex] =>
        Ok(Type::Primitive(Primitive::ComplexFloat).lookup(self).into()),
      [Double, Complex] => Ok(
        Type::Primitive(Primitive::ComplexDouble)
          .lookup(self)
          .into(),
      ),
      [Long, Double, Complex] => Ok(
        Type::Primitive(Primitive::ComplexLongDouble)
          .lookup(self)
          .into(),
      ),

      // treat complex integers as error
      [Char, Complex]
      | [Signed, Char, Complex]
      | [Unsigned, Char, Complex]
      | [Short, Complex]
      | [Short, Int, Complex]
      | [Signed, Short, Complex]
      | [Signed, Short, Int, Complex]
      | [Unsigned, Short, Complex]
      | [Unsigned, Short, Int, Complex]
      | [Int, Complex]
      | [Signed, Complex]
      | [Signed, Int, Complex]
      | [Unsigned, Complex]
      | [Unsigned, Int, Complex]
      | [Long, Complex]
      | [Long, Int, Complex]
      | [Signed, Long, Complex]
      | [Signed, Long, Int, Complex]
      | [Unsigned, Long, Complex]
      | [Unsigned, Long, Int, Complex] => {
        not_implemented_feature!("Complex integer types are not supported");
      },

      [Typedef(t)] => {
        let typedef = self.environment.find(t).shall_ok("identifier not found");
        if typedef.is_typedef() {
          Ok(typedef.qualified_type)
        } else {
          contract_violation!("identifier is not a typedef");
        }
      },
      // skip _BitInt, _Decimal32, _Decimal64, _Decimal128 here
      _ => not_implemented_feature!("{:#?}", type_specifiers.as_slice()),
    }
  }
}

impl<'c> Sema<'c> {
  fn externaldecl(
    &mut self,
    program: pd::Program<'c>,
  ) -> impl IntoIterator<Item = sd::DeclRef<'c>> {
    program.declarations.into_iter().flat_map(|declaration| {
      self.declarations(declaration.declspecs, declaration.init_declarators)
    })
  }

  pub fn declarations(
    &mut self,
    declspecs: pd::DeclSpecs<'c>,
    init_declarators: Vec<pd::InitDeclarator<'c>>,
  ) -> Vec<sd::DeclRef<'c>> {
    let parsed_declspecs = self.parse_declspecs(declspecs);
    init_declarators
      .into_iter()
      .filter_map(|init_declarator| {
        match self.declaration(parsed_declspecs, init_declarator) {
          Ok(declaration) => Some(declaration),
          Err(diag) => {
            self.add_diag(diag);
            None
          },
        }
      })
      .collect()
  }

  pub fn declaration(
    &mut self,
    parsed_declspecs: ParsedDeclspecs<'c>,
    init_declarator: pd::InitDeclarator<'c>,
  ) -> Result<sd::DeclRef<'c>, Diag<'c>> {
    match init_declarator {
      pd::InitDeclarator::Function(function) =>
        self.functiondecl(parsed_declspecs, function),
      pd::InitDeclarator::Variable(vardef) =>
        self.vardef(parsed_declspecs, vardef),
    }
  }

  pub fn functiondecl(
    &mut self,
    parsed_declspecs: ParsedDeclspecs<'c>,
    function: pd::Function<'c>,
  ) -> Result<sd::DeclRef<'c>, Diag<'c>> {
    let pd::Function {
      body,
      declarator,
      span,
    } = function;
    let (function_specifier, storage_specifier, base_return_type) =
      parsed_declspecs;
    let pd::Declarator {
      modifiers,
      name,
      span: _,
    } = declarator;

    let name = name
      .shall_ok("function must have a name; it should be handled in parser");

    let (qualified_type, parameters) = self
      .apply_modifiers_for_functiondecl(base_return_type, modifiers)
      .shall_ok("failed to apply modifiers for function declarator");

    use VarDeclKind::*;

    let declkind = if body.is_some() {
      Definition
    } else {
      Declaration
    };

    let previous_decl = self.environment.find(name);
    let (declaration_type, linkage) = {
      let linkage = if self.environment.is_global() {
        match storage_specifier {
          SS::Empty | SS::Extern => Linkage::External,
          SS::Static => Linkage::Internal,
          _ => {
            self.add_error(
              Custom(format!(
                "invalid storage classifier '{storage_specifier}' for function"
              )),
              span,
            );
            Linkage::External
          },
        }
      } else {
        if !storage_specifier.is_empty() && storage_specifier != SS::Extern {
          self.add_error(
            Custom(format!(
              "function decl at block scope shall have either extern or no \
               storage specifier. got '{}'",
              storage_specifier
            )),
            span,
          );
        }
        Linkage::External
      };
      if let Some(previous_decl) = previous_decl {
        if !Compatibility::compatible(
          &previous_decl.qualified_type,
          &qualified_type,
        ) {
          Err(
            IncompatibleType(
              name,
              previous_decl.qualified_type.to_string(),
              qualified_type.to_string(),
            ) + Severity::Error
              + span,
          )?;
        }

        debug_assert!(
          !matches!(previous_decl.declkind, Tentative),
          "function cannot be tentative"
        );

        if matches!(declkind, Definition)
          && previous_decl.definition().is_some()
        {
          Err(FunctionAlreadyDefined(name) + Severity::Error + span)?
        }
        (
          Compatibility::composite_unchecked(
            &previous_decl.qualified_type,
            &qualified_type,
            self,
          ),
          Linkage::try_merge(previous_decl.linkage, linkage).unwrap_or_else(
            |e| {
              self.add_error(StorageSpecsUnmergeable(e), span);
              previous_decl.linkage
            },
          ),
        )
      } else {
        (qualified_type, linkage)
      }
    };

    if name == "main" {
      Context::main_proto_validate(
        self,
        linkage,
        qualified_type.as_functionproto_unchecked(),
        function_specifier,
      )
      .unwrap_or_else(|e| {
        self.add_diag(e + span);
      });
    }

    let function = sd::ExternalDeclaration::new(
      self.arena(),
      previous_decl,
      name,
      declaration_type,
      linkage,
      declkind,
      sd::Function::new_decl(function_specifier, parameters).into(),
      span,
    );

    if !function.qualified_type.is_complete(self) {
      self.add_error(
        VariableIncompleteType(name, function.qualified_type.to_string()),
        span,
      );
    } else {
      self.environment.declare(name, function);
    }

    match body {
      Some(body) => match self.current_function {
        Some(_) => contract_violation!(
          "nested function definition is not allowed; 
          this should be handled in parser: current function {}, new function \
           {}
          
          Also: this may occur if the `current_function` is not properly \
           cleared 
          after an `Err` returned of the previous function definition analysis",
          self.current_function.as_ref().unwrap().name,
          function.name
        ),
        None => self.function_with_body(body, function),
      },
      None => Ok(function),
    }
  }

  fn function_with_body(
    &mut self,
    body: ps::Compound<'c>,
    function: sd::DeclRef<'c>,
  ) -> Result<sd::DeclRef<'c>, Diag<'c>> {
    self.current_labels.clear();
    self.current_gotos.clear();
    self.current_function = Some(function);

    self.environment.enter();
    self.scope_context.push(ScopeContext::Function);

    self
      .current_function
      .as_ref()
      .shall_ok("shall have function")
      .as_function_unchecked()
      .parameters
      .iter()
      .for_each(|parameter| {
        // FIXME: hsould we insert unnamed parameters or not?
        if parameter.name.starts_with('<') {
          // unnamed parameter - do nothing currently
        } else {
          // if it's incomplete type, this has already reported when building the functionproto.
          self.environment.declare(parameter.name, parameter);
        }
      });

    let statements = self.statements(body.statements);

    let _func = self.scope_context.pop();
    self.environment.exit();

    use ::std::debug_assert_matches;
    debug_assert_matches!(_func, Some(ScopeContext::Function));

    let function_span = self
      .current_function
      .as_ref()
      .shall_ok("impossible; no current function?")
      .span;

    self.current_gotos.iter().for_each(|goto| {
      if !self.current_labels.contains(goto) {
        self.add_error(LabelNotFound(goto), function_span);
      }
    });

    let labels = self
      .current_labels
      .iter()
      .copied()
      .collect_in::<ArenaVec<_>>(self.arena())
      .into_bump_slice();
    let gotos = self
      .current_gotos
      .iter()
      .copied()
      .collect_in::<ArenaVec<_>>(self.arena())
      .into_bump_slice();
    let analyzed_body = ss::Compound::new(self, statements, body.span);

    {
      let function = self
        .current_function
        .as_mut()
        .shall_ok("impossible; no current function?");
      function
        .as_function_unchecked()
        .body
        .replace(Some(analyzed_body));
      function.as_function_unchecked().labels.set(labels);
      function.as_function_unchecked().gotos.set(gotos);
    }

    self.current_labels.clear();
    self.current_gotos.clear();
    let function =
      ::std::mem::take(&mut self.current_function).shall_ok("never fails");
    Ok(function)
  }

  /// FIXME: [`SS::ThreadLocal`] is inaccurate.
  pub fn vardef(
    &mut self,
    parsed_declspecs: ParsedDeclspecs<'c>,
    vardef: pd::VarDef<'c>,
  ) -> Result<sd::DeclRef<'c>, Diag<'c>> {
    let pd::VarDef {
      declarator,
      initializer,
      span,
    } = vardef;
    let pd::Declarator {
      modifiers,
      name,
      span: _,
    } = declarator;

    let (_function_specifier, storage_specifier, raw_qualified_type) =
      parsed_declspecs;
    debug_assert!(
      _function_specifier.is_empty(),
      "variable cannot have function specifier; this should be handled in \
       parser"
    );
    let previous_decl = name.and_then(|name| {
      if !self.environment.is_global() && storage_specifier.contains(SS::Extern)
      {
        self.environment.extern_find(name)
      } else {
        self.environment.shallow_find(name)
      }
    });

    if storage_specifier.contains(SS::ThreadLocal) {
      self.add_error(
        UnsupportedFeature("ThreadLocal unimplemented".to_string()),
        span,
      );
    }

    let name = name.unwrap_or("<unnamed>");
    let is_named_constant = storage_specifier.contains(SS::Constexpr)
      || (self.langopts().is_sysy()
        && raw_qualified_type.contains(Qualifiers::Const));

    let (qualified_type, initializer) = {
      let requires_folding = self.environment.is_global()
        || storage_specifier
          .intersects(SS::Constexpr | SS::Static | SS::ThreadLocal);

      let (qtype, init) =
        if RefEq::ref_eq(&*raw_qualified_type, self.auto_type()) {
          let out = initializer.map(|initializer| {
            self.initializer(initializer, None, requires_folding)
          });
          if let Some((initializer, qualified_type)) = out {
            (qualified_type, Some(initializer))
          } else {
            Err(DeducedTypeWithNoInitializer(name) + Severity::Error + span)?
          }
        } else {
          let qualified_type =
            self.apply_modifiers_for_varty(raw_qualified_type, modifiers);
          if self.environment.is_global() && qualified_type.has_vla_dim() {
            self.add_error(TopLevelVLA(name), span);
            // skip init part...
            (qualified_type, None)
          } else if let Some((initializer, qualified_type)) =
            initializer.map(|initializer| {
              self.initializer(
                initializer,
                Some(qualified_type),
                requires_folding,
              )
            })
          {
            (qualified_type, Some(initializer))
          } else {
            (qualified_type, None)
          }
        };
      if is_named_constant && init.is_none() {
        self.add_warning(
          Custom(format!(
            "constexpr variable '{name}' shall have initializer"
          )),
          span,
        );
      }
      let constexpr_check = |qtype: QualifiedType<'c>| {
        if is_named_constant {
          if qtype
            .qualifiers()
            .intersects(Qualifiers::Atomic | Qualifiers::Volatile)
          {
            self.add_error(
              Custom(format!(
                "constexpr variable '{name}' shall not have qualifier '{}'",
                qtype.qualifiers()
              )),
              span,
            );
            qtype
              .without_qualifiers()
              .with_qualifiers(Qualifiers::Const, self)
          } else {
            qtype.with_qualifiers(Qualifiers::Const, self)
          }
        } else {
          qtype
        }
      };
      if let Some(prev_decl_ref) = previous_decl {
        let prev_qtype = prev_decl_ref.qualified_type;

        if !Compatibility::compatible(&prev_qtype, &qtype) {
          Err(
            IncompatibleType(name, prev_qtype.to_string(), qtype.to_string())
              + Severity::Error
              + span,
          )?
        } else {
          if prev_decl_ref.definition().is_some() && init.is_some() {
            self.add_error(VariableAlreadyDefined(name), span)
          }
          (
            constexpr_check(Compatibility::composite_unchecked(
              &prev_qtype,
              &qtype,
              self,
            )),
            init,
          )
        }
      } else {
        (constexpr_check(qtype), init)
      }
    };

    // arr can have 1st extend incomplete, handled downstream at init
    if !qualified_type.is_complete(self) && !qualified_type.is_array() {
      Err(
        DeclarationTyIncomplete(name.into(), qualified_type.to_string())
          + Severity::Error
          + span,
      )?;
    }

    use Linkage as L;
    use Storage as S;

    let vardef = {
      let merge = |incoming: Linkage| {
        if let Some(prev) = previous_decl {
          Linkage::try_merge(prev.linkage, incoming).unwrap_or_else(|e| {
            self.add_error(StorageSpecsUnmergeable(e), span);
            prev.linkage
          })
        } else {
          incoming
        }
      };
      let typedef = || {
        sd::ExternalDeclaration::new(
          self.arena(),
          previous_decl,
          name,
          qualified_type,
          L::None,
          VarDeclKind::Declaration,
          sd::Typedef::new().into(),
          span,
        )
      };
      if self.environment.is_global() {
        if name == "main" && storage_specifier.contains(SS::Extern) {
          self.add_warning(
            MainFunctionProtoMismatch(
              "a variable that has name 'main' with externel linkage is \
               undefined behavior.",
            ),
            span,
          );
        }

        let ten = |incoming: Linkage, storage: Storage| {
          sd::ExternalDeclaration::new(
            self.arena(),
            previous_decl,
            name,
            qualified_type,
            merge(incoming),
            VarDeclKind::Tentative,
            sd::VarDef::new(None, is_named_constant, storage).into(),
            span,
          )
        };
        let decl = |incoming: Linkage, storage: Storage| {
          sd::ExternalDeclaration::new(
            self.arena(),
            previous_decl,
            name,
            qualified_type,
            merge(incoming),
            VarDeclKind::Declaration,
            sd::VarDef::decl(storage).into(),
            span,
          )
        };
        let def = |incoming: Linkage,
                   storage: Storage,
                   initializer: sd::Initializer<'c>| {
          sd::ExternalDeclaration::new(
            self.arena(),
            previous_decl,
            name,
            qualified_type,
            merge(incoming),
            VarDeclKind::Definition,
            sd::VarDef::def(initializer, is_named_constant, storage).into(),
            span,
          )
        };
        match (storage_specifier, initializer) {
          // 6.9.3p2, tentative.
          (SS::Empty, None) => ten(L::Common, S::Static),
          (
            SS::Auto | SS::Register | SS::AutoConstexpr | SS::RegisterConstexpr,
            None,
          ) => {
            self.add_error(GlobalVar(storage_specifier), span);
            ten(L::Common, S::Static)
          },
          (SS::Static | SS::StaticConstexpr, None) =>
            ten(L::Internal, S::Static),
          // error reported above
          (SS::Constexpr, None) => ten(L::Internal, S::Static),
          // declaration.
          (SS::ThreadLocal | SS::StaticThreadLocal, None) =>
            decl(L::Internal, S::ThreadLocal),
          (SS::Extern, None) =>
            if let Some(prev) = previous_decl {
              decl(
                L::External,
                prev.as_variable().map(|v| v.storage).unwrap_or(S::Static),
              )
            } else {
              decl(L::External, S::Static)
            },
          (SS::ExternThreadLocal, None) => decl(L::External, S::ThreadLocal),
          // definition.
          (SS::Empty | SS::Constexpr, Some(init)) =>
            def(L::External, S::Static, init),
          (SS::Static | SS::StaticConstexpr, Some(init)) =>
            def(L::Internal, S::Static, init),
          (SS::ThreadLocal | SS::StaticThreadLocal, Some(init)) =>
            def(L::Internal, S::ThreadLocal, init),
          (SS::Extern | SS::ExternThreadLocal, Some(init)) => {
            self.add_warning(ExternVariableWithInitializer(name), span);
            def(L::External, S::ThreadLocal, init)
          },
          // definition w/ errors.
          (
            SS::Auto | SS::Register | SS::AutoConstexpr | SS::RegisterConstexpr,
            Some(init),
          ) => {
            self.add_error(GlobalVar(storage_specifier), span);
            def(L::Common, S::Static, init)
          },
          (SS::Typedef, Some(_init)) => {
            self.add_error(
              Custom("typedef symbol cannot have initializer.".to_string()),
              span,
            );
            typedef()
          },
          (SS::Typedef, None) => typedef(),
          _ => {
            self.add_error(
              Custom(format!("Unknown storage specs {storage_specifier}")),
              span,
            );
            decl(L::Internal, S::Static)
          },
        }
      } else {
        if (qualified_type
          .as_array()
          .is_some_and(|array| !array.is_complete(self)))
          && initializer.is_none()
        {
          self.add_error(
            IncompleteArrayDefNoInit(name, qualified_type.to_string()),
            span,
          );
        }
        if storage_specifier.contains(SS::Extern) && initializer.is_some() {
          self.add_error(ExternVariableWithInitializer(name), span);
        }
        let local = |incoming: Linkage, storage: Storage| {
          sd::ExternalDeclaration::new(
            self.arena(),
            previous_decl,
            name,
            qualified_type,
            merge(incoming),
            if storage_specifier.contains(SS::Extern) {
              VarDeclKind::Declaration
            } else {
              VarDeclKind::Definition
            },
            sd::VarDef::new(initializer, is_named_constant, storage).into(),
            span,
          )
        };
        match storage_specifier {
          SS::Empty
          | SS::Auto
          | SS::Register
          | SS::Constexpr
          | SS::AutoConstexpr
          | SS::RegisterConstexpr => local(L::None, S::Automatic),
          SS::Extern =>
            if let Some(decl) = previous_decl {
              local(
                decl.linkage,
                decl.as_variable().map(|v| v.storage).unwrap_or(S::Static),
              )
            } else {
              local(L::External, S::Static)
            },
          SS::Static | SS::StaticConstexpr => local(L::Internal, S::Static),
          // FIXME: block scope thread_local is also considered External Definitions... here ignoring...
          SS::ThreadLocal | SS::StaticThreadLocal =>
            local(L::Internal, S::ThreadLocal),
          SS::ExternThreadLocal => local(L::External, S::ThreadLocal),
          SS::Typedef => typedef(),
          _ => {
            self.add_error(
              Custom(format!("Unknown storage specs {storage_specifier}")),
              span,
            );
            local(L::None, S::Automatic)
          },
        }
      }
    };

    if !vardef.qualified_type.is_complete(self)
      && !matches!(vardef.linkage, L::External | L::Common)
    {
      self.add_error(
        VariableIncompleteType(name, vardef.qualified_type.to_string()),
        span,
      );
    } else {
      self.environment.declare(name, vardef);
    }
    Ok(vardef)
  }

  pub fn initializer(
    &self,
    initializer: pd::Initializer<'c>,
    target_type: Option<QualifiedType<'c>>,
    requires_folding: bool,
  ) -> (sd::Initializer<'c>, QualifiedType<'c>) {
    Initialization::new(self, requires_folding).doit(initializer, target_type)
  }

  pub fn folder(
    &self,
    relaxed_static_const_var: bool,
    expression: ExprRef<'c>,
  ) -> Option<ExprRef<'c>> {
    Folder::new(self, relaxed_static_const_var, expression).doit()
  }
}

impl<'c> Sema<'c> {
  pub(crate) fn expression(
    &self,
    expression: pe::Expression<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    ::stacker::maybe_grow(32 * 1024, 1024 * 1024, || match expression {
      pe::Expression::Empty(_) => Ok(self.__empty_expr),
      pe::Expression::Constant(constant) => self.constant(constant),
      pe::Expression::Unary(unary) => self.unary(unary),
      pe::Expression::Binary(binary) => self.binary(binary),
      pe::Expression::Variable(variable) => self.variable(variable),
      pe::Expression::Call(call) => self.call(call),
      pe::Expression::Paren(paren) => self.paren(paren),
      pe::Expression::Ternary(ternary) => self.ternary(ternary),
      pe::Expression::SizeOf(sizeof) => self.sizeof(sizeof),
      pe::Expression::CStyleCast(cast) => self.cast(cast),
      pe::Expression::MemberAccess(member_access) =>
        self.member_access(member_access),
      pe::Expression::ArraySubscript(array_subscript) =>
        self.array_subscript(array_subscript),
      pe::Expression::CompoundLiteral(compound_literal) =>
        self.compound_literal(compound_literal),
    })
  }

  fn sizeof(
    &self,
    sizeof: pe::SizeOf<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    match sizeof.sizeof {
      pe::SizeOfKind::Expression(expression) => {
        let analyzed_expr = self.expression(*expression).handle_with(
          self,
          se::Expression::new_error_node(self, self.uintptr_type().into()),
        );
        let size = analyzed_expr.unqualified_type().size(self);
        Ok(se::Expression::new_rvalue(
          self,
          se::RawExpr::Constant(se::Constant::Integral(Integral::from(
            size.to_builtin::<usize>(),
          ))),
          self.uintptr_type().into(),
          SourceSpan {
            end: analyzed_expr.span().end,
            ..sizeof.span
          },
        ))
      },
      pe::SizeOfKind::Type(unprocessed_type) => {
        let pe::UnprocessedType {
          declspecs,
          declarator,
        } = *unprocessed_type;
        let qualified_type = {
          let (_, _, base_type) = self.parse_declspecs(declspecs);
          self.apply_modifiers_for_varty(base_type, declarator.modifiers)
        };
        Ok(se::Expression::new_rvalue(
          self,
          se::RawExpr::Constant(se::Constant::Integral(Integral::from(
            qualified_type.size(self).to_builtin::<usize>(),
          ))),
          self.uintptr_type().into(),
          sizeof.span,
        ))
      },
    }
  }

  fn call(&self, call: pe::Call<'c>) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let pe::Call {
      arguments,
      callee,
      span,
    } = call;
    let analyzed_callee = self.expression(*callee)?.lvalue_conversion(self);

    let function_proto = match analyzed_callee.unqualified_type() {
      Type::FunctionProto(proto) => proto,
      Type::Pointer(ptr) => match &*ptr.pointee {
        Type::FunctionProto(proto) => proto,
        _ =>
          Err(InvalidCallee(ptr.pointee.to_string()) + Severity::Error + span)?,
      },
      _ => Err(
        InvalidCallee(analyzed_callee.qualified_type().to_string())
          + Severity::Error
          + span,
      )?,
    };

    let mut analyzed_arguments = Vec::new();
    for argument in arguments {
      analyzed_arguments.push(self.expression(argument)?);
    }

    if !function_proto.is_variadic
      && analyzed_arguments.len() != function_proto.parameter_types.len()
    {
      contract_violation!("argument count mismatch");
    }
    let expr_type = function_proto.return_type;

    let converted_analyzed_arguments = analyzed_arguments
      .into_iter()
      .zip(function_proto.parameter_types)
      .map(|(actual, formal)| {
        actual
          .lvalue_conversion(self)
          .decay(self)
          .assignment_conversion(self, formal)
          .handle_with(self, actual)
      });
    Ok(se::Expression::new_rvalue(
      self,
      se::Call::new(self, analyzed_callee, converted_analyzed_arguments),
      expr_type,
      span,
    ))
  }

  fn paren(&self, paren: pe::Paren<'c>) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let pe::Paren { expr, span } = paren;
    let analyzed_expr = self.expression(*expr)?;
    let expr_type = *analyzed_expr.qualified_type();
    Ok(se::Expression::new_rvalue(
      self,
      se::Paren::new(analyzed_expr),
      expr_type,
      span,
    ))
  }

  fn cast(&self, _: pe::CStyleCast) -> Result<se::ExprRef<'c>, Diag<'c>> {
    not_implemented_feature!("C-style cast is not implemented yet");
  }

  fn variable(
    &self,
    variable: pe::Variable<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let declaration = self.environment.find(variable.name).ok_or(
      UndefinedVariable(variable.name) + Severity::Error + variable.span,
    )?;
    if declaration.is_typedef() {
      Err(
        Custom(format!(
          "Unexpected type name {}, expect a variable.",
          variable.name
        )) + Severity::Error
          + variable.span,
      )
    } else {
      Ok(se::Expression::new_lvalue(
        self,
        se::Variable::new(declaration),
        declaration.qualified_type,
        variable.span,
      ))
    }
  }

  fn constant(
    &self,
    constant: pe::Constant<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let pe::Constant {
      inner: constant,
      span,
    } = constant;
    let unqualified_type = constant.unqualified_type(self.ast());
    let value_category = if constant.is_char_array() {
      // 6.5.2p5: A string literal is [...] an lvalue [...].
      se::ValueCategory::LValue
    } else {
      se::ValueCategory::RValue
    };
    Ok(se::Expression::new(
      self,
      constant,
      unqualified_type.into(),
      value_category,
      span,
    ))
  }

  fn unary(&self, unary: pe::Unary<'c>) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let pe::Unary {
      operator,
      operand: pe_expr,
      kind,
      span,
    } = unary;
    let operand = self.expression(*pe_expr)?;
    match operator {
      Operator::Ampersand => self.addressof(operator, operand, span),
      Operator::Star => self.indirect(operator, operand, span),
      Operator::Not => self.logical_not(operator, operand, span),
      Operator::Tilde => self.tilde(operator, operand, span),
      Operator::Plus | Operator::Minus =>
        self.unary_arithmetic(operator, operand, span),
      Operator::PlusPlus | Operator::MinusMinus =>
        self.ppmm(operator, operand, kind, span),
      _ => unreachable!("operator is not unary: {:#?}", operator),
    }
  }

  fn binary(
    &self,
    binary: pe::Binary<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let pe::Binary {
      left: pe_left,
      operator,
      right: pe_right,
      span,
    } = binary;
    let left = self.expression(*pe_left)?;
    let right = self.expression(*pe_right)?;
    self.do_binary(operator, left, right, span)
  }

  fn do_binary(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    use OperatorCategory::*;
    macro_rules! call {
      ($method:ident) => {
        self.$method(operator, left, right, span)
      };
    }
    match operator.category() {
      Assignment => call!(assignment),
      Logical => call!(logical),
      Relational => call!(relational),
      Arithmetic => call!(arithmetic),
      Bitwise => call!(bitwise),
      BitShift => call!(bitshift),
      Special => call!(comma),
      Uncategorized => unreachable!("operator is not binary: {:#?}", operator),
    }
  }

  fn ternary(
    &self,
    ternary: pe::Ternary<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let pe::Ternary {
      condition: pe_condition,
      then_expr: pe_then_expr,
      else_expr: pe_else_expr,
      span,
    } = ternary;
    let condition = self
      .expression(*pe_condition)?
      .lvalue_conversion(self)
      .decay(self)
      .is_contextually_convertible_to_bool()?;

    if let Some(then) = pe_then_expr {
      let then_expr = self.expression(*then)?;
      let else_expr = self.expression(*pe_else_expr)?;

      match (then_expr.unqualified_type(), else_expr.unqualified_type()) {
        (left_type, right_type)
          if left_type.is_void() || right_type.is_void() =>
          Ok(se::Expression::new_rvalue(
            self,
            se::Ternary::new(
              condition,
              se::Expression::void_conversion(then_expr, self),
              se::Expression::void_conversion(else_expr, self),
            ),
            self.void_type().into(),
            span,
          )),
        // both arithmetic -> usual arithmetic conversion
        (left_type, right_type)
          if left_type.is_arithmetic() && right_type.is_arithmetic() =>
        {
          let (then_converted, else_converted, result_type) =
            se::Expression::usual_arithmetic_conversion(
              then_expr, else_expr, self,
            )?;
          Ok(se::Expression::new_rvalue(
            self,
            se::Ternary::new(condition, then_converted, else_converted),
            result_type,
            span,
          ))
        },
        // both pointer to compatible type -> composite type
        (Type::Pointer(left_ptr), Type::Pointer(right_ptr)) =>
          match Compatibility::composite(
            &left_ptr.pointee,
            &right_ptr.pointee,
            self,
          ) {
            Some(qualified_type) => {
              let result_type = Type::Pointer(Pointer::new(qualified_type))
                .lookup(self)
                .into();
              Ok(se::Expression::new_rvalue(
                self,
                se::Ternary::new(condition, then_expr, else_expr),
                result_type,
                span,
              ))
            },
            None => Err(
              IncompatiblePointerTypes(
                then_expr.qualified_type().to_string(),
                else_expr.qualified_type().to_string(),
              ) + Severity::Error
                + span,
            ),
          },
        _ => todo!(),
      }
    } else {
      todo!()
    }
  }

  fn member_access(
    &self,
    _member_access: pe::MemberAccess,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    todo!()
  }

  fn array_subscript(
    &self,
    array_subscript: pe::ArraySubscript<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    // a[i] = *(a + i)
    let pe::ArraySubscript { array, index, span } = array_subscript;
    let lhs = self.expression(*array)?.lvalue_conversion(self).decay(self);
    let rhs = self.expression(*index)?.lvalue_conversion(self).decay(self);

    let doit = |array_side: se::ExprRef<'c>, index_side: se::ExprRef<'c>| {
      let analyzed_index = index_side; // .ptrdiff_conversion_unchecked(self);
      let elem_type =
        array_side.unqualified_type().as_pointer_unchecked().pointee;
      // store the pointer(decayed array) and index here, not the array here... maybe a wrong idesa, idk for now.
      se::Expression::new_lvalue(
        self,
        se::ArraySubscript::new(array_side, analyzed_index),
        elem_type,
        span,
      )
    };

    match (lhs.unqualified_type(), rhs.unqualified_type()) {
      (Type::Pointer(_), Type::Primitive(p)) if p.is_integer() =>
        Ok(doit(lhs, rhs)),
      (Type::Primitive(p), Type::Pointer(_)) if p.is_integer() =>
        Ok(doit(rhs, lhs)),

      (Type::Pointer(_), t) | (t, Type::Pointer(_)) =>
        Err(NonIntegerSubscript(t.to_string()) + Severity::Error + span),
      _ => Err(DerefNonPtr(lhs.to_string()) + Severity::Error + span),
    }
  }

  fn compound_literal(
    &self,
    _compound_literal: pe::CompoundLiteral,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    todo!()
  }
}
impl<'c> Sema<'c> {
  /// unary arithmetic operators: `+`, `-`
  fn unary_arithmetic(
    &self,
    operator: Operator,
    operand: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    assert!(matches!(operator, Operator::Plus | Operator::Minus));
    let operand = operand.lvalue_conversion(self).decay(self);

    if !operand.unqualified_type().is_arithmetic() {
      Err(
        NonArithmeticInUnaryOp(operator, operand.to_string())
          + Severity::Error
          + span,
      )
    } else {
      let converted_operand =
        operand.usual_arithmetic_conversion_unary(self)?;
      let expr_type = *converted_operand.qualified_type();
      Ok(se::Expression::new_rvalue(
        self,
        se::Unary::prefix(operator, converted_operand),
        expr_type,
        span,
      ))
    }
  }

  /// i didnt came up with a better name...
  ///
  /// 6.5.4.1p2: The expression `++E` is equivalent to `(E+=1)`,
  /// where the value `1` IS OF THE APPRORIATE TYPE??!! WTF
  ///
  /// ...which means:
  /// ```c
  /// char c = 'C';
  /// __auto_type i = c++; //< deduced as of type `char`
  /// __auto_type j = ++c; //  ditto
  /// __auto_type k = c+1; //< deduced as type `int`
  /// _Static_assert(j == k, "success");
  /// ```
  fn ppmm(
    &self,
    operator: Operator,
    operand: se::ExprRef<'c>,
    kind: se::UnaryKind,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    if !operand.is_modifiable_lvalue(self) {
      Err(ExprNotAssignable(operand.to_string()) + Severity::Error + span)
    } else if !operand.qualified_type().is_scalar() {
      Err(
        NonArithmeticInUnaryOp(operator, operand.to_string())
          + Severity::Error
          + span,
      )
    } else {
      let operand_type = *operand.qualified_type();
      Ok(se::Expression::new_rvalue(
        self,
        se::Unary::new(operator, operand, kind),
        operand_type,
        span,
      ))
    }
  }

  /// bitwise NOT operator `~`
  ///
  /// 6.5.4.3.4: The result of the ~ operator is the bitwise complement of its (promoted) operand.
  ///     The integer promotions are performed on the operand, and the result has the promoted type.
  fn tilde(
    &self,
    operator: Operator,
    operand: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    assert_eq!(operator, Operator::Tilde);
    let operand = operand.lvalue_conversion(self).decay(self);

    if !operand.unqualified_type().is_integer() {
      Err(
        NonIntegerInBitwiseUnaryOp(operator, operand.to_string())
          + Severity::Error
          + span,
      )
    } else {
      let converted_operand =
        operand.usual_arithmetic_conversion_unary(self)?;
      let expr_type = *converted_operand.qualified_type();
      Ok(se::Expression::new_rvalue(
        self,
        se::Unary::prefix(operator, converted_operand),
        expr_type,
        span,
      ))
    }
  }

  /// logical NOT operator `!`
  ///
  /// 6.5.4.3.5: The result of the logical negation operator `!` \[...],
  /// the result has type int. The expression `!E` is equivalent to `(0==E)`.
  fn logical_not(
    &self,
    operator: Operator,
    operand: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    assert_eq!(operator, Operator::Not);
    let converted_operand = operand
      .lvalue_conversion(self)
      .decay(self)
      .is_contextually_convertible_to_bool()?;
    Ok(se::Expression::new_rvalue(
      self,
      se::Unary::prefix(operator, converted_operand),
      self.converted_bool().into(),
      span,
    ))
  }

  /// address-of operator `&`
  ///
  /// no `lvalue_conversion`, no `decay`
  /// 6.5.4.2.3: The unary & operator yields the address of its operand.
  /// If the operand has type "type"(in my Type system it's represented as `QualifiedType`)
  fn addressof(
    &self,
    operator: Operator,
    operand: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    assert_eq!(operator, Operator::Ampersand);
    if !operand.is_lvalue() {
      Err(
        AddressofOperandNotLvalue(operand.to_string()) + Severity::Error + span,
      )
    } else if matches!(&**operand, se::RawExpr::Variable(variable) if variable.as_variable().is_some_and(|v|v.storage == Storage::Register))
    {
      Err(AddressofOperandRegVar(operand.to_string()) + Severity::Error + span)
    } else {
      let pointee = *operand.qualified_type();
      Ok(se::Expression::new_rvalue(
        self,
        se::Unary::prefix(operator, operand),
        Type::Pointer(Pointer::new(pointee)).lookup(self).into(),
        span,
      ))
    }
  }

  /// indirection operator `*`
  ///
  /// 6.5.4.2.4: The unary * operator denotes indirection.
  /// the pointee needs to `lvalue_conversion` and `decay`, but the result itself does not need to
  fn indirect(
    &self,
    operator: Operator,
    operand: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    assert_eq!(operator, Operator::Star);

    let operand = operand.lvalue_conversion(self).decay(self);

    if !operand.unqualified_type().is_pointer() {
      Err(DerefNonPtr(operand.to_string()) + Severity::Error + span)?
    }

    let pointee_type =
      &operand.unqualified_type().as_pointer_unchecked().pointee;
    if RefEq::ref_eq(&**pointee_type, self.void_type()) {
      Err(DerefVoidPtr(operand.to_string()) + Severity::Error + span)
    } else {
      // If the operand points to a function, the result is a function designator; -- which means the we don't need to perform decay here
      // if it points to an object, the result is an lvalue designating the object.
      // If the operand has type "pointer to type", the result has type "type".
      // If an invalid value has been assigned to the pointer, the behavior is undefined.
      let expr_type = *pointee_type;
      Ok(se::Expression::new_lvalue(
        self,
        se::Unary::prefix(operator, operand),
        expr_type,
        span,
      ))
    }
  }
}
impl<'c> Sema<'c> {
  /// assignment operator `=`
  fn assignment(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let expr_type = *left.qualified_type();

    match operator.associated_operator() {
      _ if !left.is_modifiable_lvalue(self) => {
        self.add_error(ExprNotAssignable(left.to_string()), span);
        Ok(left)
      },
      // plain operator `=`.
      None => {
        let assigned_expr = right
          .lvalue_conversion(self)
          .decay(self)
          .assignment_conversion(self, &expr_type)?;
        Ok(se::Expression::new_rvalue(
          self,
          se::Binary::new(operator, left, assigned_expr),
          expr_type,
          span,
        ))
      },
      Some(binary_op) => {
        let intermediate = self.do_binary(binary_op, left, right, span)?;
        let intermediate_result_type = *intermediate.qualified_type();
        let se::Binary {
          left: intermediate_left,
          right,
          ..
        } = intermediate.as_binary_unchecked();

        let intermediate_left_type = *intermediate_left.qualified_type();

        // we also need to try to convert the intermediate_result_type back to ast_type,
        // like ptr_a -= ptr_b, which would be invalid.
        //
        // currently the CompoundAssign struct is the largest,
        // and we cant afford to append this cast type into the struct again. pay it via recalc at the IR Emitter.
        // assignment conversion is merely for a check.
        {
          se::Expression::try_get_cast_type(
            &intermediate_result_type,
            &expr_type,
          )
          .map_err(|meta| meta + span)?;

          debug_assert!(
            !intermediate_left.is_lvalue(),
            "idk if it's possible for a binary expr to procude lvalue in C?"
          );
          debug_assert!(
            !matches!(
              intermediate_left.unqualified_type(),
              Type::FunctionProto(_) | Type::Array(_)
            ),
            "is it possible for binary op to prodduce such type?",
          );
        }

        Ok(se::Expression::new_rvalue(
          self,
          se::CompoundAssign::new(
            operator,
            left,
            right,
            intermediate_left_type,
            intermediate_result_type,
          ),
          expr_type,
          span,
        ))
      },
    }
  }

  /// logical operators: `&&`, `||`
  ///
  /// 1. lvalue conversion
  /// 2. decay
  /// 3. check if contextually convertible to bool(int)
  fn logical(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let lhs = left
      .lvalue_conversion(self)
      .decay(self)
      .is_contextually_convertible_to_bool()?;

    let rhs = right
      .lvalue_conversion(self)
      .decay(self)
      .is_contextually_convertible_to_bool()?;

    Ok(se::Expression::new_rvalue(
      self,
      se::Binary::new(operator, lhs, rhs),
      self.converted_bool().into(),
      span,
    ))
  }

  /// relational operators: `<`, `>`, `<=`, `>=`, `==`, `!=`
  ///
  /// same as `logical`, but with arithmetic conversions if both operands are arithmetic types
  fn relational(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let left = left.lvalue_conversion(self).decay(self);
    let right = right.lvalue_conversion(self).decay(self);
    match (left.unqualified_type(), right.unqualified_type()) {
      (l, r) if l.is_arithmetic() && r.is_arithmetic() => {
        let (lhs, rhs, _common_type) =
          se::Expression::usual_arithmetic_conversion(left, right, self)?;

        Ok(se::Expression::new_rvalue(
          self,
          se::Binary::new(operator, lhs, rhs),
          self.converted_bool().into(),
          span,
        ))
      },
      (
        Type::Primitive(Primitive::Nullptr),
        Type::Primitive(Primitive::Nullptr),
      ) if matches!(operator, Operator::EqualEqual | Operator::NotEqual) =>
        Ok(se::Expression::new_rvalue(
          self,
          se::Binary::from_operator_unchecked(operator, left, right),
          self.converted_bool().into(),
          span,
        )),

      (l, r) if l.is_pointer() || r.is_pointer() =>
        self.pointer_relational(operator, left, right, span),

      (l, r) => Err(
        InvalidComparison(l.to_string(), r.to_string(), operator)
          + Severity::Error
          + span,
      ),
    }
  }

  fn pointer_relational(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    debug_assert!(
      left.unqualified_type().is_pointer()
        || right.unqualified_type().is_pointer()
    );
    use Operator::*;

    let ptr_with_nullptr = |ptr: se::ExprRef<'c>, nullptr: se::ExprRef<'c>| {
      let ptr_type = *ptr.qualified_type();
      let casted_nullptr = se::Expression::new_rvalue(
        self,
        se::ImplicitCast::new(nullptr, NullptrToPointer),
        ptr_type,
        span,
      );
      Ok(se::Expression::new_rvalue(
        self,
        se::Binary::from_operator_unchecked(operator, ptr, casted_nullptr),
        self.converted_bool().into(),
        span,
      ))
    };

    // if one of the operand is not a pointer and is not zero, emit a warning.
    match (left.unqualified_type(), right.unqualified_type()) {
      (Type::Pointer(left_ptr), Type::Pointer(right_ptr)) => {
        let left_pointee = &left_ptr.pointee;
        let right_pointee = &right_ptr.pointee;
        if Compatibility::compatible(left_pointee, right_pointee) {
          Ok(se::Expression::new_rvalue(
            self,
            se::Binary::new(operator, left, right),
            self.converted_bool().into(),
            span,
          ))
        } else {
          Err(
            CompareDistinctPointerTypes(
              left.qualified_type().to_string(),
              right.qualified_type().to_string(),
            )
            + Severity::Error // should be warning
            +span,
          )
        }
      },

      (Type::Pointer(_), Type::Primitive(Primitive::Nullptr))
        if matches!(operator, EqualEqual | NotEqual) =>
        ptr_with_nullptr(left, right),
      (Type::Primitive(Primitive::Nullptr), Type::Pointer(_))
        if matches!(operator, EqualEqual | NotEqual) =>
        ptr_with_nullptr(right, left),
      (l, r) => Err(
        InvalidComparison(l.to_string(), r.to_string(), operator)
          + Severity::Error
          + span,
      ),
    }
  }

  fn arithmetic(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let left = left.lvalue_conversion(self).decay(self);
    let right = right.lvalue_conversion(self).decay(self);

    match (left.unqualified_type(), right.unqualified_type()) {
      (l, r) if l.is_arithmetic() && r.is_arithmetic() =>
        self.usual_arithmetic(operator, left, right, span),
      (l, r) if l.is_pointer() || r.is_pointer() =>
        self.pointer_arithematic(operator, left, right, span),
      // todo: enum constant..
      _ => Err(
        NonArithmeticInBinaryOp(left.to_string(), right.to_string(), operator)
          + Severity::Error
          + span,
      ),
    }
  }

  /// usual arithmetic conversion: `+`, `-`, `*`, `/`, `%`
  ///
  /// 1. lvalue conversion, with the exception of arrays and functionproto\(handled inside the `lvalue_conversion`\)
  /// 2. array and function decay
  /// 3. promotions\(inside `usual_arithmetic_conversion`\)
  /// 4. finally, the usual arithmetic conversion itself
  fn usual_arithmetic(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    debug_assert!(
      left.unqualified_type().is_arithmetic()
        && right.unqualified_type().is_arithmetic()
    );

    let (lhs, rhs, result_type) =
      se::Expression::usual_arithmetic_conversion(left, right, self)?;

    Ok(se::Expression::new_rvalue(
      self,
      se::Binary::new(operator, lhs, rhs),
      result_type,
      span,
    ))
  }

  /// at least one of the operand is pointer, and the operand can only be `+` or `-`.
  ///
  /// This is specified more detailed in C++ Standard \[over.built\].
  ///
  /// - left and right are both pointer of type `T`, the operator is `-` -- return type is `ptrdiff_t`.
  /// - left and right are both pointer of type `T`, the operator is `+` -- error.
  /// - left is a pointer to type `T`, right is an integer, the operator is `+` -- right converts to `ptrdiff_t`(my implementation), return type is `*T`
  /// - left is a pointer to type `T`, right is an integer, the operator is `-` -- right converts to `ptrdiff_t`(my implementation), return type is `*T`
  /// - left is an integer, right is a pointer to type `T`, the operator is `+` -- same as above.
  /// - left is an integer, right is a pointer to type `T`, the operator is `-` -- error.
  /// - left and right are pointers to incompatible type -- error.
  ///
  ///
  /// | Left         |    Op    | Right         | Result Type                 |
  /// | ------------ | -------- | ------------- | --------------------------- |
  /// | `*T`         | `-`      | `*T`          | `ptrdiff_t`                 |
  /// | `*T`         | `+`      | `*T`          | *Invalid*                   |
  /// | `*T`         | `+`      | Integer       | `*T`                        |
  /// | `*T`         | `-`      | Integer       | `*T`                        |
  /// | Integer      | `+`      | `*T`          | `*T`                        |
  /// | Integer      | `-`      | `*T`          | *Invalid*                   |
  /// | `*T1`        | `+/-`    | `*T2`         | *Incompatible*              |
  fn pointer_arithematic(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    debug_assert!(
      left.unqualified_type().is_pointer()
        || right.unqualified_type().is_pointer()
    );
    match (left.unqualified_type(), right.unqualified_type()) {
      // ptr - ptr
      (Type::Pointer(left_ptr), Type::Pointer(right_ptr))
        if operator == Operator::Minus =>
        match Compatibility::compatible(&left_ptr.pointee, &right_ptr.pointee) {
          // -> ptrdiff
          true => Ok(se::Expression::new_rvalue(
            self,
            se::Binary::new(operator, left, right),
            self.ptrdiff_type().into(), // no qual for pointer differences
            span,
          )),
          // -> error
          false => Err(
            IncompatiblePointerTypes(
              left.qualified_type().to_string(),
              right.qualified_type().to_string(),
            ) + Severity::Error
              + span,
          ),
        },
      // int + ptr => ptr

      // be aware that the left and right switched their position in order to
      // make irgen earsier.
      (Type::Primitive(lhs), Type::Pointer(_))
        if lhs.is_integer() && operator == Operator::Plus =>
      {
        let ptrty = right.unqualified_type().clone().lookup(self);
        Ok(se::Expression::new_rvalue(
          self,
          se::Binary::new(operator, right, left),
          ptrty.into(),
          span,
        ))
      },
      // ptr + int => ptr
      (Type::Pointer(_), Type::Primitive(rhs))
        if rhs.is_integer() && operator == Operator::Plus =>
      {
        let ptrty = left.unqualified_type().clone().lookup(self);
        Ok(se::Expression::new_rvalue(
          self,
          se::Binary::new(operator, left, right),
          ptrty.into(),
          span,
        ))
      },
      // ptr - int => ptr
      (Type::Pointer(_), Type::Primitive(rhs))
        if rhs.is_integer() && operator == Operator::Minus =>
      {
        let ptrty = left.unqualified_type().clone().lookup(self);
        let right = right.ptrdiff_conversion_unchecked(self);

        Ok(se::Expression::new_rvalue(
          self,
          se::Binary::new(operator, left, right),
          ptrty.into(),
          span,
        ))
      },
      _ => Err(
        InvalidOprand(
          left.qualified_type().to_string(),
          right.qualified_type().to_string(),
          operator,
        ) + Severity::Error
          + span,
      ),
    }
  }

  /// bitwise operators: `&`, `|`, `^`
  ///
  /// mostly same as arithmetic, but only for integer types
  fn bitwise(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let lhs = left.lvalue_conversion(self).decay(self);
    let rhs = right.lvalue_conversion(self).decay(self);

    if !lhs.unqualified_type().is_integer()
      || !rhs.unqualified_type().is_integer()
    {
      self.add_error(
        NonIntegerInBitwiseBinaryOp(lhs.to_string(), rhs.to_string(), operator),
        span,
      );
    }

    let (left, right, result_type) =
      se::Expression::usual_arithmetic_conversion(lhs, rhs, self)?;

    Ok(se::Expression::new_rvalue(
      self,
      se::Binary::new(operator, left, right),
      result_type,
      span,
    ))
  }

  /// bitshift operators: `<<`, `>>`
  ///
  /// lvalue conversion, decay, promote, both operands must be integer types, but no usual arithmetic conversion
  fn bitshift(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let left = left.lvalue_conversion(self).decay(self).promote(self);
    let right = right.lvalue_conversion(self).decay(self).promote(self);

    if !left.unqualified_type().is_integer()
      || !right.unqualified_type().is_integer()
    {
      Err(
        NonIntegerInBitshiftOp(left.to_string(), right.to_string(), operator)
          + Severity::Error
          + span,
      )?
    }

    // TODO: if the right is constant and it's not  a positive value, issue a warning.

    let expr_type = *left.qualified_type();
    Ok(se::Expression::new_rvalue(
      self,
      se::Binary::new(operator, left, right),
      expr_type,
      span,
    ))
  }

  /// comma operator `,`
  ///
  /// left is void converted, result is right expression
  fn comma(
    &self,
    operator: Operator,
    left: se::ExprRef<'c>,
    right: se::ExprRef<'c>,
    span: SourceSpan,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    // the result is the right expression, and the left is void converted, that's it. done.
    let expr_type = *right.qualified_type();
    Ok(se::Expression::new_rvalue(
      self,
      se::Binary::new(operator, left, right),
      expr_type,
      span,
    ))
  }
}
impl<'c> Sema<'c> {
  fn statements(
    &mut self,
    statements: Vec<ps::Statement<'c>>,
  ) -> Vec<ss::StmtRef<'c>> {
    statements
      .into_iter()
      .filter_map(|statement| match self.statement(statement) {
        Ok(stmt) => Some(stmt),
        Err(e) => {
          self.add_diag(e);
          None
        },
      })
      .collect()
  }

  fn statement(
    &mut self,
    statement: ps::Statement<'c>,
  ) -> Result<ss::StmtRef<'c>, Diag<'c>> {
    match statement {
      ps::Statement::Empty(_) => Ok(Default::default()),
      ps::Statement::Expression(expression) =>
        self.exprstmt(expression).map(Into::into),
      ps::Statement::Compound(compound_stmt) =>
        self.compound(compound_stmt).map(Into::into),
      ps::Statement::Return(return_stmt) =>
        self.returnstmt(return_stmt).map(Into::into),
      ps::Statement::Declaration(declaration) =>
        self.declstmt(declaration).map(Into::into),
      ps::Statement::If(if_stmt) => self.ifstmt(if_stmt).map(Into::into),
      ps::Statement::While(while_stmt) =>
        self.whilestmt(while_stmt).map(Into::into),
      ps::Statement::DoWhile(do_while) =>
        self.dowhilestmt(do_while).map(Into::into),
      ps::Statement::For(for_stmt) => self.forstmt(for_stmt).map(Into::into),
      ps::Statement::Label(label) => self.labelstmt(label).map(Into::into),
      ps::Statement::Switch(switch) => self.switchstmt(switch).map(Into::into),
      ps::Statement::Goto(goto) => self.gotostmt(goto).map(Into::into),
      ps::Statement::Break(break_stmt) =>
        self.breakstmt(break_stmt).map(Into::into),
      ps::Statement::Continue(continue_stmt) =>
        self.continuestmt(continue_stmt).map(Into::into),
    }
    .map(|statement| ss::Statement::alloc(self, statement))
  }

  fn statement_or_default(
    &mut self,
    statement: ps::Statement<'c>,
  ) -> ss::StmtRef<'c> {
    match self.statement(statement) {
      Ok(statement) => statement,
      Err(error) => {
        self.add_diag(error);
        self.__empty_stmt
      },
    }
  }

  #[inline]
  fn compound(
    &mut self,
    compound: ps::Compound<'c>,
  ) -> Result<ss::Compound<'c>, Diag<'c>> {
    self.compound_with(compound, |_| {})
  }

  fn compound_with<Fn>(
    &mut self,
    compound: ps::Compound<'c>,
    callback: Fn,
  ) -> Result<ss::Compound<'c>, Diag<'c>>
  where
    Fn: FnOnce(&Self),
  {
    self.environment.enter();

    callback(self);

    let statements = self.statements(compound.statements);

    self.environment.exit();

    Ok(ss::Compound::new(self, statements, compound.span))
  }

  fn exprstmt(
    &self,
    expr_stmt: pe::Expression<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    // todo: unused expression result warning
    self.expression(expr_stmt)
  }

  fn declstmt(
    &mut self,
    declaration: pd::Declaration<'c>,
  ) -> Result<ss::DeclStmt<'c>, Diag<'c>> {
    let pd::Declaration {
      declspecs,
      init_declarators,
      span,
    } = declaration;
    Ok(ss::DeclStmt::new(
      self.ast(),
      self.declarations(declspecs, init_declarators),
      span,
    ))
  }

  fn returnstmt(
    &self,
    return_stmt: ps::Return<'c>,
  ) -> Result<ss::Return<'c>, Diag<'c>> {
    let ps::Return { expression, span } = return_stmt;
    let analyzed_expr = match expression {
      Some(expr) => Some(self.expression(expr)?),
      None => None,
    };

    let return_type = self
      .current_function
      .as_ref()
      .shall_ok("return statement outside function should be handled in parser")
      .qualified_type
      .as_functionproto_unchecked()
      .return_type;

    match (analyzed_expr, &*return_type) {
      (None, Type::Primitive(Primitive::Void)) =>
        Ok(ss::Return::new(None, span)),
      (None, _) => Err(
        ReturnTypeMismatch("non-void function must return a value".to_string())
          + Severity::Error
          + span,
      ),

      (Some(_), Type::Primitive(Primitive::Void)) => Err(
        ReturnTypeMismatch("void function cannot return a value".to_string())
          + Severity::Error
          + span,
      ),

      (Some(analyzed_expr), _) => {
        let a = analyzed_expr
          .lvalue_conversion(self)
          .decay(self)
          .assignment_conversion(self, &return_type)?;
        Ok(ss::Return::new(Some(a), span))
      },
    }
  }

  fn condition(&mut self, condition: pe::Expression<'c>) -> ExprRef<'c> {
    self
      .expression(condition)
      .and_then(|e| {
        e.lvalue_conversion(self)
          .decay(self)
          .is_contextually_convertible_to_bool()
      })
      .handle_with(
        self,
        se::Expression::new_error_node(self, self.converted_bool().into()),
      )
  }

  fn ifstmt(&mut self, if_stmt: ps::If<'c>) -> Result<ss::If<'c>, Diag<'c>> {
    let ps::If {
      condition,
      then_branch,
      else_branch,
      span,
    } = if_stmt;
    let analyzed_condition = self.condition(condition);
    let analyzed_then_branch = self.statement_or_default(*then_branch);
    let analyzed_else_branch =
      else_branch.map(|else_branch| self.statement_or_default(*else_branch));
    Ok(ss::If::new(
      self,
      analyzed_condition,
      analyzed_then_branch,
      analyzed_else_branch,
      span,
    ))
  }

  fn whilestmt(
    &mut self,
    while_stmt: ps::While<'c>,
  ) -> Result<ss::While<'c>, Diag<'c>> {
    let ps::While {
      condition,
      body,
      span,
    } = while_stmt;
    let analyzed_condition = self.condition(condition);
    self.scope_context.push(ScopeContext::Loop);
    let analyzed_body = self.statement_or_default(*body);
    let _while = self.scope_context.pop();
    debug_assert!(
      matches!(_while, Some(ScopeContext::Loop)),
      "scope context stack corrupted: expected loop context"
    );

    Ok(ss::While::new(
      self,
      analyzed_condition,
      analyzed_body,
      span,
    ))
  }

  fn dowhilestmt(
    &mut self,
    do_while: ps::DoWhile<'c>,
  ) -> Result<ss::DoWhile<'c>, Diag<'c>> {
    let ps::DoWhile {
      body,
      condition,
      span,
    } = do_while;
    self.scope_context.push(ScopeContext::Loop);
    let analyzed_body = self.statement_or_default(*body);
    let _do_while = self.scope_context.pop();
    debug_assert!(
      matches!(_do_while, Some(ScopeContext::Loop)),
      "scope context stack corrupted: expected loop context"
    );

    let analyzed_condition = self.condition(condition);
    Ok(ss::DoWhile::new(
      self,
      analyzed_body,
      analyzed_condition,
      span,
    ))
  }

  fn forstmt(
    &mut self,
    for_stmt: ps::For<'c>,
  ) -> Result<ss::For<'c>, Diag<'c>> {
    let ps::For {
      initializer,
      condition,
      increment,
      body,
      span,
    } = for_stmt;
    let analyzed_initializer =
      initializer.map(|init| self.statement_or_default(*init));
    let analyzed_condition = condition.map(|cond| {
      self.expression(cond).handle_with(
        self,
        se::Expression::new_error_node(self, self.converted_bool().into()),
      )
    });
    let analyzed_increment = increment.map(|inc| {
      self.expression(inc).handle_with(
        self,
        se::Expression::new_error_node(self, self.int_type().into()),
      )
    });

    self.scope_context.push(ScopeContext::Loop);
    let analyzed_body = self.statement_or_default(*body);
    let _for = self.scope_context.pop();
    debug_assert!(
      matches!(_for, Some(ScopeContext::Loop)),
      "scope context stack corrupted: expected loop context"
    );

    Ok(ss::For::new(
      self,
      analyzed_initializer,
      analyzed_condition,
      analyzed_increment,
      analyzed_body,
      span,
    ))
  }

  fn switchstmt(
    &mut self,
    switch: ps::Switch<'c>,
  ) -> Result<ss::Switch<'c>, Diag<'c>> {
    let ps::Switch {
      cases,
      condition,
      default,
      span,
    } = switch;
    let analyzed_condition = match self.expression(condition) {
      Ok(val) if val.unqualified_type().is_integer() => val,
      Ok(val) => {
        self.add_error(
          ExprNotConstant(format!(
            "switch condition must have integer type, found '{}'",
            val.qualified_type()
          )),
          span,
        );
        val
      },
      Err(e) => {
        self.add_diag(e);
        se::Expression::new_error_node(self, self.int_type().into())
      },
    };
    let analyzed_cases = cases
      .into_iter()
      .map(|case| self.casestmt(case).shall_ok("switch case"))
      .collect::<Vec<_>>();

    self.scope_context.push(ScopeContext::Switch);
    let analyzed_default = default
      .map(|default| self.defaultstmt(default).shall_ok("switch default"));
    let _switch = self.scope_context.pop();
    debug_assert!(
      matches!(_switch, Some(ScopeContext::Switch)),
      "scope context stack corrupted: expected loop context"
    );

    Ok(ss::Switch::new(
      self,
      analyzed_condition,
      analyzed_cases,
      analyzed_default,
      span,
    ))
  }

  fn casestmt(&mut self, case: ps::Case<'c>) -> Result<ss::Case<'c>, Diag<'c>> {
    let ps::Case { body, value, span } = case;
    let analyzed_value = self.expression(value).handle_with(
      self,
      se::Expression::new_error_node(self, self.int_type().into()),
    );
    let analyzed_body = self.statements(body);

    Ok(ss::Case::new(
      self,
      self
        .folder(false, analyzed_value)
        .map(|expr| {
          if let se::RawExpr::Constant(constant) = &**expr {
            constant.clone()
          } else {
            self.add_error(NonIntegerInCaseStmt(expr.to_string()), expr.span());
            Integral::default().into()
          }
        })
        .unwrap(),
      analyzed_body,
      span,
    ))
  }

  fn defaultstmt(
    &mut self,
    default: ps::Default<'c>,
  ) -> Result<ss::Default<'c>, Diag<'c>> {
    let ps::Default { body, span } = default;
    let analyzed_body = self.statements(body);
    Ok(ss::Default::new(self, analyzed_body, span))
  }

  fn labelstmt(
    &mut self,
    label: ps::Label<'c>,
  ) -> Result<ss::Label<'c>, Diag<'c>> {
    match self.environment.is_global() {
      true => contract_violation!(
        "label statement in global scope should be handled in parser"
      ),
      false => {
        let ps::Label {
          name,
          statement,
          span,
        } = label;
        match self.current_labels.insert((*name).into()) {
          true => Ok(ss::Label::new(
            self.ast(),
            name,
            self.statement_or_default(*statement),
            span,
          )),
          false => Err(DuplicateLabel(name) + Severity::Error + span),
        }
      },
    }
  }

  fn gotostmt(&mut self, goto: ps::Goto<'c>) -> Result<ss::Goto<'c>, Diag<'c>> {
    match self.environment.is_global() {
      true => contract_violation!(
        "goto statement in global scope should be handled in parser"
      ),
      false => {
        self.current_gotos.insert((*goto.label).into());
        Ok(ss::Goto::new(goto.label, goto.span))
      },
    }
  }

  fn breakstmt(&self, break_stmt: ps::Break) -> Result<ss::Break, Diag<'c>> {
    match self.environment.is_global() {
      true => Err(TopLevelBreak + Severity::Error + break_stmt.span),
      false => match self
        .scope_context
        .iter()
        .rev()
        .find(|ctx| matches!(ctx, ScopeContext::Loop | ScopeContext::Switch))
      {
        Some(_) => Ok(ss::Break::new(break_stmt.span)),
        None => Err(BreakNotWithinLoop + Severity::Error + break_stmt.span),
      },
    }
  }

  fn continuestmt(
    &self,
    continue_stmt: ps::Continue,
  ) -> Result<ss::Continue, Diag<'c>> {
    match self.environment.is_global() {
      true => Err(TopLevelContinue + Severity::Error + continue_stmt.span),
      false => match self
        .scope_context
        .iter()
        .rev()
        .find(|ctx| matches!(ctx, ScopeContext::Loop))
      {
        Some(_) => Ok(ss::Continue::new(continue_stmt.span)),
        None =>
          Err(ContinueNotWithinLoop + Severity::Error + continue_stmt.span),
      },
    }
  }
}

#[cold]
#[inline(never)]
fn shall_ok_failed(msg: &str, location: &std::panic::Location) -> ! {
  panic!(
    "Invariant at {}: {}.
    current implementation should always return `Ok` here.
    This is a program internal error, please fix it!",
    location, msg
  );
}

pub(crate) trait ShallOk<T> {
  /// Glorified `expect` for `Result`, use this to indicate a `program error/invariant`
  ///
  /// - `.expect("some message")` -> (prob) for user side error(although rarely use this way)
  /// - `.shall_ok("some message")` -> for program internal invariant which indicates the problem is in the implementation
  fn shall_ok<M: Into<Option<&'static str>>>(self, msg: M) -> T;
}

impl<'c, T> ShallOk<T> for Result<T, Diag<'c>> {
  #[track_caller]
  fn shall_ok<M: Into<Option<&'static str>>>(self, msg: M) -> T {
    match self {
      Ok(t) => t,
      Err(_) => shall_ok_failed(
        msg.into().unwrap_or("No additional info"),
        ::std::panic::Location::caller(),
      ),
    }
  }
}
impl<T> ShallOk<T> for Option<T> {
  #[track_caller]
  fn shall_ok<M: Into<Option<&'static str>>>(self, msg: M) -> T {
    match self {
      Some(t) => t,
      None => shall_ok_failed(
        msg.into().unwrap_or("No additional info"),
        ::std::panic::Location::caller(),
      ),
    }
  }
}

pub(crate) trait HandleWith<T, Listener> {
  fn handle_with(self, listener: &Listener, default: T) -> T;
}

impl<'c, T> HandleWith<T, Sema<'c>> for Result<T, Diag<'c>> {
  /// if it's error, log it, and return a default value (means error)
  fn handle_with(self, listener: &Sema<'c>, default: T) -> T {
    match self {
      Ok(t) => t,
      Err(e) => {
        listener.add_diag(e);
        default
      },
    }
  }
}
