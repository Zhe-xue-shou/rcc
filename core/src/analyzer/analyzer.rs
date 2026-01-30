use ::rc_utils::{
  Dummy, IntoWith, contract_assert, contract_violation, not_implemented_feature,
};

use crate::{
  analyzer::{declaration as ad, expression as ae, statement as astmt},
  common::{
    Environment, Operator, OperatorCategory, SourceSpan, Storage, Symbol,
    VarDeclKind,
  },
  diagnosis::{Error, ErrorData::*, Warning, WarningData::*},
  parser::{declaration as pd, expression as pe, statement as ps},
  types::{
    Array, ArraySize, Compatibility, FunctionProto, FunctionSpecifier, Pointer,
    Primitive, QualifiedType, Type, TypeInfo,
  },
};

type TypeRes = Result<QualifiedType, Error>;
type ExprRes = Result<ae::Expression, Error>;
type DeclRes<T> = Result<T, Error>;
type StmtRes<T> = Result<T, Error>;

#[cold]
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

impl<T> ImplHelper<T> for Result<T, Error> {
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
#[allow(unused)]
trait ImplHelper2<T, Listener> {
  fn handle_with(self, context: &mut Listener, default: T) -> T;
}

impl<T> ImplHelper2<T, Analyzer> for Result<T, Error> {
  /// if it's error, log it, and return a default value (means error)
  fn handle_with(self, context: &mut Analyzer, default: T) -> T {
    match self {
      Ok(t) => t,
      Err(e) => {
        context.add_error(e);
        default
      },
    }
  }
}

#[allow(unused)]
trait ImplHelper3<T, Listener> {
  fn handle_or_dummy(self, context: &mut Listener) -> T;
}

impl<T: Dummy> ImplHelper3<T, Analyzer> for Result<T, Error> {
  fn handle_or_dummy(self, context: &mut Analyzer) -> T {
    match self {
      Ok(t) => t,
      Err(e) => {
        context.add_error(e);
        Dummy::dummy()
      },
    }
  }
}

#[derive(Debug, Default)]
pub struct Analyzer {
  program: pd::Program,
  environment: Environment,
  current_function: Option<ad::Function>,
  errors: Vec<Error>,
  warnings: Vec<Warning>,
}

impl Analyzer {
  pub fn new(program: pd::Program) -> Self {
    Self {
      program,
      ..Analyzer::default()
    }
  }

  pub fn add_error(&mut self, error: Error) {
    self.errors.push(error);
  }

  pub fn add_warning(&mut self, warning: Warning) {
    self.warnings.push(warning);
  }

  pub fn analyze(&mut self) -> ad::TranslationUnit {
    self.environment.enter();
    let translation_unit = ad::TranslationUnit::new(self.externaldecl());
    self.environment.exit();
    translation_unit
  }

  pub fn errors(&self) -> &[Error] {
    &self.errors
  }

  pub fn warnings(&self) -> &[Warning] {
    &self.warnings
  }

