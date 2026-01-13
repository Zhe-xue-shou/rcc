use crate::{
  analyzer::{Analyzer, declaration as ad, expression as ae, statement as astmt},
  breakpoint,
  common::{
    environment::{Environment, Symbol, VarDeclKind},
    error::Error,
    operator::{Category, Operator},
    rawdecl::FunctionSpecifier,
    storage::Storage,
    types::{
      Array, ArraySize, CastType, Compatibility, FunctionProto, Pointer, Primitive, QualifiedType,
      Qualifiers, Type, TypeInfo,
    },
  },
  parser::{declaration as pd, expression as pe, statement as ps},
};

#[cfg(test)]
use ::pretty_assertions::assert_eq;

type TypeRes = Result<Type, Error>;
type ExprRes = Result<ae::Expression, Error>;
type DeclRes<T> = Result<T, Error>;
type StmtRes<T> = Result<T, Error>;

/// i haven't built up the error handling system yet, and all tests are expected to pass, so use this macro to catch bugs while debugging
macro_rules! err_or_debugbreak {
  () => {{
    breakpoint!();
    Err(())
  }};
}

impl Analyzer {
  pub fn new(program: pd::Program) -> Self {
    Self {
      program,
      ..Analyzer::default()
    }
  }
  pub fn add_error(&mut self, error: String) {
    self.errors.push(error);
  }
  pub fn add_warning(&mut self, warning: String) {
    self.warnings.push(warning);
  }
  pub fn analyze(&mut self) -> DeclRes<ad::TranslationUnit> {
    self.environment.enter();
    let mut declarations = Vec::new();
    std::mem::take(&mut self.program)
      .declarations
      .into_iter()
      .try_for_each(|decl| {
        let analyzed_decl = self.analyze_declarations(decl)?;
        declarations.push(analyzed_decl);
        Ok(())
      })?;
    self.environment.exit();
    match self.errors.is_empty() {
      true => Ok(ad::TranslationUnit::new(declarations)),
      false => err_or_debugbreak!(),
    }
  }
  pub fn errors(&self) -> &[String] {
    &self.errors
  }
  pub fn warnings(&self) -> &[String] {
    &self.warnings
  }
}
impl Analyzer {
  pub fn analyze_declarations(&mut self, declaration: pd::Declaration) -> DeclRes<ad::Declaration> {
    match declaration {
      pd::Declaration::Function(function) => {
        let func = self.analyze_functiondecl(function)?;
        Ok(ad::Declaration::Function(func))
      }
      pd::Declaration::Variable(vardef) => {
        let var = self.analyze_vardef(vardef)?;
        Ok(ad::Declaration::Variable(var))
      }
    }
  }
  pub fn analyze_functiondecl(&mut self, function: pd::Function) -> DeclRes<ad::Function> {
    let pd::Function {
      body,
      declarator,
      declspecs,
    } = function;
    let (function_specifier, storage, return_type) = Self::parse_declspecs(declspecs)?;
    let storage = match storage {
      Some(s) => s,
      None => Storage::Extern,
    };
    let pd::Declarator { modifiers, name } = declarator;
    let name = name.ok_or(())?; // function must have a name
    let (qualified_type, parameters) =
      Self::apply_modifiers_for_functiondecl(return_type, modifiers)?;
    let symbol = Symbol::new_ref(Symbol::new(
      qualified_type,
      storage,
      name,
      if body.is_some() {
        VarDeclKind::Definition
      } else {
        VarDeclKind::Declaration
      },
    ));

    let body = body
      .map(|b| {
        if self.current_function.is_some() {
          return err_or_debugbreak!(); // error: nested function definition is not allowed, this should be handled in parser
        }

        self.current_function = Some(symbol.clone());

        let result = self.analyze_compound_with(b, |analyzer| {
          // Declare parameters in the function scope
          for parameter in &parameters {
            if let Some(param_symbol) = &parameter.symbol {
              analyzer
                .environment
                .symbols
                .declare(param_symbol.borrow().name.clone(), param_symbol.clone());
            }
          }
        });

        self.current_function = None;

        return result;
      })
      .transpose()?;

    self
      .environment
      .symbols
      .declare(symbol.borrow().name.clone(), symbol.clone());

    Ok(ad::Function::new(
      symbol,
      parameters,
      function_specifier,
      body,
    ))
  }
  pub fn analyze_vardef(&mut self, vardef: pd::VarDef) -> DeclRes<ad::VarDef> {
    let pd::VarDef {
      declarator,
      declspecs,
      initializer,
    } = vardef;
    let (function_specifier, storage, qualified_type) = Self::parse_declspecs(declspecs)?;
    if !function_specifier.is_empty() {
      return err_or_debugbreak!(); // var cannot have inline and noreturn
    }
    let pd::Declarator { modifiers, name } = declarator;
    let name = name.ok_or(())?;
    let qualified_type = Self::apply_modifiers_for_varty(qualified_type, modifiers);
    let initializer = match initializer {
      Some(init) => match init {
        pd::Initializer::Expression(expression) => Some(ad::Initializer::Scalar(
          self.analyze_expression(*expression)?,
        )),
        pd::Initializer::List(_) => {
          breakpoint!();
          todo!()
        }
      },
      None => None,
    };
    // todo: check initializer type compatibility

    let vardef = match self.environment.is_global() {
      true => self.global_vardef(storage, qualified_type, name.clone(), initializer),
      false => self.local_vardef(
        storage.unwrap_or(Storage::Automatic),
        qualified_type,
        name.clone(),
        initializer,
      ),
    }?;
    // no prev - just insert
    // if found a *real* definition and current vardef is also a real refinition -> error
    // prev: extern -- update storage class (and possibly initializer)
    // prev: tentative -- update to definition
    // prev: declaration -- update to definition
    if let Some(prev_symbol_ref) = self.environment.find(&name) {
      // let mut prev_symbol = prev_symbol_ref.borrow_mut();
      let prev_declkind = prev_symbol_ref.borrow().declkind.clone();
      let new_declkind = vardef.symbol.borrow().declkind.clone();
      // todo: type checking so that redeclaration/definition with different type is caught
      // not just compare
      if !QualifiedType::compatible(
        &prev_symbol_ref.borrow().qualified_type,
        &vardef.symbol.borrow().qualified_type,
      ) {
        return err_or_debugbreak!(); // error: conflicting types for redeclaration/definition
      }
      type VDK = VarDeclKind;
      match (&prev_declkind, &new_declkind) {
        (VDK::Definition, VDK::Definition) => err_or_debugbreak!(), // error: redefinition
        (VDK::Definition, VDK::Declaration) | (VDK::Definition, VDK::Tentative) => {
          // valid and nothing to do
          Ok(vardef)
        }
        (VDK::Declaration, VDK::Definition) | (VDK::Tentative, VDK::Definition) => {
          {
            let mut prev = prev_symbol_ref.borrow_mut();
            let new_symbol = vardef.symbol.borrow();
            prev.declkind = VDK::Definition;
            prev.storage_class = Storage::try_merge(&prev.storage_class, &new_symbol.storage_class)
              .unwrap_or_else(|error| {
                // self.error add: conflicting storage class
                // use prev storage class
                _ = error;
                prev.storage_class.clone()
              });
            prev.qualified_type = QualifiedType::composite_unchecked(
              &vardef.symbol.borrow().qualified_type,
              &prev_symbol_ref.borrow().qualified_type,
            );

            // dropped prev and new_symbol here
          }

          Ok(vardef)
        }
        (VDK::Declaration, VDK::Declaration)
        | (VDK::Tentative, VDK::Tentative)
        | (VDK::Declaration, VDK::Tentative)
        | (VDK::Tentative, VDK::Declaration) => {
          // only merge storage class if needed, todo
          Ok(vardef)
        }
      }
    } else {
      self
        .environment
        .symbols
        .declare(name, vardef.symbol.clone());
      Ok(vardef)
    }
  }
  fn global_vardef(
    &mut self,
    storage: Option<Storage>,
    qualified_type: QualifiedType,
    name: String,
    initializer: Option<ad::Initializer>,
  ) -> DeclRes<ad::VarDef> {
    Ok(match (storage, initializer) {
      (None, None) => {
        let symbol = Symbol::tentative(qualified_type, Storage::Extern, name);
        ad::VarDef::new(symbol, None)
      }
      (None, Some(initializer)) => {
        let symbol = Symbol::def(qualified_type, Storage::Extern, name);
        ad::VarDef::new(symbol, Some(initializer))
      }
      (Some(storage), None) => {
        let symbol = Symbol::decl(qualified_type, storage, name);
        ad::VarDef::new(symbol, None)
      }
      (Some(storage), Some(initializer)) => {
        if storage == Storage::Extern {
          return err_or_debugbreak!(); // warning, extern vardef should not have initializer
        }
        let symbol = Symbol::def(qualified_type, storage, name);
        ad::VarDef::new(symbol, Some(initializer))
      }
    })
  }
  fn local_vardef(
    &mut self,
    storage: Storage,
    qualified_type: QualifiedType,
    name: String,
    initializer: Option<ad::Initializer>,
  ) -> DeclRes<ad::VarDef> {
    if storage == Storage::Extern && initializer.is_some() {
      return err_or_debugbreak!(); // error: local extern vardef cannot have initializer
    }
    let symbol = Symbol::decl(qualified_type, storage, name);
    Ok(ad::VarDef::new(symbol, initializer))
  }
}
impl Analyzer {
  fn apply_modifiers_for_varty(
    mut qualified_type: QualifiedType,
    modifiers: Vec<pd::Modifier>,
  ) -> QualifiedType {
    // reverse order (right-to-left in C)
    for modifier in modifiers.into_iter().rev() {
      match modifier {
        pd::Modifier::Pointer(qualifiers) => {
          qualified_type = QualifiedType::new(
            qualifiers,
            Type::Pointer(Pointer::new(Box::new(qualified_type))),
          );
        }
        pd::Modifier::Array(array_mod) => {
          let size = match array_mod.bound {
            pd::ArrayBound::Constant(n) => ArraySize::Constant(n),
            pd::ArrayBound::Incomplete => ArraySize::Incomplete,
            pd::ArrayBound::Variable(_) => ArraySize::Incomplete, // simplify for now
          };
          qualified_type = QualifiedType::new(
            Qualifiers::empty(),
            Type::Array(Array {
              element_type: Box::new(qualified_type),
              size,
            }),
          );
        }
        pd::Modifier::Function(_) => {
          breakpoint!();
          unreachable!()
        }
      }
    }
    qualified_type
  }
  fn apply_modifiers_for_functiondecl(
    return_type: QualifiedType,
    modifiers: Vec<pd::Modifier>,
  ) -> DeclRes<(
    QualifiedType,
    Vec<ad::Parameter>, /* parameters name and their type, here's some repetition- parameter type had also been inside QualifiedType of the function */
  )> {
    assert_eq!(
      modifiers.len(),
      1,
      "function declarator should have only one modifier"
    );
    let function_signature = match modifiers.into_iter().next().unwrap() {
      pd::Modifier::Function(function_signature) => function_signature,
      _ => {
        breakpoint!();
        panic!("function declarator should have function modifier")
      }
    };
    // we need to build function type
    let parameters = Self::parse_parameters(function_signature.parameters)?;
    let is_variadic = function_signature.is_variadic;
    let parameter_types = parameters
      .iter()
      .map(|param| match &param.symbol {
        Some(sym) => sym.borrow().qualified_type.clone(),
        None => QualifiedType::new_unqualified(Type::Primitive(Primitive::Int)), // default to int
      })
      .collect::<Vec<QualifiedType>>();
    let functionproto = FunctionProto::new(return_type.into(), parameter_types, is_variadic);

    Ok((
      QualifiedType::new_unqualified(Type::FunctionProto(functionproto)),
      parameters,
    ))
  }
  fn parse_parameters(parameters: Vec<pd::Parameter>) -> DeclRes<Vec<ad::Parameter>> {
    let mut analyzed_parameters = Vec::new();
    for parameter in parameters {
      let pd::Parameter {
        declarator,
        declspecs,
      } = parameter;
      let (_, storage, qualified_type) = Self::parse_declspecs(declspecs)?;
      if storage.is_some() {
        return err_or_debugbreak!(); // error: parameter cannot have storage class
      }
      let pd::Declarator { modifiers, name } = declarator;
      let qualified_type = Self::apply_modifiers_for_varty(qualified_type, modifiers);
      let symbol = match name {
        Some(name) => Some(Symbol::new_ref(Symbol::new(
          qualified_type,
          Storage::Automatic,
          name,
          VarDeclKind::Declaration,
        ))),
        None => None,
      };
      analyzed_parameters.push(ad::Parameter::new(symbol));
    }
    Ok(analyzed_parameters)
  }
  fn parse_declspecs(
    declspecs: pd::DeclSpecs,
  ) -> Result<(FunctionSpecifier, Option<Storage>, QualifiedType), Error> {
    let unqualified_type = Self::get_type(declspecs.type_specifiers)?;
    let qualifiers = declspecs.qualifiers;
    let qualified_type = QualifiedType::new(qualifiers, unqualified_type);
    let storage_class = declspecs.storage_class;
    let function_specifier = declspecs.function_specifiers;

    Ok((function_specifier, storage_class, qualified_type))
  }
  fn get_type(mut type_specifiers: Vec<pd::TypeSpecifier>) -> TypeRes {
    assert_eq!(type_specifiers.is_empty(), false);
    // todo, convert typedefs into real types
    // type_specifiers.iter_mut().for_each(|ts| {});
    type_specifiers.sort_by_key(|s| s.sort_key());
    type Ts = pd::TypeSpecifier;
    // 6.7.3.1
    let m = match type_specifiers.as_slice() {
      [Ts::Nullptr] => Type::Primitive(Primitive::Nullptr),
      [Ts::Void] => Type::Primitive(Primitive::Void),

      [Ts::Bool] => Type::Primitive(Primitive::Bool),

      [Ts::Char] => Type::Primitive(Primitive::Char),
      [Ts::Signed, Ts::Char] => Type::Primitive(Primitive::SChar),
      [Ts::Unsigned, Ts::Char] => Type::Primitive(Primitive::UChar),

      [Ts::Short]
      | [Ts::Short, Ts::Int]
      | [Ts::Signed, Ts::Short]
      | [Ts::Signed, Ts::Short, Ts::Int] => Type::Primitive(Primitive::Short),
      [Ts::Unsigned, Ts::Short] | [Ts::Unsigned, Ts::Short, Ts::Int] => {
        Type::Primitive(Primitive::UShort)
      }

      [Ts::Int] | [Ts::Signed] | [Ts::Signed, Ts::Int] => Type::Primitive(Primitive::Int),
      [Ts::Unsigned] | [Ts::Unsigned, Ts::Int] => Type::Primitive(Primitive::UInt),

      [Ts::Long]
      | [Ts::Long, Ts::Int]
      | [Ts::Signed, Ts::Long]
      | [Ts::Signed, Ts::Long, Ts::Int] => Type::Primitive(Primitive::Long),
      [Ts::Unsigned, Ts::Long] | [Ts::Unsigned, Ts::Long, Ts::Int] => {
        Type::Primitive(Primitive::ULong)
      }

      [Ts::Long, Ts::Long]
      | [Ts::Long, Ts::Long, Ts::Int]
      | [Ts::Signed, Ts::Long, Ts::Long]
      | [Ts::Signed, Ts::Long, Ts::Long, Ts::Int] => Type::Primitive(Primitive::LongLong),
      [Ts::Unsigned, Ts::Long, Ts::Long] | [Ts::Unsigned, Ts::Long, Ts::Long, Ts::Int] => {
        Type::Primitive(Primitive::ULongLong)
      }

      [Ts::Float] => Type::Primitive(Primitive::Float),
      [Ts::Double] => Type::Primitive(Primitive::Double),
      [Ts::Long, Ts::Double] => Type::Primitive(Primitive::LongDouble),

      [Ts::Float, Ts::Complex] => Type::Primitive(Primitive::ComplexFloat),
      [Ts::Double, Ts::Complex] => Type::Primitive(Primitive::ComplexDouble),
      [Ts::Long, Ts::Double, Ts::Complex] => Type::Primitive(Primitive::ComplexLongDouble),

      // treat complex integers as error
      [Ts::Char, Ts::Complex]
      | [Ts::Signed, Ts::Char, Ts::Complex]
      | [Ts::Unsigned, Ts::Char, Ts::Complex]
      | [Ts::Short, Ts::Complex]
      | [Ts::Short, Ts::Int, Ts::Complex]
      | [Ts::Signed, Ts::Short, Ts::Complex]
      | [Ts::Signed, Ts::Short, Ts::Int, Ts::Complex]
      | [Ts::Unsigned, Ts::Short, Ts::Complex]
      | [Ts::Unsigned, Ts::Short, Ts::Int, Ts::Complex]
      | [Ts::Int, Ts::Complex]
      | [Ts::Signed, Ts::Complex]
      | [Ts::Signed, Ts::Int, Ts::Complex]
      | [Ts::Unsigned, Ts::Complex]
      | [Ts::Unsigned, Ts::Int, Ts::Complex]
      | [Ts::Long, Ts::Complex]
      | [Ts::Long, Ts::Int, Ts::Complex]
      | [Ts::Signed, Ts::Long, Ts::Complex]
      | [Ts::Signed, Ts::Long, Ts::Int, Ts::Complex]
      | [Ts::Unsigned, Ts::Long, Ts::Complex]
      | [Ts::Unsigned, Ts::Long, Ts::Int, Ts::Complex] => {
        breakpoint!();
        panic!("Complex integer types are not supported");
      }

      // skip _BitInt, _Decimal32, _Decimal64, _Decimal128 here
      _ => todo!("union, struct, enum, typedef, typeof, etc."),
    };
    Ok(m)
  }
}

