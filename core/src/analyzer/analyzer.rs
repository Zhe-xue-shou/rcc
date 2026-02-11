use ::rcc_utils::{
  IntoWith, contract_assert, contract_violation, not_implemented_feature,
};

use crate::{
  analyzer::{
    declaration as ad,
    expression::{self as ae},
    statement as astmt,
  },
  common::{
    Environment, Integral, Operator, OperatorCategory, SourceSpan, Storage,
    Symbol, VarDeclKind,
  },
  diagnosis::{
    Diag,
    DiagData::{self, *},
    Diagnosis, Severity,
  },
  parser::{declaration as pd, expression as pe, statement as ps},
  session::Session,
  types::{
    Array, ArraySize, Compatibility, FunctionProto, FunctionSpecifier, Pointer,
    Primitive, QualifiedType, Type, TypeInfo,
  },
};

type TypeRes = Result<QualifiedType, Diag>;
type ExprRes = Result<ae::Expression, Diag>;
type DeclRes<T> = Result<T, Diag>;
type StmtRes<T> = Result<T, Diag>;

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

trait ImplHelper<T> {
  /// Glorified `expect` for `Result`, use this to indicate a `program error/invariant`
  ///
  /// - `.expect("some message")` -> (prob) for user side error(although rarely use this way)
  /// - `.shall_ok("some message")` -> for program internal invariant which indicates the problem is in the implementation
  fn shall_ok<M: Into<Option<&'static str>>>(self, msg: M) -> T;
}

impl<T> ImplHelper<T> for Result<T, Diag> {
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
impl<T> ImplHelper<T> for Option<T> {
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

trait ImplHelper2<T, Listener> {
  fn handle_with(self, context: &Listener, default: T) -> T;
}

impl<T> ImplHelper2<T, Analyzer<'_>> for Result<T, Diag> {
  /// if it's error, log it, and return a default value (means error)
  fn handle_with(self, context: &Analyzer, default: T) -> T {
    match self {
      Ok(t) => t,
      Err(e) => {
        context.add_diag(e);
        default
      },
    }
  }
}

trait ImplHelper3<T, Listener> {
  fn handle_or_default(self, context: &Listener) -> T;
}

impl<T: ::std::default::Default> ImplHelper3<T, Analyzer<'_>>
  for Result<T, Diag>
{
  fn handle_or_default(self, context: &Analyzer) -> T {
    match self {
      Ok(t) => t,
      Err(e) => {
        context.add_diag(e);
        ::std::default::Default::default()
      },
    }
  }
}

#[derive(Debug)]
pub struct Analyzer<'session> {
  program: pd::Program,
  environment: Environment,
  current_function: Option<ad::Function>,
  session: &'session Session,
}

impl<'session> Analyzer<'session> {
  pub fn new(program: pd::Program, session: &'session Session) -> Self {
    Self {
      program,
      session,
      environment: Environment::default(),
      current_function: None,
    }
  }

  pub fn add_diag(&self, diag: Diag) {
    self.session.diagnosis.add_diag(diag);
  }

  pub fn add_error(&self, error: DiagData, span: SourceSpan) {
    self.session.diagnosis.add_error(error, span);
  }

  pub fn add_warning(&self, warning: DiagData, span: SourceSpan) {
    self.session.diagnosis.add_warning(warning, span);
  }

  pub fn analyze(&mut self) -> ad::TranslationUnit {
    self.environment.enter();
    let translation_unit = ad::TranslationUnit::new(self.externaldecl());
    self.environment.exit();
    translation_unit
  }

