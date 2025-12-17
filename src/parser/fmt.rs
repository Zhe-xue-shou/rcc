use crate::parser::{
  ast::{Declaration, FunctionDef, Program},
  expression::{Assignment, Binary, Constant, Expression, Unary, Variable},
  statement::{If, Return, Statement, VarDef},
  types::{Array, ArraySize, Function, QualifiedType, Qualifiers, Type},
};
use std::fmt::{Debug, Display};

impl Display for Qualifiers {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut qualifiers = Vec::new();
    if self.contains(Qualifiers::Const) {
      qualifiers.push("const");
    }
    if self.contains(Qualifiers::Volatile) {
      qualifiers.push("volatile");
    }
    if self.contains(Qualifiers::Restrict) {
      qualifiers.push("restrict");
    }
    write!(f, "{}", qualifiers.join(" "))
  }
}

impl Display for QualifiedType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.qualifiers.is_empty() {
      write!(f, "{}", self.unqualified_type)
    } else {
      write!(f, "{} {}", self.qualifiers, self.unqualified_type)
    }
  }
}

impl Display for Array {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}[", self.element_type)?;
    match &self.size {
      ArraySize::Constant(sz) => write!(f, "{}", sz)?,
      ArraySize::Incomplete => write!(f, "")?,
    }
    write!(f, "]")
  }
}

impl Display for Function {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}(", self.return_type)?;
    for (i, param) in self.parameter_types.iter().enumerate() {
      if i > 0 {
        write!(f, ", ")?;
      }
      write!(f, "{}", param)?;
    }
    write!(f, ")")
  }
}

impl Display for Type {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Type::Primitive(builtin) => write!(f, "{}", builtin),
      &Type::Array(_) | &Type::Pointer(_) | &Type::Function(_) => todo!(),
    }
  }
}

impl Debug for Type {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Display for Declaration {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Declaration::Function(func) => <FunctionDef as Display>::fmt(func, f),
      Declaration::Variable(var) => <VarDef as Display>::fmt(var, f),
    }
  }
}

impl Display for Program {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self
      .declarations
      .iter()
      .try_for_each(|decl| write!(f, "{}\n", decl))
  }
}
impl Debug for Program {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Display for VarDef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.initializer {
      Some(expr) => write!(f, "Declaration {} = {}", self.name, expr),
      None => write!(f, "Declaration {}", self.name),
    }
  }
}

impl Display for Statement {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Statement::Return(ret) => <Return as Display>::fmt(ret, f),
      Statement::If(if_stmt) => <If as Display>::fmt(if_stmt, f),
      Statement::Declaration(decl) => <VarDef as Display>::fmt(decl, f),
      Statement::Expression(expr) => <Expression as Display>::fmt(expr, f),
    }
  }
}

impl Debug for Statement {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Display for Return {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.expression {
      Some(expr) => write!(f, "return {}", expr),
      None => write!(f, "return"),
    }
  }
}

impl Debug for Return {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Display for If {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "if {} {}", self.condition, self.if_branch)?;
    if !self.else_branch.statements.is_empty() {
      write!(f, " else {}", self.else_branch)?;
    }
    Ok(())
  }
}

impl Debug for If {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Display for Variable {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.name)
  }
}

impl Display for Assignment {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({} = {})", self.left, self.right)
  }
}

impl Display for Expression {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Expression::Constant(c) => <Constant as Display>::fmt(c, f),
      Expression::Unary(u) => <Unary as Display>::fmt(u, f),
      Expression::Binary(b) => <Binary as Display>::fmt(b, f),
      Expression::Assignment(a) => <Assignment as Display>::fmt(a, f),
      Expression::Variable(v) => <Variable as Display>::fmt(v, f),
    }
  }
}

impl Debug for Expression {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Display for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Constant::Int8(i) => write!(f, "{}", i),
      Constant::Int16(i) => write!(f, "{}", i),
      Constant::Int32(i) => write!(f, "{}", i),
      Constant::Int64(i) => write!(f, "{}", i),
      Constant::Uint8(u) => write!(f, "{}", u),
      Constant::Uint16(u) => write!(f, "{}", u),
      Constant::Uint32(u) => write!(f, "{}", u),
      Constant::Uint64(u) => write!(f, "{}", u),
      Constant::Float32(fl) => write!(f, "{}", fl),
      Constant::Float64(fl) => write!(f, "{}", fl),
      Constant::Bool(b) => write!(f, "{}", b),
      Constant::String(s) => write!(f, "\"{}\"", s),
    }
  }
}

impl Debug for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Display for Unary {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({} {})", self.operator, self.expression)
  }
}

impl Debug for Unary {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}

impl Display for Binary {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({} {} {})", self.operator, self.left, self.right)
  }
}

impl Debug for Binary {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as Display>::fmt(self, f)
  }
}
