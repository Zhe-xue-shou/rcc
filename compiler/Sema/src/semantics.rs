use ::rcc_adt::Integral;
use ::rcc_ast::{
  Context, Session, SessionRef, VarDeclKind,
  types::{
    Array, ArraySize, Compatibility, FunctionProto, FunctionSpecifier, Pointer,
    Primitive, QualifiedType, Type, TypeInfo, UnqualExt,
  },
};
use ::rcc_parse::{declaration as pd, expression as pe, statement as ps};
use ::rcc_shared::{
  ArenaVec, CollectIn, Diag,
  DiagData::{self, *},
  Diagnosis, OpDiag, Operator, OperatorCategory, Severity, SourceSpan, Storage,
};
use ::rcc_utils::{
  RefEq, StrRef, contract_assert, contract_violation, not_implemented_feature,
};
use ::std::collections::{HashMap, HashSet};

use super::{declaration as sd, declref, expression as se, statement as ss};
use crate::initialization::Initialization;

pub(crate) enum ScopeContext {
  Function,
  Loop,
  Switch,
}

type DeclScopeAssoc<'c> = HashMap<StrRef<'c>, sd::DeclRef<'c>>;

#[derive(Debug, Default)]
struct DeclEnvironment<'c> {
  scopes: Vec<DeclScopeAssoc<'c>>,
}

impl<'c> DeclEnvironment<'c> {
  fn enter(&mut self) {
    self.scopes.push(Default::default());
  }

  fn exit(&mut self) {
    self.scopes.pop();
  }

  fn is_global(&self) -> bool {
    self.scopes.len() == 1
  }

  fn find(&self, name: StrRef<'c>) -> Option<sd::DeclRef<'c>> {
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

  fn declare(
    &mut self,
    name: StrRef<'c>,
    declaration: sd::DeclRef<'c>,
  ) -> Result<(), DiagData<'c>> {
    if !declaration.qualified_type().is_complete() {
      Err(VariableIncompleteType(
        name,
        declaration.qualified_type().to_string(),
      ))?
    }
    self
      .scopes
      .last_mut()
      .shall_ok("No scope to declare symbol")
      .insert(name, declaration);
    Ok(())
  }
}

pub struct Sema<'c> {
  program: pd::Program<'c>,
  environment: DeclEnvironment<'c>,
  current_function: Option<sd::Function<'c>>,
  current_labels: HashSet<StrRef<'c>>,
  current_gotos: HashSet<StrRef<'c>>,
  scope_context: Vec<ScopeContext>,
  pub(crate) session: SessionRef<'c, OpDiag<'c>>,

  pub(crate) __empty_expr: se::ExprRef<'c>,
  pub(crate) __empty_stmt: ss::StmtRef<'c>,
}
impl<'a> ::std::ops::Deref for Sema<'a> {
  type Target = Session<'a, OpDiag<'a>>;

  fn deref(&self) -> &Self::Target {
    self.session
  }
}
impl<'c> Sema<'c> {
  pub fn new(
    program: pd::Program<'c>,
    session: SessionRef<'c, OpDiag<'c>>,
  ) -> Self {
    Self {
      program,
      session,
      environment: Default::default(),
      current_function: Default::default(),
      current_labels: Default::default(),
      current_gotos: Default::default(),
      scope_context: Default::default(),
      __empty_expr: se::Expression::new_error_node(
        session.ast(),
        session.ast().int_type().into(),
      ),
      __empty_stmt: ss::Statement::alloc(session.ast(), Default::default()),
    }
  }