  pub fn unnamed_placeholder() -> String {
    static COUNTER: ::std::sync::atomic::AtomicUsize =
      ::std::sync::atomic::AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
    format!("<unnamed_{}>", id)
  }
}
impl<'session> Analyzer<'session> {
  fn apply_modifiers_for_varty(
    &self,
    mut qualified_type: QualifiedType,
    modifiers: Vec<pd::Modifier>,
  ) -> QualifiedType {
    // reverse order
    for modifier in modifiers.into_iter().rev() {
      match modifier {
        pd::Modifier::Pointer(qualifiers) => {
          qualified_type = QualifiedType::new(
            qualifiers,
            Type::Pointer(Pointer::new(qualified_type)).into(),
          );
        },
        pd::Modifier::Array(array_modifier) => {
          let size = match array_modifier.bound {
            None => ArraySize::Incomplete,
            Some(expr) => {
              // check 1. it's a constant expression or not, 2. it's type should be integer type 3. should be non-negative
              let analyzed_expr = self.expression(expr).handle_with(
                self,
                ae::Expression::new_error_node(QualifiedType::int()),
              );

              if analyzed_expr.qualified_type().is_scalar() {
                match analyzed_expr.fold(&self.session.diagnosis) {
                  super::folding::FoldingResult::Success(v) =>
                    if v.is_integer_constant() {
                      ArraySize::Constant(
                        match v.raw_expr().as_constant_unchecked().value {
                          ae::ConstantLiteral::Integral(integral) =>
                            integral.to_builtin(),
                          ae::ConstantLiteral::Floating(_) => unreachable!(),
                          ae::ConstantLiteral::String(_) => unreachable!(),
                          ae::ConstantLiteral::Nullptr(_) => 0,
                        },
                      )
                    } else {
                      self.add_error(
                        NonIntegerInArraySubscript(v.to_string()),
                        v.span(),
                      );
                      ArraySize::Constant(0)
                    },
                  super::folding::FoldingResult::Failure(_) => {
                    todo!("VLA")
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
          qualified_type = Array {
            element_type: qualified_type,
            size,
          }
          .into();
        },
        pd::Modifier::Function(function_signature) => {
          // func ptr or so
          let pd::FunctionSignature {
            parameters,
            is_variadic,
          } = function_signature;
          let analyzed_parameter_types = self.parse_parameter_types(parameters);
          qualified_type = FunctionProto::new(
            qualified_type,
            analyzed_parameter_types,
            is_variadic,
          )
          .into();
        },
      }
    }
    qualified_type
  }

  fn apply_modifiers_for_functiondecl(
    &self,
    return_type: QualifiedType,
    modifiers: Vec<pd::Modifier>,
  ) -> DeclRes<(
    QualifiedType,
    Vec<ad::Parameter>, /* parameters name and their type, here's some repetition
                        parameter type had also been inside QualifiedType of the function */
  )> {
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
    let parameters = self
      .parse_parameters(function_signature.parameters)
      .shall_ok("failed to parse function parameters");
    let is_variadic = function_signature.is_variadic;
    let parameter_types = parameters
      .iter()
      .map(|param| param.symbol.borrow().qualified_type.clone())
      .collect();
    let functionproto =
      FunctionProto::new(return_type, parameter_types, is_variadic);

    Ok((functionproto.into(), parameters))
  }

  fn parse_parameter_types(
    &self,
    parameters: Vec<pd::Parameter>,
  ) -> Vec<QualifiedType> {
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
          storage.is_none(),
          "parameter cannot have storage class specifier; this should be \
           handled in parser.
          also, `register` is currently unimplemented"
        );
        let pd::Declarator {
          modifiers,
          name: _,
          span: _,
        } = declarator;
        self.apply_modifiers_for_varty(base_type, modifiers)
      })
      .collect()
  }

  fn parse_parameters(
    &self,
    parameters: Vec<pd::Parameter>,
  ) -> DeclRes<Vec<ad::Parameter>> {
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
          storage.is_none(),
          "parameter cannot have storage class specifier; this should be \
           handled in parser.
          also, `register` is currently unimplemented"
        );
        let pd::Declarator {
          modifiers,
          name,
          span: _,
        } = declarator;
        let qualified_type =
          self.apply_modifiers_for_varty(base_type, modifiers);
        let symbol = Symbol::new_ref(Symbol::new(
          qualified_type,
          Storage::Automatic,
          name.unwrap_or_else(Self::unnamed_placeholder),
          VarDeclKind::Declaration,
        ));
        Ok(ad::Parameter::new(symbol, span))
      })
      .collect()
  }

  fn parse_declspecs(
    &self,
    declspecs: pd::DeclSpecs,
  ) -> Result<(FunctionSpecifier, Option<Storage>, QualifiedType), Diag> {
    let qualified_type = self
      .get_type(declspecs.type_specifiers)
      .handle_with(self, QualifiedType::int())
      .with_qualifiers(declspecs.qualifiers);
    let storage_class = declspecs.storage_class;
    let function_specifier = declspecs.function_specifiers;

    Ok((function_specifier, storage_class, qualified_type))
  }

  fn get_type(&self, mut type_specifiers: Vec<pd::TypeSpecifier>) -> TypeRes {
    assert!(!type_specifiers.is_empty());
    assert!(type_specifiers.len() <= 5); // unsigned long long int complex (integer complex not in standard) is the max
    type_specifiers.sort_by_key(|s| s.sort_key());
    type TS = pd::TypeSpecifier;
    // 6.7.3.1
    match type_specifiers.as_slice() {
      [TS::Nullptr] => Ok(Type::Primitive(Primitive::Nullptr).into()),
      [TS::Void] => Ok(Type::Primitive(Primitive::Void).into()),

      [TS::Bool] => Ok(Type::Primitive(Primitive::Bool).into()),

      [TS::Char] => Ok(Type::Primitive(Primitive::Char).into()),
      [TS::Signed, TS::Char] => Ok(Type::Primitive(Primitive::SChar).into()),
      [TS::Unsigned, TS::Char] => Ok(Type::Primitive(Primitive::UChar).into()),

      [TS::Short]
      | [TS::Short, TS::Int]
      | [TS::Signed, TS::Short]
      | [TS::Signed, TS::Short, TS::Int] =>
        Ok(Type::Primitive(Primitive::Short).into()),
      [TS::Unsigned, TS::Short] | [TS::Unsigned, TS::Short, TS::Int] =>
        Ok(Type::Primitive(Primitive::UShort).into()),

      [TS::Int] | [TS::Signed] | [TS::Signed, TS::Int] =>
        Ok(Type::Primitive(Primitive::Int).into()),
      [TS::Unsigned] | [TS::Unsigned, TS::Int] =>
        Ok(Type::Primitive(Primitive::UInt).into()),

      [TS::Long]
      | [TS::Long, TS::Int]
      | [TS::Signed, TS::Long]
      | [TS::Signed, TS::Long, TS::Int] =>
        Ok(Type::Primitive(Primitive::Long).into()),
      [TS::Unsigned, TS::Long] | [TS::Unsigned, TS::Long, TS::Int] =>
        Ok(Type::Primitive(Primitive::ULong).into()),

      [TS::Long, TS::Long]
      | [TS::Long, TS::Long, TS::Int]
      | [TS::Signed, TS::Long, TS::Long]
      | [TS::Signed, TS::Long, TS::Long, TS::Int] =>
        Ok(Type::Primitive(Primitive::LongLong).into()),
      [TS::Unsigned, TS::Long, TS::Long]
      | [TS::Unsigned, TS::Long, TS::Long, TS::Int] =>
        Ok(Type::Primitive(Primitive::ULongLong).into()),

      [TS::Float] => Ok(Type::Primitive(Primitive::Float).into()),
      [TS::Double] => Ok(Type::Primitive(Primitive::Double).into()),
      [TS::Long, TS::Double] =>
        Ok(Type::Primitive(Primitive::LongDouble).into()),

      [TS::Float, TS::Complex] =>
        Ok(Type::Primitive(Primitive::ComplexFloat).into()),
      [TS::Double, TS::Complex] =>
        Ok(Type::Primitive(Primitive::ComplexDouble).into()),
      [TS::Long, TS::Double, TS::Complex] =>
        Ok(Type::Primitive(Primitive::ComplexLongDouble).into()),

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
        if typedef.borrow().is_typedef() {
          Ok(typedef.borrow().qualified_type.clone())
        } else {
          contract_violation!("identifier is not a typedef");
        }
      },
      // skip _BitInt, _Decimal32, _Decimal64, _Decimal128 here
      _ => not_implemented_feature!("{:#?}", type_specifiers.as_slice()),
    }
  }
}

impl<'session> Analyzer<'session> {
  fn externaldecl(&mut self) -> Vec<ad::ExternalDeclaration> {
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
    declaration: pd::Declaration,
  ) -> DeclRes<ad::ExternalDeclaration> {
    match declaration {
      pd::Declaration::Function(function) => Ok(
        ad::ExternalDeclaration::Function(self.functiondecl(function)?),
      ),
      pd::Declaration::Variable(vardef) =>
        Ok(ad::ExternalDeclaration::Variable(self.vardef(vardef)?)),
    }
  }

  pub fn functiondecl(
    &mut self,
    function: pd::Function,
  ) -> DeclRes<ad::Function> {
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
      FunctionProto::validate_main_proto(
        qualified_type
          .unqualified_type()
          .as_functionproto_unchecked(),
        function_specifier,
      )
      .unwrap_or_else(|e| {
        self.add_diag(e.into_with(span));
      });
    }
    use VarDeclKind::*;

