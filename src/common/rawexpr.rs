use crate::common::{operator::Operator, types::QualifiedType};

/// likely a sophisticated version of the Two-Level Types  
/// [this article](https://blog.ezyang.com/2013/05/the-ast-typing-problem/),
/// I probably used the Parametric Polymorphism to "tie the knot" of recursion.
#[derive(Debug)]
#[allow(unused)]
pub enum RawExpr<ExprTy, VarTy, TypeTy> {
  Empty, // no-op for error recovery; for empty expr should use Option<ExprTy> instead
  Constant(Constant),
  Unary(Unary<ExprTy>),
  Binary(Binary<ExprTy>),
  Assignment(Assignment<ExprTy>),
  Variable(Variable<VarTy>),
  Call(Call<ExprTy>),
  MemberAccess(MemberAccess<ExprTy>),
  Ternary(Ternary<ExprTy>),
  SizeOf(SizeOf<ExprTy, TypeTy>),
  Cast(Cast<ExprTy>),                     // (int)x
  ArraySubscript(ArraySubscript<ExprTy>), // arr[i]
  CompoundLiteral(CompoundLiteral),       // (struct Point){.x=1, .y=2}
}

#[macro_export(local_inner_macros)]
macro_rules! type_alias_expr {
  ($exprty:ident,$varty:ident,$typety:ident) => {
    pub type RawExpr = crate::common::rawexpr::RawExpr<$exprty, $varty, $typety>;
    pub type Constant = crate::common::rawexpr::Constant;
    pub type Unary = crate::common::rawexpr::Unary<$exprty>;
    pub type Binary = crate::common::rawexpr::Binary<$exprty>;
    pub type Assignment = crate::common::rawexpr::Assignment<$exprty>;
    pub type Variable = crate::common::rawexpr::Variable<$varty>;
    pub type Call = crate::common::rawexpr::Call<$exprty>;
    pub type MemberAccess = crate::common::rawexpr::MemberAccess<$exprty>;
    pub type Ternary = crate::common::rawexpr::Ternary<$exprty>;
    pub type SizeOf = crate::common::rawexpr::SizeOf<$exprty, $typety>;
    pub type Cast = crate::common::rawexpr::Cast<$exprty>;
    pub type ArraySubscript = crate::common::rawexpr::ArraySubscript<$exprty>;
    pub type CompoundLiteral = crate::common::rawexpr::CompoundLiteral;
  };
}

#[derive(Debug)]
pub enum Constant {
  Char(i8),
  Short(i16),
  Int(i32),
  LongLong(i64),
  UChar(u8),
  UShort(u16),
  UInt(u32),
  ULongLong(u64),
  Float(f32),
  Double(f64),
  Bool(bool),
  String(String),
}
#[derive(Debug)]
pub struct Unary<ExprTy> {
  pub operator: Operator,
  pub expression: Box<ExprTy>,
}
#[derive(Debug)]
pub struct Binary<ExprTy> {
  pub operator: Operator,
  pub left: Box<ExprTy>,
  pub right: Box<ExprTy>,
}
#[derive(Debug)]
pub struct Variable<VarTy> {
  pub name: VarTy,
}
#[derive(Debug)]
pub struct Assignment<ExprTy> {
  pub left: Box<ExprTy>,
  pub right: Box<ExprTy>,
}
#[derive(Debug)]
pub struct Call<ExprTy> {
  pub callee: Box<ExprTy>,
  pub arguments: Vec<ExprTy>,
}
#[derive(Debug)]
pub struct MemberAccess<ExprTy> {
  pub object: Box<ExprTy>,
  pub member: String,
}
#[derive(Debug)]
pub struct Ternary<ExprTy> {
  pub condition: Box<ExprTy>,
  pub then_branch: Box<ExprTy>,
  pub else_branch: Box<ExprTy>,
}
#[derive(Debug)]
pub enum SizeOf<ExprTy, TypeTy> {
  Type(TypeTy), // ignore for now
  Expression(Box<ExprTy>),
}

#[derive(Debug)]
pub struct Cast<ExprTy> {
  pub target_type: QualifiedType,
  pub expression: Box<ExprTy>,
}
#[derive(Debug)]
pub struct ArraySubscript<ExprTy> {
  pub array: Box<ExprTy>,
  pub index: Box<ExprTy>,
}
#[derive(Debug)]
pub struct CompoundLiteral {
  pub target_type: QualifiedType,
  // pub initializer: Initializer,
}