  pub fn unnamed_placeholder() -> String {
    static COUNTER: ::std::sync::atomic::AtomicUsize =
      ::std::sync::atomic::AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
    format!("<unnamed_{}>", id)
  }
}
impl Analyzer {
  fn apply_modifiers_for_varty(
    &mut self,
    mut qualified_type: QualifiedType,
    modifiers: Vec<pd::Modifier>,
  ) -> QualifiedType {
    // reverse order
    for modifier in modifiers.into_iter().rev() {
      match modifier {
        pd::Modifier::Pointer(qualifiers) => {
          qualified_type = QualifiedType::new(
            qualifiers,
            Pointer::new(qualified_type.into()).into(),
          );
        },
        pd::Modifier::Array(arraymodifier) => {
          let size = match arraymodifier.bound {
            pd::ArrayBound::Constant(n) => ArraySize::Constant(n),
            pd::ArrayBound::Incomplete => ArraySize::Incomplete,
            pd::ArrayBound::Variable(_) => ArraySize::Variable,
          };
          qualified_type = Array {
            element_type: qualified_type.into(),
            size,
          }
          .into();
        },
        pd::Modifier::Function(functionsignature) => {
          // func ptr or so
          let pd::FunctionSignature {
            parameters,
            is_variadic,
          } = functionsignature;
          let analyzed_parameter_types = self.parse_parameter_types(parameters);
          qualified_type = FunctionProto::new(
            qualified_type.into(),
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
    &mut self,
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
      .collect::<Vec<QualifiedType>>();
    let functionproto =
      FunctionProto::new(return_type.into(), parameter_types, is_variadic);

    Ok((functionproto.into(), parameters))
  }

  fn parse_parameter_types(
    &mut self,
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
          "parameter cannot have storage class specifier; this should be handled in parser.
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
    &mut self,
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
          "parameter cannot have storage class specifier; this should be handled in parser.
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
          name.unwrap_or_else(|| Self::unnamed_placeholder()),
          VarDeclKind::Declaration,
        ));
        Ok(ad::Parameter::new(symbol, span))
      })
      .collect()
  }

  fn parse_declspecs(
    &mut self,
    declspecs: pd::DeclSpecs,
  ) -> Result<(FunctionSpecifier, Option<Storage>, QualifiedType), Error> {
    let qualified_type = self
      .get_type(declspecs.type_specifiers)
      .unwrap_or_else(|e| {
        self.add_error(e);
        QualifiedType::int()
      })
      .with_qualifiers(declspecs.qualifiers);
    let storage_class = declspecs.storage_class;
    let function_specifier = declspecs.function_specifiers;

    Ok((function_specifier, storage_class, qualified_type))
  }

  fn get_type(&self, mut type_specifiers: Vec<pd::TypeSpecifier>) -> TypeRes {
    assert!(!type_specifiers.is_empty());
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
      _ => not_implemented_feature!("union, struct, enum, typeof, etc."),
    }
  }
}