    let declkind = if body.is_some() {
      Definition
    } else {
      Declaration
    };

    let symbol = match self.environment.find(&name) {
      None => Symbol::new_ref(Symbol::new(
        qualified_type,
        storage,
        name.clone(),
        declkind,
      )),
      Some(prev_symbol_ref) => {
        let borrow = prev_symbol_ref.borrow();
        if !QualifiedType::compatible(&borrow.qualified_type, &qualified_type) {
          Err(
            IncompatibleType(
              name.clone(),
              borrow.qualified_type.clone(),
              qualified_type.clone(),
            )
            .into_with(Severity::Error)
            .into_with(span),
          )?;
        }

        let prev_declkind = borrow.declkind;

        // SAFETY: drop borrow before borrow_mut
        drop(borrow);

        match (prev_declkind, declkind) {
          (_, Declaration) | (Declaration, Definition) =>
            if QualifiedType::compatible(
              &prev_symbol_ref.borrow().qualified_type,
              &qualified_type,
            ) {
              // TODO: nasty exceptions w.r.t. array compatibility in function params,
              //       like `int f(int a[restrict 5])` vs `int f(int a[5])`,
              //       even with `int f(int a[*])` and `int f(int a[restrict])`
              let composite = QualifiedType::composite_unchecked(
                &prev_symbol_ref.borrow().qualified_type,
                &qualified_type,
              );
              let mut borrow_mut = prev_symbol_ref.borrow_mut();
              borrow_mut.qualified_type = composite;
              borrow_mut.declkind = VarDeclKind::merge(prev_declkind, declkind);
              prev_symbol_ref.clone()
            } else {
              Err(
                IncompatibleType(
                  name.clone(),
                  prev_symbol_ref.borrow().qualified_type.clone(),
                  qualified_type.clone(),
                )
                .into_with(Severity::Error)
                .into_with(span),
              )?
            },
          (Definition, Definition) => Err(
            FunctionAlreadyDefined(name.clone())
              .into_with(Severity::Error)
              .into_with(span),
          )?,
          (Tentative, _) | (_, Tentative) =>
            contract_violation!("function cannot be tentative"),
        }
      },
    };

    self.environment.declare_symbol(name, symbol.clone());

    let function =
      ad::Function::new(symbol, parameters, function_specifier, None, span);