impl Constant {
  pub fn from_str(str: &String) -> Self {
    let int32 = str.clone().parse::<i32>().unwrap();
    Self::Int(int32)
  }
}

impl<ExprTy> Unary<ExprTy> {
  pub fn from_operator(operator: Operator, expression: ExprTy) -> Option<Self> {
    match operator.unary() {
      true => Some(Self {
        operator,
        expression: Box::new(expression),
      }),
      false => None,
    }
  }
  pub fn new(operator: Operator, expression: ExprTy) -> Self {
    Self::from_operator(operator, expression).unwrap()
  }
}

impl<ExprTy> Binary<ExprTy> {
  pub fn from_operator(operator: Operator, left: ExprTy, right: ExprTy) -> Option<Self> {
    match operator.binary() {
      true => Some(Self {
        operator,
        left: Box::new(left),
        right: Box::new(right),
      }),
      false => None,
    }
  }
  pub fn new(operator: Operator, left: ExprTy, right: ExprTy) -> Self {
    Self::from_operator(operator, left, right).unwrap()
  }
}
impl<ExprTy> Ternary<ExprTy> {
  pub fn new(condition: ExprTy, then_branch: ExprTy, else_branch: ExprTy) -> Self {
    Self {
      condition: Box::new(condition),
      then_branch: Box::new(then_branch),
      else_branch: Box::new(else_branch),
    }
  }
}

impl<ExprTy> Call<ExprTy> {
  pub fn new(callee: ExprTy, arguments: Vec<ExprTy>) -> Self {
    Self {
      callee: Box::new(callee),
      arguments,
    }
  }
}
mod fmt {
  use super::{Assignment, Binary, Call, Constant, RawExpr, Ternary, Unary, Variable};
  use ::std::fmt::Display;

  impl<ExprTy: Display> Display for Assignment<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} =)", self.left, self.right)
    }
  }

  impl<ExprTy: Display, VarTy, TypeTy> Display for RawExpr<ExprTy, VarTy, TypeTy>
  where
    Variable<VarTy>: Display,
  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        RawExpr::Constant(c) => <Constant as Display>::fmt(c, f),
        RawExpr::Unary(u) => <Unary<ExprTy> as Display>::fmt(u, f),
        RawExpr::Binary(b) => <Binary<ExprTy> as Display>::fmt(b, f),
        RawExpr::Assignment(a) => <Assignment<ExprTy> as Display>::fmt(a, f),
        RawExpr::Variable(v) => <Variable<VarTy> as Display>::fmt(v, f),
        RawExpr::Ternary(t) => <Ternary<ExprTy> as Display>::fmt(t, f),
        RawExpr::Call(call) => <Call<ExprTy> as Display>::fmt(call, f),
        RawExpr::Empty => write!(f, "<noop>"),
        _ => todo!(),
      }
    }
  }

  impl<ExprTy: Display> Display for Call<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}(", self.callee)?;
      for (i, arg) in self.arguments.iter().enumerate() {
        write!(f, "{}", arg)?;
        if i != self.arguments.len() - 1 {
          write!(f, ", ")?;
        }
      }
      write!(f, ")")
    }
  }

  impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Constant::Char(i) => write!(f, "{}", i),
        Constant::Short(i) => write!(f, "{}", i),
        Constant::Int(i) => write!(f, "{}", i),
        Constant::LongLong(i) => write!(f, "{}", i),
        Constant::UChar(u) => write!(f, "{}", u),
        Constant::UShort(u) => write!(f, "{}", u),
        Constant::UInt(u) => write!(f, "{}", u),
        Constant::ULongLong(u) => write!(f, "{}", u),
        Constant::Float(fl) => write!(f, "{}", fl),
        Constant::Double(fl) => write!(f, "{}", fl),
        Constant::Bool(b) => write!(f, "{}", b),
        Constant::String(s) => write!(f, "\"{}\"", s),
      }
    }
  }
  impl<ExprTy: Display> Display for Unary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {})", self.expression, self.operator)
    }
  }
  impl<ExprTy: Display> Display for Binary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }
  impl<ExprTy: Display> Display for Ternary<ExprTy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "({} ? {} : {})",
        self.condition, self.then_branch, self.else_branch
      )
    }
  }
}
