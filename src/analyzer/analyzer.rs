use crate::{
  analyzer::{
    Analyzer, declaration as ad,
    expression::{self as ae, ValueCategory},
    statement as astmt,
  },
  breakpoint,
  common::{
    environment::{Environment, Symbol, VarDeclKind},
    error::Error,
    rawdecl::FunctionSpecifier,
    storage::Storage,
    types::{
      Array, ArraySize, Compatibility, FunctionProto, Pointer, Primitive, QualifiedType,
      Qualifiers, Type,
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
      false => Err(()),
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
    let body = body.map(|b| self.analyze_compound(b)).transpose()?;
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
      return Err(()); // var cannot have inline and noreturn
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
        return Err(()); // error: conflicting types for redeclaration/definition
      }
      type VDK = VarDeclKind;
      match (&prev_declkind, &new_declkind) {
        (VDK::Definition, VDK::Definition) => Err(()), // error: redefinition
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
          return Err(()); // warning, extern vardef should not have initializer
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
      return Err(()); // error: local extern vardef cannot have initializer
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
        None => QualifiedType::new(Qualifiers::empty(), Type::Primitive(Primitive::Int)), // default to int
      })
      .collect::<Vec<QualifiedType>>();
    let functionproto = FunctionProto::new(return_type, parameter_types, is_variadic);

    Ok((
      QualifiedType::new(Qualifiers::empty(), Type::FunctionProto(functionproto)),
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
        return Err(()); // error: parameter cannot have storage class
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
      [Ts::Void] => Type::Primitive(Primitive::Void),
      [Ts::Char] => Type::Primitive(Primitive::Char),
      [Ts::Signed, Ts::Char] => Type::Primitive(Primitive::Char),
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
      | [Ts::Signed, Ts::Long, Ts::Int] => Type::Primitive(Primitive::LongLong),
      [Ts::Unsigned, Ts::Long] | [Ts::Unsigned, Ts::Long, Ts::Int] => {
        Type::Primitive(Primitive::ULongLong)
      }
      [Ts::Long, Ts::Long]
      | [Ts::Long, Ts::Long, Ts::Int]
      | [Ts::Signed, Ts::Long, Ts::Long]
      | [Ts::Signed, Ts::Long, Ts::Long, Ts::Int] => Type::Primitive(Primitive::LongLong),
      [Ts::Unsigned, Ts::Long, Ts::Long] | [Ts::Unsigned, Ts::Long, Ts::Long, Ts::Int] => {
        Type::Primitive(Primitive::ULongLong)
      }
      [Ts::Float] => Type::Primitive(Primitive::Float),
      // treat long double as double for now
      [Ts::Double] | [Ts::Long, Ts::Double] => Type::Primitive(Primitive::Double),
      [Ts::Float, Ts::Complex] => Type::Primitive(Primitive::Float),
      [Ts::Double, Ts::Complex] | [Ts::Long, Ts::Double, Ts::Complex] => {
        Type::Primitive(Primitive::Double)
      }
      [Ts::Bool] => Type::Primitive(Primitive::Bool),

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
      pe::Expression::Assignment(assignment) => self.analyze_assignment(assignment),
      pe::Expression::Variable(variable) => self.analyze_variable(variable),
      pe::Expression::Call(call) => self.analyze_call(call),
      pe::Expression::MemberAccess(member_access) => todo!(),
      pe::Expression::Ternary(ternary) => self.analyze_ternary(ternary),
      pe::Expression::SizeOf(sizeof) => self.analyze_sizeof(sizeof),
      pe::Expression::Cast(cast) => todo!(),
      pe::Expression::ArraySubscript(array_subscript) => todo!(),
      pe::Expression::CompoundLiteral(compound_literal) => todo!(),
    }
  }
  fn analyze_sizeof(&mut self, sizeof: pe::SizeOf) -> ExprRes {
    match sizeof {
      pe::SizeOf::Expression(expression) => {
        let analyzed_expr = self.analyze_expression(*expression)?;
        let size = analyzed_expr.qualified_type().unqualified_type.size();
        Ok(ae::Expression::new(
          ae::RawExpr::Constant(ae::Constant::ULongLong(size as u64)),
          QualifiedType::new(Qualifiers::empty(), Type::Primitive(Primitive::ULongLong)),
          ValueCategory::RValue,
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
        Ok(ae::Expression::new(
          ae::RawExpr::Constant(ae::Constant::ULongLong(qualified_type.size() as u64)),
          QualifiedType::new(Qualifiers::empty(), Type::Primitive(Primitive::ULongLong)),
          ValueCategory::RValue,
        ))
      }
    }
  }
  fn analyze_call(&mut self, call: pe::Call) -> ExprRes {
    let pe::Call { arguments, callee } = call;
    let analyzed_callee = self.analyze_expression(*callee)?;
    let mut analyzed_arguments = Vec::new();
    for argument in arguments {
      let analyzed_argument = self.analyze_expression(argument)?;
      analyzed_arguments.push(analyzed_argument);
    }
    // find the
    todo!()
  }
  fn analyze_variable(&mut self, variable: pe::Variable) -> ExprRes {
    let symbol = self.environment.find(&variable.name).ok_or(())?;
    if symbol.borrow().is_typedef() {
      Err(())
    } else {
      Ok(ae::Expression::new(
        ae::RawExpr::Variable(ae::Variable::new(symbol.clone())),
        symbol.borrow().qualified_type.clone(),
        ValueCategory::LValue,
      ))
    }
  }
  fn analyze_constant(&mut self, constant: pe::Constant) -> ExprRes {
    let unqualified_type = match &constant {
      ae::Constant::Char(_) => Type::Primitive(Primitive::Char),
      ae::Constant::Short(_) => Type::Primitive(Primitive::Short),
      ae::Constant::Int(_) => Type::Primitive(Primitive::Int),
      ae::Constant::LongLong(_) => Type::Primitive(Primitive::LongLong),
      ae::Constant::UChar(_) => Type::Primitive(Primitive::UChar),
      ae::Constant::UShort(_) => Type::Primitive(Primitive::UShort),
      ae::Constant::UInt(_) => Type::Primitive(Primitive::UInt),
      ae::Constant::ULongLong(_) => Type::Primitive(Primitive::ULongLong),
      ae::Constant::Float(_) => Type::Primitive(Primitive::Float),
      ae::Constant::Double(_) => Type::Primitive(Primitive::Double),
      ae::Constant::Bool(_) => Type::Primitive(Primitive::Bool),
      // in C, char[N] is the type of string literal - although it's stored in read-only memory
      // in C++ it's const char[N]
      // ^^^ verified by clangd's AST
      ae::Constant::String(str) => Type::Array(Array::new(
        Box::new(QualifiedType::new(
          Qualifiers::empty(),
          Type::Primitive(Primitive::Char),
        )),
        // this is wrong for multi-byte characters, but let's ignore that for now
        ArraySize::Constant(str.len() + 1 /* null terminator */),
      )),
    };
    let value_category = if matches!(constant, ae::Constant::String(_)) {
      ValueCategory::LValue
    } else {
      ValueCategory::RValue
    };
    Ok(ae::Expression::new(
      ae::RawExpr::Constant(constant),
      QualifiedType::new(Qualifiers::empty(), unqualified_type),
      value_category,
    ))
  }
  fn analyze_unary(&mut self, unary: pe::Unary) -> ExprRes {
    let pe::Unary {
      operator,
      expression: pe_expr,
    } = unary;
    let expression = self.analyze_expression(*pe_expr)?;
    // TODO: type promotion of the unary and the expr_type
    let qualified_type = expression.qualified_type().clone();
    let value_category = ValueCategory::RValue;
    Ok(ae::Expression::new(
      ae::RawExpr::Unary(ae::Unary::new(operator, expression)),
      qualified_type,
      value_category,
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
    // ditto, todo
    let qualified_type = left.qualified_type().clone();
    Ok(ae::Expression::new(
      ae::RawExpr::Binary(ae::Binary::new(operator, left, right)),
      qualified_type,
      ValueCategory::RValue,
    ))
  }
  fn analyze_ternary(&mut self, ternary: pe::Ternary) -> ExprRes {
    let pe::Ternary {
      condition: pe_condition,
      then_branch: pe_then_expr,
      else_branch: pe_else_expr,
    } = ternary;
    let condition = self.analyze_expression(*pe_condition)?;
    let then_expr = self.analyze_expression(*pe_then_expr)?;
    let else_expr = self.analyze_expression(*pe_else_expr)?;

    if !then_expr
      .qualified_type()
      .compatible_with(&else_expr.qualified_type())
    {
      Err(())
    } else {
      let qualified_type =
        QualifiedType::composite_unchecked(then_expr.qualified_type(), else_expr.qualified_type());
      Ok(ae::Expression::new(
        ae::RawExpr::Ternary(ae::Ternary::new(condition, then_expr, else_expr)),
        qualified_type,
        ValueCategory::RValue,
      ))
    }
  }
  fn analyze_assignment(&mut self, assignment: pe::Assignment) -> ExprRes {
    let pe::Assignment {
      left: pe_left,
      right: pe_right,
    } = assignment;
    let left = self.analyze_expression(*pe_left)?;
    let right = self.analyze_expression(*pe_right)?;
    if !left.is_modifiable_lvalue() {
      Err(()) // expression is not assignable
    } else {
      // check type compatibility, todo
      todo!()
    }
  }
}
impl Analyzer {
  fn analyze_compound(&mut self, compound: ps::Compound) -> StmtRes<astmt::Compound> {
    let mut statements = Vec::new();
    for statement in compound.statements {
      let analyzed_stmt = self.analyze_statement(statement)?;
      statements.push(analyzed_stmt);
    }
    Ok(astmt::Compound::new(statements))
  }
}
impl Analyzer {
  fn analyze_statement(&mut self, statement: ps::Statement) -> StmtRes<astmt::Statement> {
    match statement {
      ps::Statement::Expression(expression) => self.analyze_expressionstmt(expression),
      ps::Statement::Compound(compound_stmt) => Ok(astmt::Statement::Compound(
        self.analyze_compound(compound_stmt)?,
      )),
      ps::Statement::Empty() => Ok(astmt::Statement::Empty()),
      ps::Statement::Return(return_stmt) => {
        Ok(astmt::Statement::Return(self.analyze_return(return_stmt)?))
      }
      ps::Statement::Declaration(declaration) => {
        let analyzed_decl = self.analyze_declarations(declaration)?;
        Ok(astmt::Statement::Declaration(analyzed_decl))
      }
      ps::Statement::If(if_stmt) => Ok(astmt::Statement::If(self.analyze_if(if_stmt)?)),
      ps::Statement::While(while_stmt) => {
        Ok(astmt::Statement::While(self.analyze_while(while_stmt)?))
      }
      ps::Statement::DoWhile(do_while) => {
        Ok(astmt::Statement::DoWhile(self.analyze_do_while(do_while)?))
      }
      ps::Statement::For(for_stmt) => Ok(astmt::Statement::For(self.analyze_for(for_stmt)?)),
      ps::Statement::Label(label) => {
        todo!("requires adding field into analyzer struct to indicarte which function we're in")
      }
      ps::Statement::Switch(switch) => todo!(),
      ps::Statement::Goto(goto) => todo!(),
      ps::Statement::Break(single_label) => todo!("ditto, but requires the compound stack"),
      ps::Statement::Continue(single_label) => todo!("ditto"),
    }
  }
  fn analyze_expressionstmt(&mut self, expr_stmt: pe::Expression) -> StmtRes<astmt::Statement> {
    // todo: unused expression resyult warning
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
    Ok(astmt::Return::new(analyzed_expr))
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
}
impl ::core::default::Default for Analyzer {
  fn default() -> Self {
    Self {
      program: pd::Program::new(),
      environment: Environment::new(),
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
      left: Box::new(pe::Expression::Constant(pe::Constant::Int(1))),
      operator: crate::common::operator::Operator::Plus,
      right: Box::new(pe::Expression::Constant(pe::Constant::Int(1))),
    });
    let analyzed_expr = analyzer.analyze_expression(expr);
    println!("{:#?}", analyzed_expr.unwrap());
  }
}