    match body {
      Some(body) => match self.current_function {
        Some(_) => contract_violation!(
          "nested function definition is not allowed; 
          this should be handled in parser: current function {}, new function \
           {}
          
          Also: this may occur if the `current_function` is not properly \
           cleared 
          after an `Err` returned of the previous function definition analysis",
          self.current_function.as_ref().unwrap().symbol.borrow().name,
          function.symbol.borrow().name
        ),
        None => self.function_with_body(body, function),
      },
      None => Ok(function),
    }
  }

  fn function_with_body(
    &mut self,
    body: ps::Compound,
    function: ad::Function,
  ) -> DeclRes<ad::Function> {
    self.current_function = Some(function);

    self.environment.enter();

    self
      .current_function
      .as_ref()
      .shall_ok("shall have function")
      .parameters
      .iter()
      .for_each(|parameter| {
        // FIXME: hsould we insert unnamed parameters or not?
        if parameter.symbol.borrow().name.starts_with('<') {
          // unnamed parameter - do nothing currently
        } else {
          self.environment.declare_symbol(
            parameter.symbol.borrow().name.clone(),
            parameter.symbol.clone(),
          );
        }
      });

    let statements = self.statements(body.statements);

    self.environment.exit();

    self
      .current_function
      .as_mut()
      .shall_ok("impossible; no current function?")
      .body = Some(astmt::Compound::new(statements, body.span));
    // verify labels and gotos
    let function =
      std::mem::take(&mut self.current_function).expect("never fails");

    function.gotos.iter().for_each(|goto| {
      if !function.labels.contains(goto) {
        contract_violation!(
          "goto label '{}' not found; this should be handled in parser",
          goto
        );
      }
    });
    Ok(function)
  }

  pub fn vardef(&mut self, vardef: pd::VarDef) -> DeclRes<ad::VarDef> {
    let pd::VarDef {
      declarator,
      declspecs,
      initializer,
      span,
    } = vardef;
    let (function_specifier, storage, qualified_type) =
      self.parse_declspecs(declspecs).shall_ok("vardef");
    contract_assert!(
      function_specifier.is_empty(),
      "variable cannot have function specifier; this should be handled in \
       parser"
    );
    let pd::Declarator {
      modifiers,
      name,
      span: _,
    } = declarator;
    let name = name
      .shall_ok("variable must have a name; it should be handled in parser");
    let qualified_type =
      self.apply_modifiers_for_varty(qualified_type, modifiers);
    let initializer = initializer.and_then(|initializer| {
      self.initializer(
        initializer,
        &qualified_type,
        self.environment.is_global()
          || matches!(
            storage,
            Some(Storage::Constexpr | Storage::Static | Storage::ThreadLocal)
          ),
      )
    });

    let vardef = match self.environment.is_global() {
      true => self.global_vardef(
        storage,
        qualified_type,
        name.clone(),
        initializer,
        span,
      ),
      false => self.local_vardef(
        storage.unwrap_or(Storage::Automatic),
        qualified_type,
        name.clone(),
        initializer,
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
    if let Some(prev_symbol_ref) = self.environment.shallow_find(&name) {
      if !QualifiedType::compatible(
        &prev_symbol_ref.borrow().qualified_type,
        &vardef.symbol.borrow().qualified_type,
      ) {
        Err(
          IncompatibleType(
            name,
            prev_symbol_ref.borrow().qualified_type.clone(),
            vardef.symbol.borrow().qualified_type.clone(),
          )
          .into_with(Severity::Error)
          .into_with(span),
        )?
      }
      let prev_declkind = prev_symbol_ref.borrow().declkind;
      let new_declkind = vardef.symbol.borrow().declkind;
      #[allow(clippy::upper_case_acronyms)]
      type VDK = VarDeclKind;
      match (&prev_declkind, &new_declkind) {
        (VDK::Definition, VDK::Definition) => Err(
          VariableAlreadyDefined(vardef.symbol.borrow().name.clone())
            .into_with(Severity::Error)
            .into_with(span),
        ),
        (VDK::Definition, VDK::Declaration)
        | (VDK::Definition, VDK::Tentative) => {
          // valid and nothing to do
          Ok(vardef)
        },
        (VDK::Declaration, VDK::Definition)
        | (VDK::Tentative, VDK::Definition) => {
          {
            let mut prev = prev_symbol_ref.borrow_mut();
            let new_symbol = vardef.symbol.borrow();
            prev.declkind = VDK::Definition;
            prev.storage_class = Storage::try_merge(
              &prev.storage_class,
              &new_symbol.storage_class,
            )
            .unwrap_or_else(|error| {
              self.add_diag(error.into_with(span));
              prev.storage_class
            });
            prev.qualified_type = QualifiedType::composite_unchecked(
              &new_symbol.qualified_type,
              &prev.qualified_type,
            );

            // dropped prev and new_symbol here
          }

          Ok(vardef)
        },
        (VDK::Declaration, VDK::Declaration)
        | (VDK::Tentative, VDK::Tentative)
        | (VDK::Declaration, VDK::Tentative)
        | (VDK::Tentative, VDK::Declaration) => {
          // only merge storage class if needed, todo
          Ok(vardef)
        },
      }
    } else {
      self.environment.declare_symbol(name, vardef.symbol.clone());
      Ok(vardef)
    }
  }

  fn initializer(
    &self,
    initializer: pd::Initializer,
    target_type: &QualifiedType,
    requires_folding: bool,
  ) -> Option<ad::Initializer> {
    match initializer {
      pd::Initializer::Expression(expression) => self
        .expression(*expression)
        .map(|expr| {
          expr
            .assignment_conversion(target_type)
            .handle_or_default(self)
        })
        .map(|expr| {
          Some(ad::Initializer::Scalar(if !requires_folding {
            expr
          } else {
            expr
              .fold(&self.session.diagnosis)
              .inspect_error(|e| {
                self.add_error(
                  ExprNotConstant(format!(
                    "Expression {e} cannot be evaluated to a constant value"
                  )),
                  e.span(),
                );
              })
              .unwrap()
          }))
        })
        .unwrap_or_else(|e| {
          self.add_diag(e);
          None
        }),
      pd::Initializer::List(_) => {
        not_implemented_feature!("initializer list");
      },
    }
  }

  fn global_vardef(
    &self,
    storage: Option<Storage>,
    qualified_type: QualifiedType,
    name: String,
    initializer: Option<ad::Initializer>,
    span: SourceSpan,
  ) -> DeclRes<ad::VarDef> {
    Ok(match (storage, initializer) {
      (None, None) => {
        let symbol = Symbol::tentative(qualified_type, Storage::Extern, name);
        ad::VarDef::new(symbol, None, span)
      },
      (None, Some(initializer)) => {
        let symbol = Symbol::def(qualified_type, Storage::Extern, name);
        ad::VarDef::new(symbol, Some(initializer), span)
      },
      (Some(storage), None) => {
        let symbol = Symbol::decl(qualified_type, storage, name);
        ad::VarDef::new(symbol, None, span)
      },
      (Some(storage), Some(initializer)) => {
        if storage == Storage::Extern {
          self.add_warning(ExternVariableWithInitializer(name.clone()), span);
        }
        let symbol = Symbol::def(qualified_type, storage, name);
        ad::VarDef::new(symbol, Some(initializer), span)
      },
    })
  }

  fn local_vardef(
    &self,
    storage: Storage,
    qualified_type: QualifiedType,
    name: String,
    initializer: Option<ad::Initializer>,
    span: SourceSpan,
  ) -> DeclRes<ad::VarDef> {
    if storage == Storage::Extern && initializer.is_some() {
      self.add_error(LocalExternVarWithInitializer(name.clone()), span);
    }
    let symbol = Symbol::decl(qualified_type, storage, name);
    Ok(ad::VarDef::new(symbol, initializer, span))
  }
}

impl<'session> Analyzer<'session> {
  fn expression(&self, expression: pe::Expression) -> ExprRes {
    match expression {
      pe::Expression::Empty(_) => Ok(Default::default()),
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

  fn sizeof(&self, sizeof: pe::SizeOf) -> ExprRes {
    match sizeof.sizeof {
      pe::SizeOfKind::Expression(expression) => {
        let analyzed_expr = self.expression(*expression).handle_with(
          self,
          ae::Expression::new_error_node(Primitive::ULongLong.into()),
        );
        let size = analyzed_expr.unqualified_type().size();
        Ok(ae::Expression::new_rvalue(
          ae::RawExpr::Constant(
            ae::ConstantLiteral::Integral(size.into()).into_with(SourceSpan {
              end: analyzed_expr.span().end,
              ..sizeof.span
            }),
          ),
          Primitive::ULongLong.into(),
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
        Ok(ae::Expression::new_rvalue(
          ae::RawExpr::Constant(
            ae::ConstantLiteral::Integral(qualified_type.size().into())
              .into_with(sizeof.span),
          ),
          Primitive::ULongLong.into(),
        ))
      },
    }
  }

  fn call(&self, call: pe::Call) -> ExprRes {
    let pe::Call {
      arguments,
      callee,
      span,
    } = call;
    let analyzed_callee = self.expression(*callee)?;

    let function_proto = match analyzed_callee.unqualified_type() {
      Type::FunctionProto(proto) => proto,
      Type::Pointer(ptr) => match ptr.pointee.unqualified_type() {
        Type::FunctionProto(proto) => proto,
        _ => Err(
          InvalidCallee(ptr.pointee.clone())
            .into_with(Severity::Error)
            .into_with(span),
        )?,
      },
      _ => Err(
        InvalidCallee(analyzed_callee.qualified_type().clone())
          .into_with(Severity::Error)
          .into_with(span),
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
    let expr_type = function_proto.return_type.clone();
    // todo: type promotion, currently just match the exact/compatible types
    Ok(ae::Expression::new_rvalue(
      ae::Call::new(analyzed_callee, analyzed_arguments, span).into(),
      expr_type,
    ))
  }

  fn paren(&self, paren: pe::Paren) -> ExprRes {
    let pe::Paren { expr, span } = paren;
    let analyzed_expr = self.expression(*expr)?;
    let expr_type = analyzed_expr.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::Paren::new(analyzed_expr, span).into(),
      expr_type,
    ))
  }

  fn cast(&self, _: pe::CStyleCast) -> ExprRes {
    not_implemented_feature!("C-style cast is not implemented yet");
  }

  fn variable(&self, variable: pe::Variable) -> ExprRes {
    let symbol = self.environment.find(&variable.name).ok_or(
      UndefinedVariable(variable.name.clone())
        .into_with(Severity::Error)
        .into_with(variable.span),
    )?;
    if symbol.borrow().is_typedef() {
      contract_violation!(
        "variable cannot be a typedef; this should be handled in parser"
      );
    } else {
      Ok(ae::Expression::new_lvalue(
        ae::Variable::new(symbol.clone(), variable.span).into(),
        symbol.borrow().qualified_type.clone(),
      ))
    }
  }

  fn constant(&self, constant: pe::Constant) -> ExprRes {
    let pe::Constant {
      value: constant,
      span,
    } = constant;
    let unqualified_type = constant.unqualified_type();
    let value_category = if constant.is_char_array() {
      ae::ValueCategory::LValue
    } else {
      ae::ValueCategory::RValue
    };
    Ok(ae::Expression::new(
      ae::Constant::new(constant, span).into(),
      unqualified_type.into(),
      value_category,
    ))
  }

  fn unary(&self, unary: pe::Unary) -> ExprRes {
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

  fn binary(&self, binary: pe::Binary) -> ExprRes {
    let pe::Binary {
      left: pe_left,
      operator,
      right: pe_right,
      span,
    } = binary;
    let left = self.expression(*pe_left)?;
    let right = self.expression(*pe_right)?;
    use OperatorCategory::*;
    match operator.category() {
      Assignment => self.assignment(operator, left, right, span),
      Logical => self.logical(operator, left, right, span),
      Relational => self.relational(operator, left, right, span),
      Arithmetic => self.arithmetic(operator, left, right, span),
      Bitwise => self.bitwise(operator, left, right, span),
      BitShift => self.bitshift(operator, left, right, span),
      Special => self.comma(operator, left, right, span),
      Uncategorized => unreachable!("operator is not binary: {:#?}", operator),
    }
  }

  fn ternary(&self, ternary: pe::Ternary) -> ExprRes {
    let pe::Ternary {
      condition: pe_condition,
      then_expr: pe_then_expr,
      else_expr: pe_else_expr,
      span,
    } = ternary;
    let condition = self.expression(*pe_condition)?;
    let then_expr = self.expression(*pe_then_expr)?;
    let else_expr = self.expression(*pe_else_expr)?;

    match (then_expr.unqualified_type(), else_expr.unqualified_type()) {
      (Type::Primitive(Primitive::Void), Type::Primitive(Primitive::Void)) =>
        Ok(ae::Expression::new_rvalue(
          ae::Ternary::new(condition, then_expr, else_expr, span).into(),
          QualifiedType::void(),
        )),
      (Type::Primitive(Primitive::Void), _) => Ok(ae::Expression::new_rvalue(
        ae::Ternary::new(
          condition,
          then_expr,
          ae::Expression::void_conversion(else_expr),
          span,
        )
        .into(),
        QualifiedType::void(),
      )),
      (_, Type::Primitive(Primitive::Void)) => Ok(ae::Expression::new_rvalue(
        ae::Ternary::new(
          condition,
          ae::Expression::void_conversion(then_expr),
          else_expr,
          span,
        )
        .into(),
        QualifiedType::void(),
      )),
      // both arithmetic -> usual arithmetic conversion
      (left_type, right_type)
        if left_type.is_arithmetic() && right_type.is_arithmetic() =>
      {
        let (then_converted, else_converted, result_type) =
          ae::Expression::usual_arithmetic_conversion(then_expr, else_expr)?;
        Ok(ae::Expression::new_rvalue(
          ae::Ternary::new(condition, then_converted, else_converted, span)
            .into(),
          result_type,
        ))
      },
      // both pointer to compatible type -> composite type
      (Type::Pointer(left_ptr), Type::Pointer(right_ptr)) => {
        let left_pointee = &left_ptr.pointee;
        let right_pointee = &right_ptr.pointee;
        if QualifiedType::compatible(left_pointee, right_pointee) {
          let qualified_type =
            QualifiedType::composite_unchecked(left_pointee, right_pointee);
          let result_type = Pointer::new(qualified_type).into();
          Ok(ae::Expression::new_rvalue(
            ae::Ternary::new(condition, then_expr, else_expr, span).into(),
            result_type,
          ))
        } else {
          Err(
            IncompatiblePointerTypes(
              left_pointee.to_string(),
              right_pointee.to_string(),
            )
            .into_with(Severity::Error)
            .into_with(span),
          )
        }
      },
      _ => todo!(),
    }
  }

  fn member_access(&self, _member_access: pe::MemberAccess) -> ExprRes {
    todo!()
  }

  fn array_subscript(&self, array_subscript: pe::ArraySubscript) -> ExprRes {
    // a[i] = *(a + i)
    let pe::ArraySubscript {
      array: pe_array,
      index: pe_index,
      span,
    } = array_subscript;
    let analyzed_array =
      self.expression(*pe_array)?.lvalue_conversion().decay();
    let analyzed_index =
      self.expression(*pe_index)?.lvalue_conversion().decay();

    if !analyzed_index.unqualified_type().is_integer() {
      Err(
        NonIntegerSubscript(analyzed_index.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )?
    }

    let analyzed_index = analyzed_index.ptrdiff_conversion_unchecked();

    if let Type::Pointer(ptr) = analyzed_array.unqualified_type() {
      let elem_type = ptr.pointee.clone();
      Ok(ae::Expression::new_lvalue(
        // store the pointer(decayed array) and index here, not the array here... maybe a wrong idesa, idk for now.
        ae::ArraySubscript::new(analyzed_array, analyzed_index, span).into(),
        elem_type,
      ))
    } else {
      Err(
        DerefNonPtr(analyzed_array.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )
    }
  }

  fn compound_literal(
    &self,
    _compound_literal: pe::CompoundLiteral,
  ) -> ExprRes {
    todo!()
  }
}
impl<'session> Analyzer<'session> {
  /// unary arithmetic operators: `+`, `-`
  fn unary_arithmetic(
    &self,
    operator: Operator,
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert!(matches!(operator, Operator::Plus | Operator::Minus));
    let operand = operand.lvalue_conversion().decay();

    if !operand.unqualified_type().is_arithmetic() {
      Err(
        NonArithmeticInUnaryOp(operator, operand.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )
    } else {
      let converted_operand = operand.usual_arithmetic_conversion_unary()?;
      let expr_type = converted_operand.qualified_type().clone();
      Ok(ae::Expression::new_rvalue(
        ae::Unary::prefix(operator, converted_operand, span).into(),
        expr_type,
      ))
    }
  }

  /// i didnt came up with a better name...
  fn ppmm(
    &self,
    operator: Operator,
    operand: ae::Expression,
    kind: ae::UnaryKind,
    span: SourceSpan,
  ) -> ExprRes {
    assert!(matches!(
      operator,
      Operator::PlusPlus | Operator::MinusMinus
    ));
    let operand = operand.decay();
    if operand.value_category() != ae::ValueCategory::LValue {
      Err(
        ExprNotAssignable(operand.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )
    } else if !operand.unqualified_type().is_arithmetic() {
      Err(
        NonArithmeticInUnaryOp(operator, operand.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )
    } else {
      // checked version would assert and panic if the operand is lvalue.
      let converted_operand =
        operand.usual_arithmetic_conversion_unary_unchecked()?;
      let expr_type = converted_operand.qualified_type().clone();
      Ok(ae::Expression::new_rvalue(
        ae::Unary::new(operator, converted_operand, kind, span).into(),
        expr_type,
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
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(operator, Operator::Tilde);
    let operand = operand.lvalue_conversion().decay();

    if !operand.unqualified_type().is_integer() {
      Err(
        NonIntegerInBitwiseUnaryOp(operator, operand.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )
    } else {
      let converted_operand = operand.usual_arithmetic_conversion_unary()?;
      let expr_type = converted_operand.qualified_type().clone();
      Ok(ae::Expression::new_rvalue(
        ae::Unary::prefix(operator, converted_operand, span).into(),
        expr_type,
      ))
    }
  }

  /// logical NOT operator `!`
  fn logical_not(
    &self,
    operator: Operator,
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(operator, Operator::Not);
    let operand = operand.lvalue_conversion().decay();

    let converted_operand = operand.conditional_conversion()?;
    Ok(ae::Expression::new_rvalue(
      ae::Unary::prefix(operator, converted_operand, span).into(),
      QualifiedType::bool_type(),
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
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(operator, Operator::Ampersand);
    if !operand.is_lvalue() {
      Err(
        AddressofOperandNotLvalue(operand.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )
    } else {
      let pointee = operand.qualified_type().clone();
      Ok(ae::Expression::new_rvalue(
        ae::Unary::prefix(operator, operand, span).into(),
        Pointer::new(pointee).into(),
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
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(operator, Operator::Star);

    let operand = operand.lvalue_conversion().decay();

    if !operand.unqualified_type().is_pointer() {
      Err(
        DerefNonPtr(operand.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )?
    }

    let pointee_type =
      &operand.unqualified_type().as_pointer_unchecked().pointee;
    if pointee_type.unqualified_type() == &Type::Primitive(Primitive::Void) {
      Err(
        DerefVoidPtr(operand.to_string())
          .into_with(Severity::Error)
          .into_with(span),
      )
    } else {
      // If the operand points to a function, the result is a function designator; -- which means the we don't need to perform decay here
      // if it points to an object, the result is an lvalue designating the object.
      // If the operand has type "pointer to type", the result has type "type".
      // If an invalid value has been assigned to the pointer, the behavior is undefined.
      let expr_type = pointee_type.clone();
      Ok(ae::Expression::new_lvalue(
        ae::Unary::prefix(operator, operand, span).into(),
        expr_type,
      ))
    }
  }
}
impl<'session> Analyzer<'session> {
  /// assignment operator `=`
  fn assignment(
    &self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    if !left.is_modifiable_lvalue() {
      self.add_error(ExprNotAssignable(left.to_string()), span);
      return Ok(left);
    }
    let assigned_expr = right
      .lvalue_conversion()
      .decay()
      .assignment_conversion(left.qualified_type())?;
    let expr_type = left.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::Binary::new(operator, left, assigned_expr, span).into(),
      expr_type,
    ))
  }

  /// logical operators: `&&`, `||`
  ///
  /// 1. lvalue conversion
  /// 2. decay
  /// 3. conditional conversion
  fn logical(
    &self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    let lhs = left.conditional_conversion()?;
    let rhs = right.conditional_conversion()?;
    Ok(ae::Expression::new_rvalue(
      ae::Binary::new(operator, lhs, rhs, span).into(),
      QualifiedType::bool_type(), // todo: this should be an `int` according to standard(?)
    ))
  }

  /// relational operators: `<`, `>`, `<=`, `>=`, `==`, `!=`
  ///
  /// same as `logical`, but with arithmetic conversions if both operands are arithmetic types
  fn relational(
    &self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    // Path A
    if left.unqualified_type().is_arithmetic()
      && right.unqualified_type().is_arithmetic()
    {
      let (lhs, rhs, _common_type) =
        ae::Expression::usual_arithmetic_conversion(left, right)?;

      return Ok(ae::Expression::new_rvalue(
        ae::Binary::new(operator, lhs, rhs, span).into(),
        QualifiedType::bool_type(), // ditto
      ));
    }
    todo!()
  }

  fn arithmetic(
    &self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    match (left.unqualified_type(), right.unqualified_type()) {
      (l, r) if l.is_pointer() || r.is_pointer() =>
        self.pointer_arithematic(operator, left, right, span),
      (l, r) if l.is_arithmetic() && r.is_arithmetic() =>
        self.usual_arithmetic(operator, left, right, span),
      // todo: enum constant..
      _ => Err(
        NonArithmeticInBinaryOp(left.to_string(), right.to_string(), operator)
          .into_with(Severity::Error)
          .into_with(span),
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
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    debug_assert!(
      left.unqualified_type().is_arithmetic()
        && right.unqualified_type().is_arithmetic()
    );

    let (lhs, rhs, result_type) =
      ae::Expression::usual_arithmetic_conversion(left, right)?;

    Ok(ae::Expression::new_rvalue(
      ae::Binary::new(operator, lhs, rhs, span).into(),
      result_type,
    ))
  }

  fn pointer_arithematic(
    &self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    debug_assert!(
      left.unqualified_type().is_pointer()
        || right.unqualified_type().is_pointer()
    );
    match (left.unqualified_type(), right.unqualified_type()) {
      // ptr - ptr -> intptr_t
      (Type::Pointer(left_ptr), Type::Pointer(right_ptr))
        if operator == Operator::Minus =>
        match QualifiedType::compatible(&left_ptr.pointee, &right_ptr.pointee) {
          true => Ok(ae::Expression::new_rvalue(
            ae::Binary::new(operator, left, right, span).into(),
            QualifiedType::ptrdiff(), // no qual for pointer difference
          )),
          false => Err(
            IncompatiblePointerTypes(
              left_ptr.pointee.to_string(),
              right_ptr.pointee.to_string(),
            )
            .into_with(Severity::Error)
            .into_with(span),
          ),
        },
      // int + ptr => ptr
      (Type::Primitive(lhs), Type::Pointer(ptr))
        if lhs.is_integer() && operator == Operator::Plus =>
      {
        let ptrty = right.unqualified_type().clone();
        Ok(ae::Expression::new_rvalue(
          ae::Binary::new(
            operator,
            left.ptrdiff_conversion_unchecked(),
            right,
            span,
          )
          .into(),
          ptrty.into(),
        ))
      },
      // ptr + int, ptr - int => ptr
      (Type::Pointer(ptr), Type::Primitive(rhs))
        if rhs.is_integer()
          && matches!(operator, Operator::Plus | Operator::Minus) =>
      {
        let ptrty = left.unqualified_type().clone();
        Ok(ae::Expression::new_rvalue(
          ae::Binary::new(
            operator,
            left,
            right.ptrdiff_conversion_unchecked(),
            span,
          )
          .into(),
          ptrty.into(),
        ))
      },
      // relops
      (Type::Pointer(_), Type::Pointer(_))
        if matches!(
          operator.category(),
          OperatorCategory::Logical | OperatorCategory::Relational
        ) =>
        Ok(ae::Expression::new_rvalue(
          ae::Binary::new(operator, left, right, span).into(),
          QualifiedType::bool_type(),
        )),
      _ => Err(
        InvalidOprand(
          left.qualified_type().clone(),
          right.qualified_type().clone(),
          operator,
        )
        .into_with(Severity::Error)
        .into_with(span),
      ),
    }
  }

  /// bitwise operators: `&`, `|`, `^`
  ///
  /// mostly same as arithmetic, but only for integer types
  fn bitwise(
    &self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    if !left.unqualified_type().is_integer()
      || !right.unqualified_type().is_integer()
    {
      self.add_error(
        NonIntegerInBitwiseBinaryOp(
          left.to_string(),
          right.to_string(),
          operator,
        ),
        span,
      );
    }

    let (lhs, rhs, result_type) =
      ae::Expression::usual_arithmetic_conversion(left, right)?;

    Ok(ae::Expression::new_rvalue(
      ae::Binary::new(operator, lhs, rhs, span).into(),
      result_type,
    ))
  }

  /// bitshift operators: `<<`, `>>`
  ///
  /// lvalue conversion, decay, promote, both operands must be integer types, but no usual arithmetic conversion
  fn bitshift(
    &self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    let lhs = left.lvalue_conversion().decay().promote();
    let rhs = right.lvalue_conversion().decay().promote();

    if !lhs.unqualified_type().is_integer()
      || !rhs.unqualified_type().is_integer()
    {
      Err(
        NonIntegerInBitshiftOp(lhs.to_string(), rhs.to_string(), operator)
          .into_with(Severity::Error)
          .into_with(span),
      )?
    }

    let expr_type = lhs.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::Binary::new(operator, lhs, rhs, span).into(),
      expr_type,
    ))
  }

  /// comma operator `,`
  ///
  /// left is void converted, result is right expression
  fn comma(
    &self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    // the result is the right expression, and the left is void converted, that's it. done.
    let expr_type = right.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::Binary::new(operator, left /* .void_conversion()*/, right, span)
        .into(),
      expr_type,
    ))
  }
}
impl<'session> Analyzer<'session> {
  fn statements(
    &mut self,
    statements: Vec<ps::Statement>,
  ) -> Vec<astmt::Statement> {
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
    statement: ps::Statement,
  ) -> StmtRes<astmt::Statement> {
    match statement {
      ps::Statement::Expression(expression) => self.exprstmt(expression),
      ps::Statement::Compound(compound_stmt) =>
        self.compound(compound_stmt).map(Into::into),
      ps::Statement::Empty(_) => Ok(astmt::Statement::default()),
      ps::Statement::Return(return_stmt) =>
        self.returnstmt(return_stmt).map(Into::into),
      ps::Statement::Declaration(declaration) =>
        self.declarations(declaration).map(Into::into),
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
  }

  #[inline]
  fn compound(&mut self, compound: ps::Compound) -> StmtRes<astmt::Compound> {
    self.compound_with(compound, |_| {})
  }

  fn compound_with<Fn>(
    &mut self,
    compound: ps::Compound,
    callback: Fn,
  ) -> StmtRes<astmt::Compound>
  where
    Fn: FnOnce(&Self),
  {
    self.environment.enter();

    callback(self);

    let statements = self.statements(compound.statements);

    self.environment.exit();

    Ok(astmt::Compound::new(statements, compound.span))
  }

  fn exprstmt(&self, expr_stmt: pe::Expression) -> StmtRes<astmt::Statement> {
    // todo: unused expression result warning
    Ok(self.expression(expr_stmt)?.into())
  }

  fn returnstmt(&self, return_stmt: ps::Return) -> StmtRes<astmt::Return> {
    let ps::Return { expression, span } = return_stmt;
    let analyzed_expr = match expression {
      Some(expr) => Some(self.expression(expr)?),
      None => None,
    };

    let return_type = match &self
      .current_function
      .as_ref()
      .shall_ok("return statement outside function should be handled in parser")
      .symbol
      .borrow()
      .qualified_type
      .unqualified_type()
    {
      Type::FunctionProto(proto) => proto.return_type.clone(),
      _ => {
        contract_violation!("current function's type is not function proto")
      },
    };
    match (&analyzed_expr, return_type.unqualified_type()) {
      (None, Type::Primitive(Primitive::Void)) =>
        Ok(astmt::Return::new(None, span)),
      (None, _) => Err(
        ReturnTypeMismatch("non-void function must return a value".to_string())
          .into_with(Severity::Error)
          .into_with(span),
      ),

      (Some(_), Type::Primitive(Primitive::Void)) => Err(
        ReturnTypeMismatch("void function cannot return a value".to_string())
          .into_with(Severity::Error)
          .into_with(span),
      ),

      (Some(_), _) => {
        let a = unsafe {
          // this has value for ABSOLUTELY sure
          analyzed_expr.unwrap_unchecked()
        }
        .lvalue_conversion()
        .decay()
        .assignment_conversion(&return_type)?;
        Ok(astmt::Return::new(Some(a), span))
      },
    }
  }

  fn ifstmt(&mut self, if_stmt: ps::If) -> StmtRes<astmt::If> {
    let ps::If {
      condition,
      then_branch,
      else_branch,
      span,
    } = if_stmt;
    let analyzed_condition = self
      .expression(condition)
      .and_then(|e| e.conditional_conversion())
      .handle_with(
        self,
        ae::Expression::new_error_node(QualifiedType::bool_type()),
      );
    let analyzed_then_branch =
      self.statement(*then_branch).handle_or_default(self).into();
    let analyzed_else_branch = else_branch.map(|else_branch| {
      self.statement(*else_branch).handle_or_default(self).into()
    });
    Ok(astmt::If::new(
      analyzed_condition,
      analyzed_then_branch,
      analyzed_else_branch,
      span,
    ))
  }

  fn whilestmt(&mut self, while_stmt: ps::While) -> StmtRes<astmt::While> {
    let ps::While {
      condition,
      body,
      tag: label,
      span,
    } = while_stmt;
    let analyzed_condition = self
      .expression(condition)
      .and_then(|e| e.conditional_conversion())
      .handle_with(
        self,
        ae::Expression::new_error_node(QualifiedType::bool_type()),
      );
    let analyzed_body = self.statement(*body).handle_or_default(self).into();
    Ok(astmt::While::new(
      analyzed_condition,
      analyzed_body,
      label,
      span,
    ))
  }

  fn dowhilestmt(&mut self, do_while: ps::DoWhile) -> StmtRes<astmt::DoWhile> {
    let ps::DoWhile {
      body,
      condition,
      tag: label,
      span,
    } = do_while;
    let analyzed_body = self.statement(*body).handle_or_default(self).into();
    let analyzed_condition = self
      .expression(condition)
      .and_then(|e| e.conditional_conversion())
      .handle_with(
        self,
        ae::Expression::new_error_node(QualifiedType::bool_type()),
      );
    Ok(astmt::DoWhile::new(
      analyzed_body,
      analyzed_condition,
      label,
      span,
    ))
  }

  fn forstmt(&mut self, for_stmt: ps::For) -> StmtRes<astmt::For> {
    let ps::For {
      initializer,
      condition,
      increment,
      body,
      tag: label,
      span,
    } = for_stmt;
    let analyzed_initializer = initializer
      .map(|init| self.statement(*init).handle_or_default(self).into());
    let analyzed_condition = condition.map(|cond| {
      self.expression(cond).handle_with(
        self,
        ae::Expression::new_error_node(QualifiedType::bool_type()),
      )
    });
    let analyzed_increment =
      increment.map(|inc| self.expression(inc).handle_or_default(self));
    let analyzed_body = self.statement(*body).handle_or_default(self).into();
    Ok(astmt::For::new(
      analyzed_initializer,
      analyzed_condition,
      analyzed_increment,
      analyzed_body,
      label,
      span,
    ))
  }

  fn switchstmt(&mut self, switch: ps::Switch) -> StmtRes<astmt::Switch> {
    let ps::Switch {
      cases,
      condition,
      default,
      tag,
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
        ae::Expression::new_error_node(QualifiedType::int())
      },
    };
    let analyzed_cases = cases
      .into_iter()
      .map(|case| self.casestmt(case).shall_ok("switch case"))
      .collect::<Vec<_>>();

    let analyzed_default = default
      .map(|default| self.defaultstmt(default).shall_ok("switch default"));
    Ok(astmt::Switch::new(
      analyzed_condition,
      analyzed_cases,
      analyzed_default,
      tag,
      span,
    ))
  }

  fn casestmt(&mut self, case: ps::Case) -> StmtRes<astmt::Case> {
    let ps::Case { body, value, span } = case;
    let analyzed_value = self
      .expression(value)
      .handle_with(self, ae::Expression::new_error_node(QualifiedType::int()));
    let analyzed_body = self.statements(body);

    Ok(astmt::Case::new(
      analyzed_value
        .fold(&self.session.diagnosis)
        .transform(|expr| {
          if let ae::RawExpr::Constant(constant) = expr.raw_expr() {
            if constant.is_integral() {
              constant.value.clone()
            } else {
              self.add_error(
                NonIntegerInCaseStmt(constant.value.clone()),
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

  fn defaultstmt(&mut self, default: ps::Default) -> StmtRes<astmt::Default> {
    let ps::Default { body, span } = default;
    let analyzed_body = self.statements(body);
    Ok(astmt::Default::new(analyzed_body, span))
  }

  fn labelstmt(&mut self, label: ps::Label) -> StmtRes<astmt::Label> {
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
        match self
          .current_function
          .as_mut()
          .unwrap()
          .labels
          .insert(name.clone())
        {
          true => Ok(astmt::Label::new(
            name,
            self.statement(*statement).handle_or_default(self),
            span,
          )),
          false => Err(
            DuplicateLabel(name)
              .into_with(Severity::Error)
              .into_with(span),
          ),
        }
      },
    }
  }

  fn gotostmt(&mut self, goto: ps::Goto) -> StmtRes<astmt::Goto> {
    match self.environment.is_global() {
      true => contract_violation!(
        "goto statement in global scope should be handled in parser"
      ),
      false => {
        self
          .current_function
          .as_mut()
          .unwrap()
          .gotos
          .insert(goto.label.clone());
        Ok(astmt::Goto::new(goto.label, goto.span))
      },
    }
  }

  fn breakstmt(&self, break_stmt: ps::Break) -> StmtRes<astmt::Break> {
    match self.environment.is_global() {
      true => contract_violation!(
        "break statement in global scope should be handled in parser"
      ),
      false => Ok(astmt::Break::new(break_stmt.tag, break_stmt.span)),
    }
  }

  fn continuestmt(
    &self,
    continue_stmt: ps::Continue,
  ) -> StmtRes<astmt::Continue> {
    match self.environment.is_global() {
      true => contract_violation!(
        "continue statement in global scope should be handled in parser"
      ),
      false => Ok(astmt::Continue::new(continue_stmt.tag, continue_stmt.span)),
    }
  }
}

mod test {
  #[test]
  fn oneplusone() {
    use super::*;
    let session = Session::no_manager();
    // 1 + 1
    let analyzer = Analyzer::new(Default::default(), &session);
    let expr = pe::Expression::oneplusone();
    let analyzed_expr = analyzer.expression(expr);

    assert!(analyzed_expr.is_ok());
    println!("{:#?}", dbg!(analyzed_expr.unwrap()));
  }
}