impl Analyzer {
  fn externaldecl(&mut self) -> Vec<ad::ExternalDeclaration> {
    let mut declarations = Vec::new();
    std::mem::take(&mut self.program)
      .declarations
      .into_iter()
      .for_each(|decl| match self.declarations(decl) {
        Ok(declaration) => declarations.push(declaration),
        Err(e) => self.add_error(e),
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
    let storage = storage.unwrap_or_else(|| Storage::Extern);
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
      FunctionProto::main_proto_validate(
        qualified_type
          .unqualified_type()
          .as_functionproto_unchecked(),
        function_specifier,
      )
      .unwrap_or_else(|e| {
        self.add_error(e.into_with(span));
      });
    }

    let symbol = Symbol::new_ref(Symbol::new(
      qualified_type,
      storage,
      name.clone(),
      if body.is_some() {
        VarDeclKind::Definition
      } else {
        VarDeclKind::Declaration
      },
    ));

    self.environment.declare_symbol(name, symbol.clone());

    let function =
      ad::Function::new(symbol, parameters, function_specifier, None, span);

    match body {
      Some(body) => match self.current_function {
        Some(_) => contract_violation!(
          "nested function definition is not allowed; 
          this should be handled in parser: current function {}, new function {}
          
          Also: this may occur if the `current_function` is not properly cleared 
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
      "variable cannot have function specifier; this should be handled in parser"
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
    let initializer = match initializer {
      Some(init) => match init {
        pd::Initializer::Expression(expression) => self
          .expression(*expression)
          .map(|expr| Some(ad::Initializer::Scalar(expr)))
          .unwrap_or(None),
        pd::Initializer::List(_) => {
          not_implemented_feature!("initializer list");
        },
      },
      None => None,
    };
    // todo: check initializer type compatibility

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
        return Err(
          IncompatibleType(
            name,
            prev_symbol_ref.borrow().qualified_type.clone(),
            vardef.symbol.borrow().qualified_type.clone(),
          )
          .into_with(span),
        );
      }
      let prev_declkind = prev_symbol_ref.borrow().declkind;
      let new_declkind = vardef.symbol.borrow().declkind;
      type VDK = VarDeclKind;
      match (&prev_declkind, &new_declkind) {
        (VDK::Definition, VDK::Definition) => Err(
          VariableAlreadyDefined(vardef.symbol.borrow().name.clone())
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
              self.add_error(error.into_with(span));
              prev.storage_class.clone()
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

  fn global_vardef(
    &mut self,
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
          self.add_warning(
            ExternVariableWithInitializer(name.clone()).into_with(span),
          );
        }
        let symbol = Symbol::def(qualified_type, storage, name);
        ad::VarDef::new(symbol, Some(initializer), span)
      },
    })
  }

  fn local_vardef(
    &mut self,
    storage: Storage,
    qualified_type: QualifiedType,
    name: String,
    initializer: Option<ad::Initializer>,
    span: SourceSpan,
  ) -> DeclRes<ad::VarDef> {
    if storage == Storage::Extern && initializer.is_some() {
      self
        .add_error(LocalExternVarWithInitializer(name.clone()).into_with(span));
    }
    let symbol = Symbol::decl(qualified_type, storage, name);
    Ok(ad::VarDef::new(symbol, initializer, span))
  }
}

impl Analyzer {
  fn expression(&mut self, expression: pe::Expression) -> ExprRes {
    match expression {
      pe::Expression::Empty => Ok(ae::Expression::default()),
      pe::Expression::Constant(constant) => self.constant(constant),
      pe::Expression::Unary(unary) => self.unary(unary),
      pe::Expression::Binary(binary) => self.binary(binary),
      pe::Expression::Variable(variable) => self.variable(variable),
      pe::Expression::Call(call) => self.call(call),
      pe::Expression::Paren(paren) => self.paren(paren),
      pe::Expression::Ternary(ternary) => self.ternary(ternary),
      pe::Expression::SizeOf(sizeof) => self.sizeof(sizeof),
      pe::Expression::CStyleCast(cast) => self.cast(cast),
      pe::Expression::MemberAccess(_) => not_implemented_feature!(),
      pe::Expression::ArraySubscript(_) => not_implemented_feature!(),
      pe::Expression::CompoundLiteral(_) => not_implemented_feature!(),
    }
  }

  fn sizeof(&mut self, sizeof: pe::SizeOf) -> ExprRes {
    match sizeof.sizeof {
      pe::SizeOfKind::Expression(expression) => {
        let analyzed_expr = self.expression(*expression).handle_with(
          self,
          ae::Expression::new_error_node(Primitive::ULongLong.into()),
        );
        let size = analyzed_expr.unqualified_type().size();
        Ok(ae::Expression::new_rvalue(
          ae::RawExpr::Constant(
            ae::ConstantLiteral::ULongLong(size as u64).into_with(SourceSpan {
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
        } = unprocessed_type;
        let qualified_type = {
          let (_, _, base_type) =
            self.parse_declspecs(declspecs).shall_ok("sizeof type");
          self.apply_modifiers_for_varty(base_type, declarator.modifiers)
        };
        Ok(ae::Expression::new_rvalue(
          ae::RawExpr::Constant(
            ae::ConstantLiteral::ULongLong(qualified_type.size() as u64)
              .into_with(sizeof.span),
          ),
          Primitive::ULongLong.into(),
        ))
      },
    }
  }

  fn call(&mut self, call: pe::Call) -> ExprRes {
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
        _ =>
          return Err(
            InvalidCallee(ptr.pointee.unqualified_type().to_string())
              .into_with(span),
          ),
      },
      _ =>
        return Err(
          InvalidCallee(analyzed_callee.unqualified_type().to_string())
            .into_with(span),
        ),
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
    let expr_type = function_proto.return_type.as_ref().clone();
    // todo: type promotion, currently just match the exact/compatible types
    Ok(ae::Expression::new_rvalue(
      ae::Call::new(analyzed_callee, analyzed_arguments, span).into(),
      expr_type,
    ))
  }

  fn paren(&mut self, paren: pe::Paren) -> ExprRes {
    let pe::Paren { expr, span } = paren;
    let analyzed_expr = self.expression(*expr)?;
    let expr_type = analyzed_expr.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::Paren::new(analyzed_expr, span).into(),
      expr_type,
    ))
  }

  fn cast(&mut self, _: pe::CStyleCast) -> ExprRes {
    not_implemented_feature!("C-style cast is not implemented yet");
  }

  fn variable(&mut self, variable: pe::Variable) -> ExprRes {
    let symbol = self.environment.find(&variable.name).ok_or(
      UndefinedVariable(variable.name.clone()).into_with(variable.span),
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

  fn constant(&mut self, constant: pe::Constant) -> ExprRes {
    let pe::Constant { constant, span } = constant;
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

  fn unary(&mut self, unary: pe::Unary) -> ExprRes {
    let pe::Unary {
      operator,
      operand: pe_expr,
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
      Operator::PlusPlus | Operator::MinusMinus => not_implemented_feature!(
        "Unary ++ and -- operators are not implemented yet"
      ),
      _ => unreachable!("operator is not unary: {:#?}", operator),
    }
  }

  fn binary(&mut self, binary: pe::Binary) -> ExprRes {
    let pe::Binary {
      left: pe_left,
      operator,
      right: pe_right,
      span,
    } = binary;
    let left = self.expression(*pe_left)?;
    let right = self.expression(*pe_right)?;
    match operator.category() {
      OperatorCategory::Assignment =>
        self.assignment(operator, left, right, span),
      OperatorCategory::Logical => self.logical(operator, left, right, span),
      OperatorCategory::Relational =>
        self.relational(operator, left, right, span),
      OperatorCategory::Arithmetic =>
        self.arithmetic(operator, left, right, span),
      OperatorCategory::Bitwise => self.bitwise(operator, left, right, span),
      OperatorCategory::BitShift => self.bitshift(operator, left, right, span),
      OperatorCategory::Comma => self.comma(operator, left, right, span),
    }
  }

  fn ternary(&mut self, ternary: pe::Ternary) -> ExprRes {
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
          let result_type = Pointer::new(qualified_type.into()).into();
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
            .into_with(span),
          )
        }
      },
      _ => todo!(),
    }
  }
}
impl Analyzer {
  /// unary arithmetic operators: `+`, `-`
  fn unary_arithmetic(
    &mut self,
    operator: Operator,
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert!(matches!(operator, Operator::Plus | Operator::Minus));
    let operand = operand.lvalue_conversion().decay();

    if !operand.unqualified_type().is_arithmetic() {
      Err(NonArithmeticInUnaryOp(operator, operand.to_string()).into_with(span))
    } else {
      let converted_operand = operand.usual_arithmetic_conversion_unary()?;
      let expr_type = converted_operand.qualified_type().clone();
      Ok(ae::Expression::new_rvalue(
        ae::Unary::new(operator, converted_operand, span).into(),
        expr_type,
      ))
    }
  }

  /// bitwise NOT operator `~`
  ///
  /// 6.5.4.3.4: The result of the ~ operator is the bitwise complement of its (promoted) operand.
  ///     The integer promotions are performed on the operand, and the result has the promoted type.
  fn tilde(
    &mut self,
    operator: Operator,
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(operator, Operator::Tilde);
    let operand = operand.lvalue_conversion().decay();

    if !operand.unqualified_type().is_integer() {
      Err(
        NonIntegerInBitwiseUnaryOp(operator, operand.to_string())
          .into_with(span),
      )
    } else {
      let converted_operand = operand.usual_arithmetic_conversion_unary()?;
      let expr_type = converted_operand.qualified_type().clone();
      Ok(ae::Expression::new_rvalue(
        ae::Unary::new(operator, converted_operand, span).into(),
        expr_type,
      ))
    }
  }

  /// logical NOT operator `!`
  fn logical_not(
    &mut self,
    operator: Operator,
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(operator, Operator::Not);
    let operand = operand.lvalue_conversion().decay();

    let converted_operand = operand.conditional_conversion()?;
    Ok(ae::Expression::new_rvalue(
      ae::Unary::new(operator, converted_operand, span).into(),
      QualifiedType::bool(),
    ))
  }

  /// address-of operator `&`
  ///
  /// no `lvalue_conversion`, no `decay`
  /// 6.5.4.2.3: The unary & operator yields the address of its operand.
  /// If the operand has type "type"(in my Type system it's represented as `QualifiedType`)
  fn addressof(
    &mut self,
    operator: Operator,
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(operator, Operator::Ampersand);
    if !operand.is_lvalue() {
      Err(AddressofOperandNotLvalue(operand.to_string()).into_with(span))
    } else {
      let pointee = operand.qualified_type().clone();
      Ok(ae::Expression::new_rvalue(
        ae::Unary::new(operator, operand, span).into(),
        Pointer::new(pointee.into()).into(),
      ))
    }
  }

  /// indirection operator `*`
  ///
  /// 6.5.4.2.4: The unary * operator denotes indirection.
  /// the pointee needs to `lvalue_conversion` and `decay`, but the result itself does not need to
  fn indirect(
    &mut self,
    operator: Operator,
    operand: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(operator, Operator::Star);

    let operand = operand.lvalue_conversion().decay();

    if !operand.unqualified_type().is_pointer() {
      return Err(DerefNonPtr(operand.to_string()).into_with(span));
    }

    let pointee_type =
      &operand.unqualified_type().as_pointer_unchecked().pointee;
    if pointee_type.unqualified_type() == &Type::Primitive(Primitive::Void) {
      Err(DerefVoidPtr(operand.to_string()).into_with(span))
    } else {
      // If the operand points to a function, the result is a function designator; -- which means the we don't need to perform decay here
      // if it points to an object, the result is an lvalue designating the object.
      // If the operand has type "pointer to type", the result has type "type".
      // If an invalid value has been assigned to the pointer, the behavior is undefined.
      let expr_type = pointee_type.as_ref().clone();
      Ok(ae::Expression::new_lvalue(
        ae::Unary::new(operator, operand, span).into(),
        expr_type,
      ))
    }
  }
}
impl Analyzer {
  /// assignment operator `=`
  fn assignment(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    assert_eq!(
      operator,
      Operator::Assign,
      "compound assignment(e.g. +=, /=, etc) not implemented"
    );
    if !left.is_modifiable_lvalue() {
      self.add_error(ExprNotAssignable(left.to_string()).into_with(span));
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
    &mut self,
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
      QualifiedType::bool(), // todo: this should be an `int` according to standard(?)
    ))
  }

  /// relational operators: `<`, `>`, `<=`, `>=`, `==`, `!=`
  ///
  /// same as `logical`, but with arithmetic conversions if both operands are arithmetic types
  fn relational(
    &mut self,
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
        QualifiedType::bool(), // ditto
      ));
    }
    todo!()
  }

  /// usual arithmetic conversion: `+`, `-`, `*`, `/`, `%`
  ///
  /// 1. lvalue conversion, with the exception of arrays and functionproto\(handled inside the `lvalue_conversion`\)
  /// 2. array and function decay
  /// 3. promotions\(inside `usual_arithmetic_conversion`\)
  /// 4. finally, the usual arithmetic conversion itself
  fn arithmetic(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    let (lhs, rhs, result_type) =
      ae::Expression::usual_arithmetic_conversion(left, right)?;

    Ok(ae::Expression::new_rvalue(
      ae::Binary::new(operator, lhs, rhs, span).into(),
      result_type,
    ))
  }

  /// bitwise operators: `&`, `|`, `^`
  ///
  /// mostly same as arithmetic, but only for integer types
  fn bitwise(
    &mut self,
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
          operator.clone(),
        )
        .into_with(span),
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
    &mut self,
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
      // return err_or_debugbreak!(); // error: bitshift operator requires integer operands
      return Err(
        NonIntegerInBitshiftOp(lhs.to_string(), rhs.to_string(), operator)
          .into_with(span),
      );
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
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
    span: SourceSpan,
  ) -> ExprRes {
    // the result is the right expression, and the left is void converted, that's it. done.
    let expr_type = right.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::Binary::new(operator, left.void_conversion(), right, span).into(),
      expr_type,
    ))
  }
}
impl Analyzer {
  fn statements(
    &mut self,
    statements: Vec<ps::Statement>,
  ) -> Vec<astmt::Statement> {
    statements
      .into_iter()
      .filter_map(|statement| match self.statement(statement) {
        Ok(stmt) => Some(stmt),
        Err(e) => {
          self.add_error(e);
          None
        },
      })
      .collect::<Vec<_>>()
  }