  pub fn context(&self) -> &'c Context<'c> {
    self.ast()
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

  pub fn analyze(&mut self) -> sd::TranslationUnit<'c> {
    self.environment.enter();
    let translation_unit =
      sd::TranslationUnit::new(self.context(), self.externaldecl());

    self.environment.exit();
    translation_unit
  }
}
impl<'c> Sema<'c> {
  /// IMPORTANT: currently, caller shoould check:
  /// 1. whether the `restrict` is valid; (it's only valid for pointers and non-static local variable.)
  /// 2. the type is complete or not, via [`TypeInfo::size`].
  fn apply_modifiers_for_varty(
    &self,
    mut qualified_type: QualifiedType<'c>,
    modifiers: Vec<pd::Modifier<'c>>,
  ) -> QualifiedType<'c> {
    // reverse order
    for modifier in modifiers.into_iter().rev() {
      match modifier {
        pd::Modifier::Pointer(qualifiers) => {
          qualified_type = QualifiedType::new(
            qualifiers,
            Type::Pointer(Pointer::new(qualified_type)).lookup(self.context()),
          );
        },
        pd::Modifier::Array(array_modifier) => {
          let size = match array_modifier.bound {
            None => ArraySize::Incomplete,
            Some(expr) => {
              // check 1. it's a constant expression or not, 2. it's type should be integer type 3. should be non-negative
              let analyzed_expr = self.expression(expr).handle_with(
                self,
                se::Expression::new_error_node(
                  self.context(),
                  self.context().int_type().into(),
                ),
              );

              if analyzed_expr.qualified_type().is_scalar() {
                use super::folding::FoldingResult::*;
                match analyzed_expr.fold(self.session) {
                  Success(v) =>
                    if v.is_integer_constant() {
                      ArraySize::Constant(
                        match v.raw_expr().as_constant_unchecked() {
                          se::Constant::Integral(integral) =>
                            integral.to_builtin(),
                          _ => unreachable!(),
                        },
                      )
                    } else {
                      self.add_error(
                        NonIntegerInArraySubscript(v.to_string()),
                        v.span(),
                      );
                      ArraySize::Constant(0)
                    },
                  Failure(_) => {
                    todo!("VLA not supported")
                  },
                }
              } else {
                self.add_error(
                  NonIntegerInArraySubscript(analyzed_expr.to_string()),
                  analyzed_expr.span(),
                );
                ArraySize::Constant(0) // error case
              }
            },
          };
          if !qualified_type.is_complete() {
            self.add_error(
              ArrayHasIncompleteType(qualified_type.to_string()),
              array_modifier.span,
            );
          }
          qualified_type = Type::Array(Array::new(qualified_type, size))
            .lookup(self.context())
            .into();
        },
        pd::Modifier::Function(function_signature) => {
          // func ptr or so
          let pd::FunctionSignature {
            parameters,
            is_variadic,
          } = function_signature;
          let analyzed_parameter_types = self.parse_parameter_types(parameters);
          let p = self.context().intern(FunctionProto::<'c>::new(
            qualified_type,
            analyzed_parameter_types.into_bump_slice(),
            is_variadic,
          ));
          qualified_type = p.into();
        },
      }
    }
    qualified_type
  }

  fn apply_modifiers_for_functiondecl(
    &self,
    return_type: QualifiedType<'c>,
    modifiers: Vec<pd::Modifier<'c>>,
  ) -> Result<
    (
      QualifiedType<'c>,
      Vec<sd::Parameter<'c>>, /* parameters name and their type, here's some repetition
                              parameter type had also been inside QualifiedType of the function */
    ),
    Diag<'c>,
  > {
    contract_assert!(
      modifiers.len() == 1,
      "function declarator should have only one modifier"
    );
    let function_signature = match modifiers.into_iter().next().unwrap() {
      pd::Modifier::Function(function_signature) => function_signature,
      _ => {
        contract_violation!("function declarator should have function modifier")
      },
    };
    // we need to build function type
    let parameters = self.parse_parameters(function_signature.parameters);
    let parameter_types = parameters
      .iter()
      .map(|param| {
        let qualified_type = param.declaration.qualified_type();
        if !qualified_type.is_complete() {
          self.add_error(
            VariableIncompleteType(
              param.declaration.name(),
              qualified_type.to_string(),
            ),
            param.span,
          );
        }
        qualified_type
      })
      .collect_in::<ArenaVec<_>>(self.context().arena());
    Ok((
      self
        .context()
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
        let (_, storage, base_type) = self
          .parse_declspecs(declspecs)
          .shall_ok("Failed to parse declspecs for parameter");
        contract_assert!(
          storage.is_none() || storage.is_some_and(|s| s.is_register())
        );
        // strictly speaking the names shall be unique but it doesnt matter here really.
        let pd::Declarator {
          modifiers,
          name: _,
          span: _,
        } = declarator;
        self.apply_modifiers_for_varty(base_type, modifiers)
      })
      .collect_in(&**(self.context().arena()))
  }

  fn parse_parameters(
    &self,
    parameters: Vec<pd::Parameter<'c>>,
  ) -> Vec<sd::Parameter<'c>> {
    parameters
      .into_iter()
      .map(|parameter| {
        let pd::Parameter {
          declarator,
          declspecs,
          span,
        } = parameter;
        let (_, storage, base_type) = self
          .parse_declspecs(declspecs)
          .shall_ok("Failed to parse declspecs for parameter");
        contract_assert!(
          storage.is_none() || storage.is_some_and(|s| s.is_register())
        );
        let pd::Declarator {
          modifiers,
          name,
          span: _,
        } = declarator;
        let qualified_type =
          self.apply_modifiers_for_varty(base_type, modifiers);
        let declaration = declref::DeclNode::decl(
          self.context(),
          qualified_type,
          Storage::Automatic,
          name.unwrap_or_else(|| self.ast().unnamed_str()),
          None,
        );
        sd::Parameter::new(declaration, span)
      })
      .collect()
  }

  fn parse_declspecs(
    &self,
    declspecs: pd::DeclSpecs<'c>,
  ) -> Result<(FunctionSpecifier, Option<Storage>, QualifiedType<'c>), Diag<'c>>
  {
    let qualified_type = self
      .get_type(declspecs.type_specifiers)
      .handle_with(self, self.context().int_type().into())
      .with_qualifiers(declspecs.qualifiers);
    let storage_class = declspecs.storage_class;
    let function_specifier = declspecs.function_specifiers;

    Ok((function_specifier, storage_class, qualified_type))
  }

  fn get_type(
    &self,
    mut type_specifiers: Vec<pd::TypeSpecifier<'c>>,
  ) -> Result<QualifiedType<'c>, Diag<'c>> {
    assert!(!type_specifiers.is_empty());
    assert!(type_specifiers.len() <= 5); // unsigned long long int complex (integer complex not in standard) is the max
    type_specifiers.sort_by_key(|s| s.sort_key());
    type TS<'a> = pd::TypeSpecifier<'a>;
    // 6.7.3.1
    match type_specifiers.as_slice() {
      [TS::Nullptr] => Ok(
        Type::Primitive(Primitive::Nullptr)
          .lookup(self.context())
          .into(),
      ),
      [TS::Void] => Ok(
        Type::Primitive(Primitive::Void)
          .lookup(self.context())
          .into(),
      ),

      [TS::Bool] => Ok(
        Type::Primitive(Primitive::Bool)
          .lookup(self.context())
          .into(),
      ),

      [TS::Char] => Ok(
        Type::Primitive(Primitive::Char)
          .lookup(self.context())
          .into(),
      ),
      [TS::Signed, TS::Char] => Ok(
        Type::Primitive(Primitive::SChar)
          .lookup(self.context())
          .into(),
      ),
      [TS::Unsigned, TS::Char] => Ok(
        Type::Primitive(Primitive::UChar)
          .lookup(self.context())
          .into(),
      ),

      [TS::Short]
      | [TS::Short, TS::Int]
      | [TS::Signed, TS::Short]
      | [TS::Signed, TS::Short, TS::Int] =>
        Ok(self.context().short_type().into()),
      [TS::Unsigned, TS::Short] | [TS::Unsigned, TS::Short, TS::Int] =>
        Ok(self.context().ushort_type().into()),

      [TS::Int] | [TS::Signed] | [TS::Signed, TS::Int] =>
        Ok(self.context().int_type().into()),
      [TS::Unsigned] | [TS::Unsigned, TS::Int] =>
        Ok(self.context().uint_type().into()),

      [TS::Long]
      | [TS::Long, TS::Int]
      | [TS::Signed, TS::Long]
      | [TS::Signed, TS::Long, TS::Int] =>
        Ok(self.context().long_type().into()),
      [TS::Unsigned, TS::Long] | [TS::Unsigned, TS::Long, TS::Int] =>
        Ok(self.context().ulong_type().into()),

      [TS::Long, TS::Long]
      | [TS::Long, TS::Long, TS::Int]
      | [TS::Signed, TS::Long, TS::Long]
      | [TS::Signed, TS::Long, TS::Long, TS::Int] =>
        Ok(self.context().long_long_type().into()),
      [TS::Unsigned, TS::Long, TS::Long]
      | [TS::Unsigned, TS::Long, TS::Long, TS::Int] =>
        Ok(self.context().ulong_long_type().into()),

      [TS::Float] => Ok(self.context().float32_type().into()),
      [TS::Double] => Ok(self.context().float64_type().into()),
      [TS::Long, TS::Double] => Ok(
        Type::Primitive(Primitive::LongDouble)
          .lookup(self.context())
          .into(),
      ),

      [TS::Float, TS::Complex] => Ok(
        Type::Primitive(Primitive::ComplexFloat)
          .lookup(self.context())
          .into(),
      ),
      [TS::Double, TS::Complex] => Ok(
        Type::Primitive(Primitive::ComplexDouble)
          .lookup(self.context())
          .into(),
      ),
      [TS::Long, TS::Double, TS::Complex] => Ok(
        Type::Primitive(Primitive::ComplexLongDouble)
          .lookup(self.context())
          .into(),
      ),

      // treat complex integers as error
      [TS::Char, TS::Complex]
      | [TS::Signed, TS::Char, TS::Complex]
      | [TS::Unsigned, TS::Char, TS::Complex]
      | [TS::Short, TS::Complex]
      | [TS::Short, TS::Int, TS::Complex]
      | [TS::Signed, TS::Short, TS::Complex]
      | [TS::Signed, TS::Short, TS::Int, TS::Complex]
      | [TS::Unsigned, TS::Short, TS::Complex]
      | [TS::Unsigned, TS::Short, TS::Int, TS::Complex]
      | [TS::Int, TS::Complex]
      | [TS::Signed, TS::Complex]
      | [TS::Signed, TS::Int, TS::Complex]
      | [TS::Unsigned, TS::Complex]
      | [TS::Unsigned, TS::Int, TS::Complex]
      | [TS::Long, TS::Complex]
      | [TS::Long, TS::Int, TS::Complex]
      | [TS::Signed, TS::Long, TS::Complex]
      | [TS::Signed, TS::Long, TS::Int, TS::Complex]
      | [TS::Unsigned, TS::Long, TS::Complex]
      | [TS::Unsigned, TS::Long, TS::Int, TS::Complex] => {
        not_implemented_feature!("Complex integer types are not supported");
      },

      [TS::Typedef(t)] => {
        let typedef = self.environment.find(t).shall_ok("identifier not found");
        if typedef.is_typedef() {
          Ok(typedef.qualified_type())
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
  fn externaldecl(&mut self) -> Vec<sd::ExternalDeclarationRef<'c>> {
    let mut declarations = Vec::new();
    std::mem::take(&mut self.program)
      .declarations
      .into_iter()
      .for_each(|decl| match self.declarations(decl) {
        Ok(declaration) => declarations.push(declaration),
        Err(e) => self.add_diag(e),
      });
    declarations
  }

  pub fn declarations(
    &mut self,
    declaration: pd::Declaration<'c>,
  ) -> Result<sd::ExternalDeclarationRef<'c>, Diag<'c>> {
    match declaration {
      pd::Declaration::Function(function) => Ok(
        sd::ExternalDeclarationRef::Function(self.functiondecl(function)?),
      ),
      pd::Declaration::Variable(vardef) =>
        Ok(sd::ExternalDeclarationRef::Variable(self.vardef(vardef)?)),
    }
  }

  pub fn functiondecl(
    &mut self,
    function: pd::Function<'c>,
  ) -> Result<sd::FunctionRef<'c>, Diag<'c>> {
    let pd::Function {
      body,
      declarator,
      declspecs,
      span,
    } = function;
    let (function_specifier, storage, return_type) = self
      .parse_declspecs(declspecs)
      .shall_ok("current implementation shall not return Err here");
    let storage = storage.unwrap_or(Storage::Extern);
    let pd::Declarator {
      modifiers,
      name,
      span: _,
    } = declarator;
    let name = name
      .shall_ok("function must have a name; it should be handled in parser");

    let (qualified_type, parameters) = self
      .apply_modifiers_for_functiondecl(return_type, modifiers)
      .shall_ok("failed to apply modifiers for function declarator");

    if name == "main" {
      Context::main_proto_validate(
        self.context(),
        qualified_type.unqualified_type.as_functionproto_unchecked(),
        function_specifier,
      )
      .unwrap_or_else(|e| {
        self.add_diag(e + span);
      });
    }
    use VarDeclKind::*;

    use crate::declref;

    let declkind = if body.is_some() {
      Definition
    } else {
      Declaration
    };

    let previous_decl = self.environment.find(name);
    if let Some(previous_decl) = previous_decl {
      if !Compatibility::compatible(
        &previous_decl.qualified_type(),
        &qualified_type,
      ) {
        Err(
          IncompatibleType(
            name,
            previous_decl.qualified_type().to_string(),
            qualified_type.to_string(),
          ) + Severity::Error
            + span,
        )?;
      }

      if matches!(previous_decl.declkind(), Tentative) {
        contract_violation!("function cannot be tentative");
      }

      if matches!(declkind, Definition) && previous_decl.definition().is_some()
      {
        Err(FunctionAlreadyDefined(name.to_string()) + Severity::Error + span)?;
      }
    }

    let declaration_type = previous_decl.map_or(qualified_type, |previous| {
      Compatibility::composite_unchecked(
        &previous.qualified_type(),
        &qualified_type,
        self.context(),
      )
    });

    let declaration = declref::DeclNode::alloc(
      self.context(),
      declaration_type,
      storage,
      name,
      declkind,
      previous_decl,
    );

    _ = self
      .environment
      .declare(name, declaration)
      .map_err(|e| self.add_error(e, span));

    let function = sd::Function::new(
      self.context(),
      declaration,
      parameters,
      function_specifier,
      None,
      span,
    );

    match body {
      Some(body) => match self.current_function {
        Some(_) => contract_violation!(
          "nested function definition is not allowed; 
          this should be handled in parser: current function {}, new function \
           {}
          
          Also: this may occur if the `current_function` is not properly \
           cleared 
          after an `Err` returned of the previous function definition analysis",
          self.current_function.as_ref().unwrap().declaration.name(),
          function.declaration.name()
        ),
        None => self.function_with_body(body, function),
      },
      None => Ok(sd::Function::alloc(self.context(), function)),
    }
  }

  fn function_with_body(
    &mut self,
    body: ps::Compound<'c>,
    function: sd::Function<'c>,
  ) -> Result<sd::FunctionRef<'c>, Diag<'c>> {
    self.current_labels.clear();
    self.current_gotos.clear();
    self.current_function = Some(function);

    self.environment.enter();
    self.scope_context.push(ScopeContext::Function);

    self
      .current_function
      .as_ref()
      .shall_ok("shall have function")
      .parameters
      .iter()
      .for_each(|parameter| {
        // FIXME: hsould we insert unnamed parameters or not?
        if parameter.declaration.name().starts_with('<') {
          // unnamed parameter - do nothing currently
        } else {
          _ = self
            .environment
            .declare(parameter.declaration.name(), parameter.declaration)
          // if it's incomplete type, this has already reported when building the functionproto.
          // .map_err(|e| self.add_error(e, parameter.span));
        }
      });

    let statements = self.statements(body.statements);

    let _func = self.scope_context.pop();
    self.environment.exit();
    debug_assert!(matches!(_func, Some(ScopeContext::Function)));

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
      .collect_in::<ArenaVec<_>>(self.context().arena())
      .into_bump_slice();
    let gotos = self
      .current_gotos
      .iter()
      .copied()
      .collect_in::<ArenaVec<_>>(self.context().arena())
      .into_bump_slice();
    let analyzed_body =
      ss::Compound::new(self.context(), statements, body.span);

    {
      let function = self
        .current_function
        .as_mut()
        .shall_ok("impossible; no current function?");
      function.body = Some(analyzed_body);
      function.labels = labels;
      function.gotos = gotos;
    }

    self.current_labels.clear();
    self.current_gotos.clear();
    let function =
      std::mem::take(&mut self.current_function).shall_ok("never fails");
    Ok(sd::Function::alloc(self.context(), function))
  }

  pub fn vardef(
    &mut self,
    vardef: pd::VarDef<'c>,
  ) -> Result<sd::VarDefRef<'c>, Diag<'c>> {
    let pd::VarDef {
      declarator,
      declspecs,
      initializer,
      span,
    } = vardef;
    let pd::Declarator {
      modifiers,
      name,
      span: _,
    } = declarator;
    let name = name
      .shall_ok("variable must have a name; it should be handled in parser");

    let (storage, qualified_type, initializer) = if declspecs
      .type_specifiers
      .len()
      == 1
      && matches!(
        unsafe { declspecs.type_specifiers.first().unwrap_unchecked() },
        ::rcc_parse::declaration::TypeSpecifier::AutoType
      ) {
      let storage = declspecs.storage_class;
      let function_specifier = declspecs.function_specifiers;
      contract_assert!(
        function_specifier.is_empty(),
        "variable cannot have function specifier; this should be handled in \
         parser"
      );
      let out = initializer.map(|initializer| {
        self.initializer(
          initializer,
          None,
          self.environment.is_global()
            || matches!(
              storage,
              Some(Storage::Constexpr | Storage::Static | Storage::ThreadLocal)
            ),
        )
      });
      if let Some((initializer, qualified_type)) = out {
        (storage, qualified_type, Some(initializer))
      } else {
        Err(
          DeducedTypeWithNoInitializer(name.to_string())
            + Severity::Error
            + span,
        )?
      }
    } else {
      let (function_specifier, storage, raw_qualified_type) =
        self.parse_declspecs(declspecs).shall_ok("vardef");
      contract_assert!(
        function_specifier.is_empty(),
        "variable cannot have function specifier; this should be handled in \
         parser"
      );
      let raw_qualified_type =
        self.apply_modifiers_for_varty(raw_qualified_type, modifiers);
      let out = initializer.map(|initializer| {
        self.initializer(
          initializer,
          Some(raw_qualified_type),
          self.environment.is_global()
            || matches!(
              storage,
              Some(Storage::Constexpr | Storage::Static | Storage::ThreadLocal)
            ),
        )
      });
      if let Some((initializer, qualified_type)) = out {
        (storage, qualified_type, Some(initializer))
      } else {
        (storage, raw_qualified_type, None)
      }
    };
    // arr can have 1st extend incomplete, handled downstream at init
    if !qualified_type.is_complete() && !qualified_type.is_array() {
      Err(
        DeclarationTyIncomplete(name.into(), qualified_type.to_string())
          + Severity::Error
          + span,
      )?;
    }

    let previous_decl = self.environment.shallow_find(name);

    let vardef = match self.environment.is_global() {
      true => self.global_vardef(
        storage,
        qualified_type,
        name,
        initializer,
        previous_decl,
        span,
      ),
      false => self.local_vardef(
        storage.unwrap_or(Storage::Automatic),
        qualified_type,
        name,
        initializer,
        previous_decl,
        span,
      ),
    }
    .shall_ok("failed to create vardef");
    // no prev - just insert
    // if found a *real* definition and current vardef is also a real refinition -> error
    // prev: extern -- update storage class (and possibly initializer)
    // prev: tentative -- update to definition
    // prev: declaration -- update to definition
    // prev: typedef w/ current vardef or vice versa -> override
    // prev and cur all typedef -> if all same nothing, otherwise error
    if let Some(prev_decl_ref) = previous_decl {
      if !Compatibility::compatible(
        &prev_decl_ref.qualified_type(),
        &vardef.declaration.qualified_type(),
      ) {
        Err(
          IncompatibleType(
            name,
            prev_decl_ref.qualified_type().to_string(),
            vardef.declaration.qualified_type().to_string(),
          ) + Severity::Error
            + span,
        )?
      }
      let prev_declkind = prev_decl_ref.declkind();
      let new_declkind = vardef.declaration.declkind();
      #[allow(clippy::upper_case_acronyms)]
      type VDK = VarDeclKind;
      match (&prev_declkind, &new_declkind) {
        (VDK::Definition, VDK::Definition) => Err(
          VariableAlreadyDefined(vardef.declaration.name().to_string())
            + Severity::Error
            + span,
        )?,
        (VDK::Definition, VDK::Declaration)
        | (VDK::Definition, VDK::Tentative) => {
          // valid and nothing to do
        },
        (VDK::Declaration, VDK::Definition)
        | (VDK::Tentative, VDK::Definition) => {
          let merged_storage = Storage::try_merge(
            &prev_decl_ref.storage_class(),
            &vardef.declaration.storage_class(),
          )
          .unwrap_or_else(|error| {
            self.add_diag(error + span);
            prev_decl_ref.storage_class()
          });

          vardef.declaration.set_storage_class(merged_storage);
          vardef.declaration.set_qualified_type(
            QualifiedType::composite_unchecked(
              &vardef.declaration.qualified_type(),
              &prev_decl_ref.qualified_type(),
              self.context(),
            ),
          );
        },
        (VDK::Declaration, VDK::Declaration)
        | (VDK::Tentative, VDK::Tentative)
        | (VDK::Declaration, VDK::Tentative)
        | (VDK::Tentative, VDK::Declaration) => {
          // only merge storage class if needed, todo
        },
      }
    }

    _ = self
      .environment
      .declare(name, vardef.declaration)
      .map_err(|e| self.add_error(e, span));
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

  fn global_vardef(
    &self,
    storage: Option<Storage>,
    qualified_type: QualifiedType<'c>,
    name: StrRef<'c>,
    initializer: Option<sd::Initializer<'c>>,
    previous_decl: Option<sd::DeclRef<'c>>,
    span: SourceSpan,
  ) -> Result<sd::VarDefRef<'c>, Diag<'c>> {
    Ok(match (storage, initializer) {
      (None, None) => {
        let declaration = declref::DeclNode::tentative(
          self.context(),
          qualified_type,
          Storage::Extern,
          name,
          previous_decl,
        );
        sd::VarDef::new(self.context(), declaration, None, span)
      },
      (None, Some(initializer)) => {
        let declaration = declref::DeclNode::def(
          self.context(),
          qualified_type,
          Storage::Extern,
          name,
          previous_decl,
        );
        sd::VarDef::new(self.context(), declaration, Some(initializer), span)
      },
      (Some(Storage::Register), initializer) => {
        self.add_error(GlobalRegVar(name.to_string()), span);
        let declkind = if initializer.is_some() {
          VarDeclKind::Definition
        } else {
          VarDeclKind::Declaration
        };
        let declaration = declref::DeclNode::alloc(
          self.context(),
          qualified_type,
          Storage::Extern,
          name,
          declkind,
          previous_decl,
        );
        sd::VarDef::new(self.context(), declaration, initializer, span)
      },
      (Some(storage), None) => sd::VarDef::new(
        self.context(),
        declref::DeclNode::decl(
          self.context(),
          qualified_type,
          storage,
          name,
          previous_decl,
        ),
        None,
        span,
      ),
      (Some(storage), Some(initializer)) => {
        let storage = match storage {
          Storage::Extern => {
            self.add_warning(
              ExternVariableWithInitializer(name.to_string()),
              span,
            );
            storage
          },
          _ => storage,
        };
        sd::VarDef::new(
          self.context(),
          declref::DeclNode::def(
            self.context(),
            qualified_type,
            storage,
            name,
            previous_decl,
          ),
          Some(initializer),
          span,
        )
      },
    })
  }

  fn local_vardef(
    &self,
    storage: Storage,
    qualified_type: QualifiedType<'c>,
    name: StrRef<'c>,
    initializer: Option<sd::Initializer<'c>>,
    previous_decl: Option<sd::DeclRef<'c>>,
    span: SourceSpan,
  ) -> Result<sd::VarDefRef<'c>, Diag<'c>> {
    if storage == Storage::Extern && initializer.is_some() {
      self.add_error(LocalExternVarWithInitializer(name.to_string()), span);
    }
    if (qualified_type
      .as_array()
      .is_some_and(|array| !array.is_complete()))
      && initializer.is_none()
    {
      self.add_error(
        IncompleteArrayDefNoInit(name.into(), qualified_type.to_string()),
        span,
      );
    }
    let declaration = declref::DeclNode::def(
      self.context(),
      qualified_type,
      storage,
      name,
      previous_decl,
    );
    Ok(sd::VarDef::new(
      self.context(),
      declaration,
      initializer,
      span,
    ))
  }
}

impl<'c> Sema<'c> {
  pub(crate) fn expression(
    &self,
    expression: pe::Expression<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    match expression {
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
    }
  }

  fn sizeof(
    &self,
    sizeof: pe::SizeOf<'c>,
  ) -> Result<se::ExprRef<'c>, Diag<'c>> {
    match sizeof.sizeof {
      pe::SizeOfKind::Expression(expression) => {
        let analyzed_expr = self.expression(*expression).handle_with(
          self,
          se::Expression::new_error_node(
            self.context(),
            self.context().ulong_long_type().into(),
          ),
        );
        let size = analyzed_expr.unqualified_type().size();
        Ok(se::Expression::new_rvalue(
          self.context(),
          se::RawExpr::Constant(se::Constant::Integral(size.into())),
          self.context().ulong_long_type().into(),
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
          let (_, _, base_type) =
            self.parse_declspecs(declspecs).shall_ok("sizeof type");
          self.apply_modifiers_for_varty(base_type, declarator.modifiers)
        };
        Ok(se::Expression::new_rvalue(
          self.context(),
          se::RawExpr::Constant(se::Constant::Integral(
            qualified_type.size().into(),
          )),
          self.context().ulong_long_type().into(),
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
    let analyzed_callee = self.expression(*callee)?
    // .decay(self.context()) // this `should` decay, but clang decay only if the return result is assigned to a variable.
    ;

    let function_proto = match analyzed_callee.unqualified_type() {
      Type::FunctionProto(proto) => proto,
      Type::Pointer(ptr) => match ptr.pointee.unqualified_type {
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
          .lvalue_conversion(self.context())
          .decay(self.context())
          .assignment_conversion(self.context(), formal)
          .handle_with(self, actual)
      })
      .collect::<Vec<_>>();
    Ok(se::Expression::new_rvalue(
      self.context(),
      se::Call::new(
        self.context(),
        analyzed_callee,
        converted_analyzed_arguments,
      ),
      expr_type,
      span,
    ))
  }

  fn paren(&self, paren: pe::Paren<'c>) -> Result<se::ExprRef<'c>, Diag<'c>> {
    let pe::Paren { expr, span } = paren;
    let analyzed_expr = self.expression(*expr)?;
    let expr_type = *analyzed_expr.qualified_type();
    Ok(se::Expression::new_rvalue(
      self.context(),
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
      contract_violation!(
        "variable cannot be a typedef; this should be handled in parser"
      );
    } else {
      Ok(se::Expression::new_lvalue(
        self.context(),
        se::Variable::new(declaration),
        declaration.qualified_type(),
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
    let unqualified_type = constant.unqualified_type(self.context());
    let value_category = if constant.is_char_array() {
      // 6.5.2p5: A string literal is [...] an lvalue [...].
      se::ValueCategory::LValue
    } else {
      se::ValueCategory::RValue
    };
    Ok(se::Expression::new(
      self.context(),
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
      .lvalue_conversion(self.context())
      .decay(self.context())
      .is_contextually_convertible_to_bool()?;

    if let Some(then) = pe_then_expr {
      let then_expr = self.expression(*then)?;
      let else_expr = self.expression(*pe_else_expr)?;

      match (then_expr.unqualified_type(), else_expr.unqualified_type()) {
        (left_type, right_type)
          if left_type.is_void() || right_type.is_void() =>
          Ok(se::Expression::new_rvalue(
            self.context(),
            se::Ternary::new(
              condition,
              se::Expression::void_conversion(then_expr, self.context()),
              se::Expression::void_conversion(else_expr, self.context()),
            ),
            self.ast().void_type().into(),
            span,
          )),
        // both arithmetic -> usual arithmetic conversion
        (left_type, right_type)
          if left_type.is_arithmetic() && right_type.is_arithmetic() =>
        {
          let (then_converted, else_converted, result_type) =
            se::Expression::usual_arithmetic_conversion(
              then_expr,
              else_expr,
              self.context(),
            )?;
          Ok(se::Expression::new_rvalue(
            self.context(),
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
            self.context(),
          ) {
            Some(qualified_type) => {
              let result_type = Type::Pointer(Pointer::new(qualified_type))
                .lookup(self.context())
                .into();
              Ok(se::Expression::new_rvalue(
                self.context(),
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
    let lhs = self
      .expression(*array)?
      .lvalue_conversion(self.context())
      .decay(self.context());
    let rhs = self
      .expression(*index)?
      .lvalue_conversion(self.context())
      .decay(self.context());

    let doit = |array_side: se::ExprRef<'c>, index_side: se::ExprRef<'c>| {
      let analyzed_index = index_side; // .ptrdiff_conversion_unchecked(self.context());
      let elem_type =
        array_side.unqualified_type().as_pointer_unchecked().pointee;
      // store the pointer(decayed array) and index here, not the array here... maybe a wrong idesa, idk for now.
      se::Expression::new_lvalue(
        self.context(),
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
    let operand = operand
      .lvalue_conversion(self.context())
      .decay(self.context());

    if !operand.unqualified_type().is_arithmetic() {
      Err(
        NonArithmeticInUnaryOp(operator, operand.to_string())
          + Severity::Error
          + span,
      )
    } else {
      let converted_operand =
        operand.usual_arithmetic_conversion_unary(self.context())?;
      let expr_type = *converted_operand.qualified_type();
      Ok(se::Expression::new_rvalue(
        self.context(),
        se::Unary::prefix(operator, converted_operand),
        expr_type,
        span,
      ))
    }
  }

  /// i didnt came up with a better name...
  ///
  /// 6.5.4.1.2: The expression `++E` is equivalent to `(E+=1)`,
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
    if !operand.is_modifiable_lvalue() {
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
        self.context(),
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
    let operand = operand
      .lvalue_conversion(self.context())
      .decay(self.context());

    if !operand.unqualified_type().is_integer() {
      Err(
        NonIntegerInBitwiseUnaryOp(operator, operand.to_string())
          + Severity::Error
          + span,
      )
    } else {
      let converted_operand =
        operand.usual_arithmetic_conversion_unary(self.context())?;
      let expr_type = *converted_operand.qualified_type();
      Ok(se::Expression::new_rvalue(
        self.context(),
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
    let operand = operand
      .lvalue_conversion(self.context())
      .decay(self.context());

    let converted_operand = operand
      .lvalue_conversion(self.context())
      .is_contextually_convertible_to_bool()?;
    Ok(se::Expression::new_rvalue(
      self.context(),
      se::Unary::prefix(operator, converted_operand),
      self.context().converted_bool().into(),
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
    } else if matches!(operand.raw_expr(), se::RawExpr::Variable(variable) if variable.declaration.storage_class().is_register())
    {
      Err(AddressofOperandRegVar(operand.to_string()) + Severity::Error + span)
    } else {
      let pointee = *operand.qualified_type();
      Ok(se::Expression::new_rvalue(
        self.context(),
        se::Unary::prefix(operator, operand),
        Type::Pointer(Pointer::new(pointee))
          .lookup(self.context())
          .into(),
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

    let operand = operand
      .lvalue_conversion(self.context())
      .decay(self.context());

    if !operand.unqualified_type().is_pointer() {
      Err(DerefNonPtr(operand.to_string()) + Severity::Error + span)?
    }

    let pointee_type =
      &operand.unqualified_type().as_pointer_unchecked().pointee;
    if RefEq::ref_eq(pointee_type.unqualified_type, self.context().void_type())
    {
      Err(DerefVoidPtr(operand.to_string()) + Severity::Error + span)
    } else {
      // If the operand points to a function, the result is a function designator; -- which means the we don't need to perform decay here
      // if it points to an object, the result is an lvalue designating the object.
      // If the operand has type "pointer to type", the result has type "type".
      // If an invalid value has been assigned to the pointer, the behavior is undefined.
      let expr_type = *pointee_type;
      Ok(se::Expression::new_lvalue(
        self.context(),
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
      _ if !left.is_modifiable_lvalue() => {
        self.add_error(ExprNotAssignable(left.to_string()), span);
        Ok(left)
      },
      // plain operator `=`.
      None => {
        let assigned_expr = right
          .lvalue_conversion(self.context())
          .decay(self.context())
          .assignment_conversion(self.context(), &expr_type)?;
        Ok(se::Expression::new_rvalue(
          self.context(),
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
        } = intermediate.raw_expr().as_binary_unchecked();

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
          self.context(),
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
      .lvalue_conversion(self.context())
      .decay(self.context())
      .is_contextually_convertible_to_bool()?;

    let rhs = right
      .lvalue_conversion(self.context())
      .decay(self.context())
      .is_contextually_convertible_to_bool()?;

    Ok(se::Expression::new_rvalue(
      self.context(),
      se::Binary::new(operator, lhs, rhs),
      self.context().converted_bool().into(),
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
    let left = left.lvalue_conversion(self.context()).decay(self.context());
    let right = right
      .lvalue_conversion(self.context())
      .decay(self.context());
    match (left.unqualified_type(), right.unqualified_type()) {
      (l, r) if l.is_arithmetic() && r.is_arithmetic() => {
        let (lhs, rhs, _common_type) =
          se::Expression::usual_arithmetic_conversion(
            left,
            right,
            self.context(),
          )?;

        Ok(se::Expression::new_rvalue(
          self.context(),
          se::Binary::new(operator, lhs, rhs),
          self.context().converted_bool().into(),
          span,
        ))
      },
      (
        Type::Primitive(Primitive::Nullptr),
        Type::Primitive(Primitive::Nullptr),
      ) if matches!(operator, Operator::EqualEqual | Operator::NotEqual) =>
        Ok(se::Expression::new_rvalue(
          self.context(),
          se::Binary::from_operator_unchecked(operator, left, right),
          self.context().converted_bool().into(),
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
    let ptr_with_nullptr = |ptr: se::ExprRef<'c>, nullptr: se::ExprRef<'c>| {
      let ptr_type = *ptr.qualified_type();
      let casted_nullptr = se::Expression::new_rvalue(
        self.context(),
        se::ImplicitCast::new(
          nullptr,
          ::rcc_ast::types::CastType::NullptrToPointer,
        ),
        ptr_type,
        span,
      );
      Ok(se::Expression::new_rvalue(
        self.context(),
        se::Binary::from_operator_unchecked(operator, ptr, casted_nullptr),
        self.context().converted_bool().into(),
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
            self.context(),
            se::Binary::new(operator, left, right),
            self.context().converted_bool().into(),
            span,
          ))
        } else {
          Err(
            CompareDistinctPointerTypes(
              left.qualified_type().to_string(),
              right.qualified_type().to_string(),
            )
            +(Severity::Error) // should be warning
            +span,
          )
        }
      },

      (Type::Pointer(ptr), Type::Primitive(Primitive::Nullptr))
        if matches!(operator, Operator::EqualEqual | Operator::NotEqual) =>
        ptr_with_nullptr(left, right),
      (Type::Primitive(Primitive::Nullptr), Type::Pointer(ptr))
        if matches!(operator, Operator::EqualEqual | Operator::NotEqual) =>
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
    let left = left.lvalue_conversion(self.context()).decay(self.context());
    let right = right
      .lvalue_conversion(self.context())
      .decay(self.context());

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
      se::Expression::usual_arithmetic_conversion(left, right, self.context())?;

    Ok(se::Expression::new_rvalue(
      self.context(),
      se::Binary::new(operator, lhs, rhs),
      result_type,
      span,
    ))
  }

  /// at least one of the operand is pointer, and the operand can only be `+` or `-`.
  ///
  /// This is specified more detailed in C++ Standard [over.built].
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
            self.context(),
            se::Binary::new(operator, left, right),
            self.context().ptrdiff_type().into(), // no qual for pointer differences
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

      // be aware that the left and right switched their position in order to make irgen earsier.
      (Type::Primitive(lhs), Type::Pointer(_))
        if lhs.is_integer() && operator == Operator::Plus =>
      {
        let ptrty = right.unqualified_type().clone().lookup(self.context());
        Ok(se::Expression::new_rvalue(
          self.context(),
          se::Binary::new(operator, right, left),
          ptrty.into(),
          span,
        ))
      },
      // ptr + int => ptr
      (Type::Pointer(_), Type::Primitive(rhs))
        if rhs.is_integer() && operator == Operator::Plus =>
      {
        let ptrty = left.unqualified_type().clone().lookup(self.context());
        Ok(se::Expression::new_rvalue(
          self.context(),
          se::Binary::new(operator, left, right),
          ptrty.into(),
          span,
        ))
      },
      // ptr - int => ptr
      (Type::Pointer(_), Type::Primitive(rhs))
        if rhs.is_integer() && operator == Operator::Minus =>
      {
        let ptrty = left.unqualified_type().clone().lookup(self.context());
        let right = right.ptrdiff_conversion_unchecked(self.context());

        Ok(se::Expression::new_rvalue(
          self.context(),
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
    let lhs = left.lvalue_conversion(self.context()).decay(self.context());
    let rhs = right
      .lvalue_conversion(self.context())
      .decay(self.context());

    if !lhs.unqualified_type().is_integer()
      || !rhs.unqualified_type().is_integer()
    {
      self.add_error(
        NonIntegerInBitwiseBinaryOp(lhs.to_string(), rhs.to_string(), operator),
        span,
      );
    }

    let (left, right, result_type) =
      se::Expression::usual_arithmetic_conversion(lhs, rhs, self.context())?;

    Ok(se::Expression::new_rvalue(
      self.context(),
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
    let left = left
      .lvalue_conversion(self.context())
      .decay(self.context())
      .promote(self.context());
    let right = right
      .lvalue_conversion(self.context())
      .decay(self.context())
      .promote(self.context());

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
      self.context(),
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
      self.context(),
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
    .map(|statement| ss::Statement::alloc(self.context(), statement))
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

    Ok(ss::Compound::new(self.context(), statements, compound.span))
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
  ) -> Result<sd::ExternalDeclarationRef<'c>, Diag<'c>> {
    self.declarations(declaration)
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
      .declaration
      .qualified_type()
      .unqualified_type
      .as_functionproto_unchecked()
      .return_type;

    match (analyzed_expr, return_type.unqualified_type) {
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
          .lvalue_conversion(self.context())
          .decay(self.context())
          .assignment_conversion(self.context(), &return_type)?;
        Ok(ss::Return::new(Some(a), span))
      },
    }
  }

  fn ifstmt(&mut self, if_stmt: ps::If<'c>) -> Result<ss::If<'c>, Diag<'c>> {
    let ps::If {
      condition,
      then_branch,
      else_branch,
      span,
    } = if_stmt;
    let analyzed_condition = self
      .expression(condition)
      .and_then(|e| {
        e.lvalue_conversion(self.context())
          .is_contextually_convertible_to_bool()
      })
      .handle_with(
        self,
        se::Expression::new_error_node(
          self.context(),
          self.context().converted_bool().into(),
        ),
      );
    let analyzed_then_branch = self.statement_or_default(*then_branch);
    let analyzed_else_branch =
      else_branch.map(|else_branch| self.statement_or_default(*else_branch));
    Ok(ss::If::new(
      self.context(),
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
    let analyzed_condition = self
      .expression(condition)
      .and_then(|e| {
        e.lvalue_conversion(self.context())
          .is_contextually_convertible_to_bool()
      })
      .handle_with(
        self,
        se::Expression::new_error_node(
          self.context(),
          self.context().converted_bool().into(),
        ),
      );
    self.scope_context.push(ScopeContext::Loop);
    let analyzed_body = self.statement_or_default(*body);
    let _while = self.scope_context.pop();
    debug_assert!(
      matches!(_while, Some(ScopeContext::Loop)),
      "scope context stack corrupted: expected loop context"
    );

    Ok(ss::While::new(
      self.context(),
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

    let analyzed_condition = self
      .expression(condition)
      .and_then(|e| {
        e.lvalue_conversion(self.context())
          .is_contextually_convertible_to_bool()
      })
      .handle_with(
        self,
        se::Expression::new_error_node(
          self.context(),
          self.context().converted_bool().into(),
        ),
      );
    Ok(ss::DoWhile::new(
      self.context(),
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
        se::Expression::new_error_node(
          self.context(),
          self.context().converted_bool().into(),
        ),
      )
    });
    let analyzed_increment = increment.map(|inc| {
      self.expression(inc).handle_with(
        self,
        se::Expression::new_error_node(
          self.context(),
          self.context().int_type().into(),
        ),
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
      self.context(),
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
        se::Expression::new_error_node(
          self.context(),
          self.context().int_type().into(),
        )
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
      self.context(),
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
      se::Expression::new_error_node(
        self.context(),
        self.context().int_type().into(),
      ),
    );
    let analyzed_body = self.statements(body);

    Ok(ss::Case::new(
      self.context(),
      analyzed_value.fold(self.session).transform(|expr| {
        if let se::RawExpr::Constant(constant) = expr.raw_expr() {
          if constant.is_integral() {
            constant.clone()
          } else {
            self.add_error(
              NonIntegerInCaseStmt(constant.to_string()),
              expr.span(),
            );
            Integral::default().into()
          }
        } else {
          contract_violation!("constant folding did not yield a constant")
        }
      }),
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
    Ok(ss::Default::new(self.context(), analyzed_body, span))
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
            self.context(),
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