impl Analyzer {
  fn analyze_expression(&mut self, expression: pe::Expression) -> ExprRes {
    match expression {
      pe::Expression::Empty => Ok(ae::Expression::default()),
      pe::Expression::Constant(constant) => self.analyze_constant(constant),
      pe::Expression::Unary(unary) => self.analyze_unary(unary),
      pe::Expression::Binary(binary) => self.analyze_binary(binary),
      pe::Expression::Variable(variable) => self.analyze_variable(variable),
      pe::Expression::Call(call) => self.analyze_call(call),
      pe::Expression::Ternary(ternary) => self.analyze_ternary(ternary),
      pe::Expression::SizeOf(sizeof) => self.analyze_sizeof(sizeof),
      pe::Expression::CStyleCast(cast) => self.analyze_cast(cast),
      pe::Expression::MemberAccess(_) => todo!(),
      pe::Expression::ArraySubscript(_) => todo!(),
      pe::Expression::CompoundLiteral(_) => todo!(),
    }
  }
  fn analyze_sizeof(&mut self, sizeof: pe::SizeOf) -> ExprRes {
    match sizeof {
      pe::SizeOf::Expression(expression) => {
        let analyzed_expr = self.analyze_expression(*expression)?;
        let size = analyzed_expr.qualified_type().unqualified_type.size();
        Ok(ae::Expression::new_rvalue(
          ae::RawExpr::Constant(ae::Constant::ULongLong(size as u64)),
          QualifiedType::new_unqualified(Type::Primitive(Primitive::ULongLong)),
        ))
      }
      pe::SizeOf::Type(unprocessed_type) => {
        let pe::UnprocessedType {
          declspecs,
          declarator,
        } = unprocessed_type;
        let qualified_type = {
          let (_, _, base_type) = Self::parse_declspecs(declspecs)?;
          Self::apply_modifiers_for_varty(base_type, declarator.modifiers)
        };
        Ok(ae::Expression::new_rvalue(
          ae::RawExpr::Constant(ae::Constant::ULongLong(qualified_type.size() as u64)),
          QualifiedType::new_unqualified(Type::Primitive(Primitive::ULongLong)),
        ))
      }
    }
  }
  fn analyze_call(&mut self, call: pe::Call) -> ExprRes {
    let pe::Call { arguments, callee } = call;
    let analyzed_callee = self.analyze_expression(*callee)?;

    let function_proto = match analyzed_callee.unqualified_type() {
      Type::FunctionProto(proto) => proto,
      Type::Pointer(ptr) => match &ptr.pointee.unqualified_type {
        Type::FunctionProto(proto) => proto,
        _ => return err_or_debugbreak!(), // error: callee is not a function pointer
      },
      _ => return err_or_debugbreak!(), // error: callee is not a function
    };

    let mut analyzed_arguments = Vec::new();
    for argument in arguments {
      let analyzed_argument = self.analyze_expression(argument)?;
      analyzed_arguments.push(analyzed_argument);
    }

    if !function_proto.is_variadic
      && analyzed_arguments.len() != function_proto.parameter_types.len()
    {
      return err_or_debugbreak!(); // error: argument count mismatch
    }
    let expr_type = function_proto.return_type.as_ref().clone();
    // todo: type promotion, currently just match the exact/compatible types
    Ok(ae::Expression::new_rvalue(
      ae::RawExpr::Call(ae::Call::new(analyzed_callee, analyzed_arguments)),
      expr_type,
    ))
  }
  fn analyze_cast(&mut self, cast: pe::CStyleCast) -> ExprRes {
    todo!()
  }
  fn analyze_variable(&mut self, variable: pe::Variable) -> ExprRes {
    let symbol = self.environment.find(&variable.name).ok_or(())?;
    if symbol.borrow().is_typedef() {
      err_or_debugbreak!()
    } else {
      Ok(ae::Expression::new_lvalue(
        ae::RawExpr::Variable(ae::Variable::new(symbol.clone())),
        symbol.borrow().qualified_type.clone(),
      ))
    }
  }
  fn analyze_constant(&mut self, constant: pe::Constant) -> ExprRes {
    let unqualified_type = constant.unqualified_type();
    let value_category = if constant.is_char_array() {
      ae::ValueCategory::LValue
    } else {
      ae::ValueCategory::RValue
    };
    Ok(ae::Expression::new(
      ae::RawExpr::Constant(constant),
      QualifiedType::new_unqualified(unqualified_type),
      value_category,
    ))
  }
  fn analyze_unary(&mut self, unary: pe::Unary) -> ExprRes {
    let pe::Unary {
      operator,
      expression: pe_expr,
    } = unary;
    // TODO: type conversions based on operator
    let expression = self.analyze_expression(*pe_expr)?;
    let qualified_type = expression.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::RawExpr::Unary(ae::Unary::new(operator, expression)),
      qualified_type,
    ))
  }
  fn analyze_binary(&mut self, binary: pe::Binary) -> ExprRes {
    let pe::Binary {
      left: pe_left,
      operator,
      right: pe_right,
    } = binary;
    let left = self.analyze_expression(*pe_left)?;
    let right = self.analyze_expression(*pe_right)?;
    match operator.category() {
      Category::Assignment => self.analyze_assignment(operator, left, right),
      Category::Logical => self.analyze_logical(operator, left, right),
      Category::Relational => self.analyze_relational(operator, left, right),
      Category::Arithmetic => self.analyze_arithmetic(operator, left, right),
      Category::Bitwise => self.analyze_bitwise(operator, left, right),
      Category::BitShift => self.analyze_bitshift(operator, left, right),
      Category::Comma => self.analyze_comma(operator, left, right),
    }
  }
  fn analyze_ternary(&mut self, ternary: pe::Ternary) -> ExprRes {
    let pe::Ternary {
      condition: pe_condition,
      then_expr: pe_then_expr,
      else_expr: pe_else_expr,
    } = ternary;
    let condition = self.analyze_expression(*pe_condition)?;
    let then_expr = self.analyze_expression(*pe_then_expr)?;
    let else_expr = self.analyze_expression(*pe_else_expr)?;

    match (then_expr.unqualified_type(), else_expr.unqualified_type()) {
      (Type::Primitive(Primitive::Void), Type::Primitive(Primitive::Void)) => {
        Ok(ae::Expression::new_rvalue(
          ae::RawExpr::Ternary(ae::Ternary::new(condition, then_expr, else_expr)),
          QualifiedType::void(),
        ))
      }
      (Type::Primitive(Primitive::Void), _) => Ok(ae::Expression::new_rvalue(
        ae::RawExpr::Ternary(ae::Ternary::new(
          condition,
          then_expr,
          ae::Expression::void_conversion(else_expr),
        )),
        QualifiedType::void(),
      )),
      (_, Type::Primitive(Primitive::Void)) => Ok(ae::Expression::new_rvalue(
        ae::RawExpr::Ternary(ae::Ternary::new(
          condition,
          ae::Expression::void_conversion(then_expr),
          else_expr,
        )),
        QualifiedType::void(),
      )),
      // both arithmetic -> usual arithmetic conversion
      (left_type, right_type) if left_type.is_arithmetic() && right_type.is_arithmetic() => {
        let (then_converted, else_converted, result_type) =
          ae::Expression::usual_arithmetic_conversion(then_expr, else_expr)?;
        Ok(ae::Expression::new_rvalue(
          ae::RawExpr::Ternary(ae::Ternary::new(condition, then_converted, else_converted)),
          result_type,
        ))
      }
      // both pointer to compatible type -> composite type
      (Type::Pointer(left_ptr), Type::Pointer(right_ptr)) => {
        let left_pointee = &left_ptr.pointee;
        let right_pointee = &right_ptr.pointee;
        if QualifiedType::compatible(left_pointee, right_pointee) {
          let qualified_type = QualifiedType::composite_unchecked(left_pointee, right_pointee);
          let result_type =
            QualifiedType::new_unqualified(Type::Pointer(Pointer::new(Box::new(qualified_type))));
          Ok(ae::Expression::new_rvalue(
            ae::RawExpr::Ternary(ae::Ternary::new(condition, then_expr, else_expr)),
            result_type,
          ))
        } else {
          err_or_debugbreak!() // error: incompatible pointer types in ternary expression
        }
      }
      _ => todo!(),
    }
  }
  fn analyze_assignment(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
  ) -> ExprRes {
    assert!(
      operator == Operator::Assign,
      "compound assignment not implemented"
    );
    if !left.is_modifiable_lvalue() {
      return err_or_debugbreak!(); // expression is not assignable
    }
    let assigned_expr = right
      .lvalue_conversion()
      .decay()
      .assignment_conversion(&left.qualified_type())?;
    let expr_type = left.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::RawExpr::Binary(ae::Binary::new(operator, left, assigned_expr)),
      expr_type,
    ))
  }
  fn analyze_logical(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    let lhs = left.conditional_conversion()?;
    let rhs = right.conditional_conversion()?;
    Ok(ae::Expression::new_rvalue(
      ae::RawExpr::Binary(ae::Binary::new(operator, lhs, rhs)),
      QualifiedType::new_unqualified(Type::Primitive(Primitive::Bool)),
    ))
  }
  fn analyze_relational(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    // Path A
    if left.unqualified_type().is_arithmetic() && right.unqualified_type().is_arithmetic() {
      let (lhs, rhs, _common_type) = ae::Expression::usual_arithmetic_conversion(left, right)?;

      return Ok(ae::Expression::new_rvalue(
        ae::RawExpr::Binary(ae::Binary::new(operator, lhs, rhs)),
        QualifiedType::new_unqualified(Primitive::Bool.into()),
      ));
    }
    todo!()
  }
  fn analyze_arithmetic(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    let (lhs, rhs, result_type) = ae::Expression::usual_arithmetic_conversion(left, right)?;
    Ok(ae::Expression::new_rvalue(
      ae::RawExpr::Binary(ae::Binary::new(operator, lhs, rhs)),
      result_type,
    ))
  }
  fn analyze_bitwise(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
  ) -> ExprRes {
    let left = left.lvalue_conversion().decay();
    let right = right.lvalue_conversion().decay();

    if !left.unqualified_type().is_integer() || !right.unqualified_type().is_integer() {
      return err_or_debugbreak!(); // error: bitwise operator requires integer operands
    }

    let (lhs, rhs, result_type) = ae::Expression::usual_arithmetic_conversion(left, right)?;

    Ok(ae::Expression::new_rvalue(
      ae::RawExpr::Binary(ae::Binary::new(operator, lhs, rhs)),
      result_type,
    ))
  }
  fn analyze_bitshift(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
  ) -> ExprRes {
    let lhs = left.lvalue_conversion().decay().promote();
    let rhs = right.lvalue_conversion().decay().promote();

    if !lhs.unqualified_type().is_integer() || !rhs.unqualified_type().is_integer() {
      return err_or_debugbreak!(); // error: bitshift operator requires integer operands
    }

    let expr_type = lhs.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::RawExpr::Binary(ae::Binary::new(operator, lhs, rhs)),
      expr_type,
    ))
  }
  fn analyze_comma(
    &mut self,
    operator: Operator,
    left: ae::Expression,
    right: ae::Expression,
  ) -> ExprRes {
    // the result is the right expression, and the left is void converted, that's it. done.
    let expr_type = right.qualified_type().clone();
    Ok(ae::Expression::new_rvalue(
      ae::RawExpr::Binary(ae::Binary::new(operator, left.void_conversion(), right)),
      expr_type,
    ))
  }
}
impl Analyzer {
  fn analyze_statement(&mut self, statement: ps::Statement) -> StmtRes<astmt::Statement> {
    match statement {
      ps::Statement::Expression(expression) => self.analyze_exprstmt(expression),
      ps::Statement::Compound(compound_stmt) => Ok(astmt::Statement::Compound(
        self.analyze_compound(compound_stmt)?,
      )),
      ps::Statement::Empty() => Ok(astmt::Statement::Empty()),
      ps::Statement::Return(return_stmt) => {
        Ok(astmt::Statement::Return(self.analyze_return(return_stmt)?))
      }
      ps::Statement::Declaration(declaration) => Ok(astmt::Statement::Declaration(
        self.analyze_declarations(declaration)?,
      )),
      ps::Statement::If(if_stmt) => Ok(astmt::Statement::If(self.analyze_if(if_stmt)?)),
      ps::Statement::While(while_stmt) => {
        Ok(astmt::Statement::While(self.analyze_while(while_stmt)?))
      }
      ps::Statement::DoWhile(do_while) => {
        Ok(astmt::Statement::DoWhile(self.analyze_do_while(do_while)?))
      }
      ps::Statement::For(for_stmt) => Ok(astmt::Statement::For(self.analyze_for(for_stmt)?)),
      ps::Statement::Label(_) => {
        todo!("requires adding field into analyzer struct to indicarte which function we're in")
      }
      ps::Statement::Switch(switch) => Ok(astmt::Statement::Switch(self.analyze_switch(switch)?)),
      ps::Statement::Goto(_) => todo!(),
      ps::Statement::Break(_) => todo!("ditto, but requires the compound stack"),
      ps::Statement::Continue(_) => todo!("ditto"),
    }
  }
  fn analyze_compound(&mut self, compound: ps::Compound) -> StmtRes<astmt::Compound> {
    self.analyze_compound_with(compound, |_| {})
  }
  fn analyze_compound_with<Fn>(
    &mut self,
    compound: ps::Compound,
    callback: Fn,
  ) -> StmtRes<astmt::Compound>
  where
    Fn: FnOnce(&mut Self),
  {
    self.environment.enter();

    callback(self);

    // if any fail, we still exit the scope
    let result = (|| {
      let mut statements = Vec::new();
      for statement in compound.statements {
        let analyzed_stmt = self.analyze_statement(statement)?;
        statements.push(analyzed_stmt);
      }
      Ok(astmt::Compound::new(statements))
    })();

    self.environment.exit();

    result
  }
  fn analyze_exprstmt(&mut self, expr_stmt: pe::Expression) -> StmtRes<astmt::Statement> {
    // todo: unused expression result warning
    Ok(astmt::Statement::Expression(
      self.analyze_expression(expr_stmt)?,
    ))
  }
  fn analyze_return(&mut self, return_stmt: ps::Return) -> StmtRes<astmt::Return> {
    let ps::Return { expression } = return_stmt;
    let analyzed_expr = match expression {
      Some(expr) => Some(self.analyze_expression(expr)?),
      None => None,
    };
    assert!(
      self.current_function.is_some(),
      "return statement outside function should be handled in parser"
    );
    let return_type = match &self
      .current_function
      .as_ref()
      .unwrap()
      .borrow()
      .qualified_type
      .unqualified_type
    {
      Type::FunctionProto(proto) => proto.return_type.as_ref().clone(),
      _ => {
        breakpoint!();
        panic!("current function's type is not function proto")
      }
    };
    match (&analyzed_expr, &return_type.unqualified_type) {
      (None, Type::Primitive(Primitive::Void)) => {
        return Ok(astmt::Return::new(None));
      }
      (None, _) => {
        return err_or_debugbreak!(); // error: non-void function must return a value
      }
      (Some(_), Type::Primitive(Primitive::Void)) => {
        return err_or_debugbreak!(); // error: returning a value from a void function
      }
      (Some(_), _) => {
        let a = unsafe {
          // this has value for absolutely sure
          analyzed_expr.unwrap_unchecked()
        }
        .lvalue_conversion()
        .decay()
        .assignment_conversion(&return_type)?;
        Ok(astmt::Return::new(Some(a)))
      }
    }
  }
  fn analyze_if(&mut self, if_stmt: ps::If) -> StmtRes<astmt::If> {
    let ps::If {
      condition,
      then_branch,
      else_branch,
    } = if_stmt;
    let analyzed_condition = self.analyze_expression(condition)?;
    let analyzed_then_branch = Box::new(self.analyze_statement(*then_branch)?);
    let analyzed_else_branch = match else_branch {
      Some(else_branch) => Some(Box::new(self.analyze_statement(*else_branch)?)),
      None => None,
    };
    Ok(astmt::If::new(
      analyzed_condition,
      analyzed_then_branch,
      analyzed_else_branch,
    ))
  }
  fn analyze_while(&mut self, while_stmt: ps::While) -> StmtRes<astmt::While> {
    let ps::While {
      condition,
      body,
      label,
    } = while_stmt;
    let analyzed_condition = self.analyze_expression(condition)?;
    let analyzed_body = Box::new(self.analyze_statement(*body)?);
    Ok(astmt::While::new(analyzed_condition, analyzed_body, label))
  }
  fn analyze_do_while(&mut self, do_while: ps::DoWhile) -> StmtRes<astmt::DoWhile> {
    let ps::DoWhile {
      body,
      condition,
      label,
    } = do_while;
    let analyzed_body = Box::new(self.analyze_statement(*body)?);
    let analyzed_condition = self.analyze_expression(condition)?;
    Ok(astmt::DoWhile::new(
      analyzed_body,
      analyzed_condition,
      label,
    ))
  }
  fn analyze_for(&mut self, for_stmt: ps::For) -> StmtRes<astmt::For> {
    let ps::For {
      initializer,
      condition,
      increment,
      body,
      label,
    } = for_stmt;
    let analyzed_initializer = initializer
      .map(|init| self.analyze_statement(*init))
      .transpose()?
      .map(Box::new);
    let analyzed_condition = condition
      .map(|cond| self.analyze_expression(cond))
      .transpose()?;
    let analyzed_increment = increment
      .map(|inc| self.analyze_expression(inc))
      .transpose()?;
    let analyzed_body = Box::new(self.analyze_statement(*body)?);
    Ok(astmt::For::new(
      analyzed_initializer,
      analyzed_condition,
      analyzed_increment,
      analyzed_body,
      label,
    ))
  }
  fn analyze_switch(&mut self, switch: ps::Switch) -> StmtRes<astmt::Switch> {
    todo!()
  }
}
impl ::core::default::Default for Analyzer {
  fn default() -> Self {
    Self {
      program: pd::Program::new(),
      environment: Environment::new(),
      current_function: None,
      errors: Vec::new(),
      warnings: Vec::new(),
    }
  }
}

mod test {

  #[test]
  fn oneplusone() {
    use crate::{analyzer::Analyzer, parser::expression as pe};
    // 1 + 1
    let mut analyzer = Analyzer::default();
    let expr = pe::Expression::Binary(pe::Binary {
      left: Box::new(pe::Expression::Constant(pe::Constant::Short(1))),
      operator: crate::common::operator::Operator::Plus,
      right: Box::new(pe::Expression::Constant(pe::Constant::Int(1))),
    });
    let analyzed_expr = analyzer.analyze_expression(expr);

    assert!(analyzed_expr.is_ok());
    println!("{:?}", analyzed_expr.unwrap());
  }
}