  fn statement(
    &mut self,
    statement: ps::Statement,
  ) -> StmtRes<astmt::Statement> {
    match statement {
      ps::Statement::Expression(expression) => self.exprstmt(expression),
      ps::Statement::Compound(compound_stmt) =>
        Ok(astmt::Statement::Compound(self.compound(compound_stmt)?)),
      ps::Statement::Empty() => Ok(astmt::Statement::Empty()),
      ps::Statement::Return(return_stmt) =>
        Ok(astmt::Statement::Return(self.returnstmt(return_stmt)?)),
      ps::Statement::Declaration(declaration) => Ok(
        astmt::Statement::Declaration(self.declarations(declaration)?),
      ),
      ps::Statement::If(if_stmt) =>
        Ok(astmt::Statement::If(self.ifstmt(if_stmt)?)),
      ps::Statement::While(while_stmt) =>
        Ok(astmt::Statement::While(self.whilestmt(while_stmt)?)),
      ps::Statement::DoWhile(do_while) =>
        Ok(astmt::Statement::DoWhile(self.dowhilestmt(do_while)?)),
      ps::Statement::For(for_stmt) =>
        Ok(astmt::Statement::For(self.forstmt(for_stmt)?)),
      ps::Statement::Label(label) =>
        Ok(astmt::Statement::Label(self.labelstmt(label)?)),
      ps::Statement::Switch(switch) =>
        Ok(astmt::Statement::Switch(self.switchstmt(switch)?)),
      ps::Statement::Goto(goto) => self.gotostmt(goto),
      ps::Statement::Break(break_stmt) => self.breakstmt(break_stmt),
      ps::Statement::Continue(continue_stmt) =>
        self.continuestmt(continue_stmt),
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
    Fn: FnOnce(&mut Self),
  {
    self.environment.enter();

    callback(self);

    let statements = self.statements(compound.statements);

    self.environment.exit();

    Ok(astmt::Compound::new(statements, compound.span))
  }

  fn exprstmt(
    &mut self,
    expr_stmt: pe::Expression,
  ) -> StmtRes<astmt::Statement> {
    // todo: unused expression result warning
    Ok(astmt::Statement::Expression(self.expression(expr_stmt)?))
  }

  fn returnstmt(&mut self, return_stmt: ps::Return) -> StmtRes<astmt::Return> {
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
      Type::FunctionProto(proto) => proto.return_type.as_ref().clone(),
      _ => {
        contract_violation!("current function's type is not function proto")
      },
    };
    match (&analyzed_expr, return_type.unqualified_type()) {
      (None, Type::Primitive(Primitive::Void)) =>
        Ok(astmt::Return::new(None, span)),
      (None, _) => Err(
        ReturnTypeMismatch("non-void function must return a value".to_string())
          .into_with(span),
      ),

      (Some(_), Type::Primitive(Primitive::Void)) => Err(
        ReturnTypeMismatch("void function cannot return a value".to_string())
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
      .handle_with(self, ae::Expression::new_error_node(QualifiedType::bool()));
    let analyzed_then_branch =
      self.statement(*then_branch).handle_or_dummy(self).into();
    let analyzed_else_branch = match else_branch {
      Some(else_branch) =>
        Some(self.statement(*else_branch).handle_or_dummy(self).into()),
      None => None,
    };
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
      .handle_with(self, ae::Expression::new_error_node(QualifiedType::bool()));
    let analyzed_body = self.statement(*body).handle_or_dummy(self).into();
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
    let analyzed_body = self.statement(*body).handle_or_dummy(self).into();
    let analyzed_condition = self
      .expression(condition)
      .and_then(|e| e.conditional_conversion())
      .handle_with(self, ae::Expression::new_error_node(QualifiedType::bool()));
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
      .map(|init| self.statement(*init).handle_or_dummy(self).into());
    let analyzed_condition = condition.map(|cond| {
      self.expression(cond).handle_with(
        self,
        ae::Expression::new_error_node(QualifiedType::bool()),
      )
    });
    let analyzed_increment =
      increment.map(|inc| self.expression(inc).handle_or_dummy(self));
    let analyzed_body = self.statement(*body).handle_or_dummy(self).into();
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
          ))
          .into_with(span),
        );
        val
      },
      Err(e) => {
        self.add_error(e);
        ae::Expression::new_error_node(QualifiedType::int())
      },
    };
    let analyzed_cases = cases
      .into_iter()
      .map(|case| self.casestmt(case).shall_ok("switch case"))
      .collect::<Vec<_>>();

    let analyzed_default = match default {
      Some(default) =>
        Some(self.defaultstmt(default).shall_ok("switch default")),
      None => None,
    };
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
    let analyzed_value = match self.expression(value) {
      Ok(val) if val.is_integer_constant() => val,
      Ok(val) => {
        self.add_error(
          ExprNotConstant(format!(
            "Integer constant expression must have integer type, found '{}'",
            val.qualified_type()
          ))
          .into_with(span),
        );
        val
      },
      Err(e) => {
        self.add_error(e);
        ae::Expression::new_error_node(QualifiedType::int())
      },
    };
    let analyzed_body = self.statements(body);

    Ok(astmt::Case::new(analyzed_value, analyzed_body, span))
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
            self.statement(*statement).handle_or_dummy(self),
            span,
          )),
          false => Err(DuplicateLabel(name).into_with(span)),
        }
      },
    }
  }

  fn gotostmt(&mut self, goto: ps::Goto) -> StmtRes<astmt::Statement> {
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
        Ok(astmt::Statement::Goto(astmt::Goto::new(
          goto.label, goto.span,
        )))
      },
    }
  }

  fn breakstmt(&mut self, break_stmt: ps::Break) -> StmtRes<astmt::Statement> {
    match self.environment.is_global() {
      true => contract_violation!(
        "break statement in global scope should be handled in parser"
      ),
      false => Ok(astmt::Statement::Break(astmt::Break::new(
        break_stmt.tag,
        break_stmt.span,
      ))),
    }
  }

  fn continuestmt(
    &mut self,
    continue_stmt: ps::Continue,
  ) -> StmtRes<astmt::Statement> {
    match self.environment.is_global() {
      true => contract_violation!(
        "continue statement in global scope should be handled in parser"
      ),
      false => Ok(astmt::Statement::Continue(astmt::Continue::new(
        continue_stmt.tag,
        continue_stmt.span,
      ))),
    }
  }
}

mod test {

  #[test]
  fn oneplusone() {
    use crate::{analyzer::Analyzer, parser::expression as pe};
    // 1 + 1
    let mut analyzer = Analyzer::default();
    let expr = pe::Expression::oneplusone();
    let analyzed_expr = analyzer.expression(expr);

    assert!(analyzed_expr.is_ok());
    println!("{:#?}", dbg!(analyzed_expr.unwrap()));
  }
}
