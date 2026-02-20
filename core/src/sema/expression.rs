use crate::{
  blueprints::type_alias_expr,
  common::{Operator, OperatorCategory, SourceSpan, Storage, SymbolRef},
  types::{CastType, QualifiedType, Qualifiers, Type, TypeRef},
};

type_alias_expr! {Expression<'context>, QualifiedType<'context>, Variable<'context> ImplicitCast<'context> Assignment<'context>}
#[derive(Debug, Clone, Copy, ::strum_macros::Display, PartialEq)]
pub enum ValueCategory {
  LValue,
  /// 6.3.2: "rvalue" is in this document described as the "value of an expression".
  ///        which, is different from the one defined in C++ standard.
  RValue,
}
use ValueCategory::{LValue, RValue};

#[derive(Debug)]
pub struct Expression<'context> {
  raw_expr: RawExpr<'context>,
  expr_type: QualifiedType<'context>,
  value_category: ValueCategory,
}
impl<'context> Expression<'context> {
  pub fn new(
    raw_expr: RawExpr<'context>,
    expr_type: QualifiedType<'context>,
    value_category: ValueCategory,
  ) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category,
    }
  }

  pub fn new_rvalue(
    raw_expr: RawExpr<'context>,
    expr_type: QualifiedType<'context>,
  ) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category: RValue,
    }
  }

  pub fn new_lvalue(
    raw_expr: RawExpr<'context>,
    expr_type: QualifiedType<'context>,
  ) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category: LValue,
    }
  }

  pub fn new_error_node(expr_type: QualifiedType<'context>) -> Self {
    Self {
      raw_expr: RawExpr::Empty(Empty::default()),
      expr_type,
      value_category: RValue,
    }
  }

  pub fn unqualified_type(&self) -> TypeRef<'context> {
    self.expr_type.unqualified_type
  }

  pub fn qualifiers(&self) -> &Qualifiers {
    &self.expr_type.qualifiers
  }

  pub fn qualified_type(&self) -> &QualifiedType<'context> {
    &self.expr_type
  }

  pub fn raw_expr(&self) -> &RawExpr<'context> {
    &self.raw_expr
  }

  pub fn value_category(&self) -> ValueCategory {
    self.value_category
  }

  pub(crate) fn destructure(
    self,
  ) -> (RawExpr<'context>, QualifiedType<'context>, ValueCategory) {
    (self.raw_expr, self.expr_type, self.value_category)
  }
}

impl<'context> Expression<'context> {
  pub fn is_lvalue(&self) -> bool {
    matches!(self.value_category, LValue)
  }

  /// 6.3.2.1:  A modifiable lvalue is an lvalue that does not have array type, does not have an incomplete
  ///           type, does not have a const-qualified type, and if it is a structure or union, does not have any
  ///           member (including, recursively, any member or element of all contained aggregates or unions) with
  ///           a const-qualified type.
  pub fn is_modifiable_lvalue(&self) -> bool {
    self.is_lvalue() && self.qualified_type().is_modifiable()
  }

  pub fn into_rvalue(self) -> Self {
    Self {
      value_category: RValue,
      ..self
    }
  }

  pub fn span(&self) -> SourceSpan {
    self.raw_expr.span()
  }
}

impl<'context> ::core::default::Default for Expression<'context> {
  fn default() -> Self {
    const DUMMY_UNQUAL: TypeRef<'static> =
      &Type::Primitive(crate::types::Primitive::Void);
    const DUMMY: QualifiedType<'static> = DUMMY_UNQUAL.into();
    Self {
      raw_expr: RawExpr::Empty(Empty::default()),
      expr_type: DUMMY,
      value_category: RValue,
    }
  }
}

#[derive(Debug)]
pub struct Variable<'context> {
  pub name: SymbolRef<'context>,
  pub span: SourceSpan,
}
impl<'context> Variable<'context> {
  pub fn new(name: SymbolRef<'context>, span: SourceSpan) -> Self {
    Self { name, span }
  }
}
#[derive(Debug)]
pub struct ImplicitCast<'context> {
  pub expr: Box<Expression<'context>>,
  pub cast_type: CastType,
  pub span: SourceSpan,
}
impl<'context> ImplicitCast<'context> {
  pub fn new(
    expr: Box<Expression<'context>>,
    cast_type: CastType,
    span: SourceSpan,
  ) -> Self {
    Self {
      expr,
      cast_type,
      span,
    }
  }
}
/// assignment-expression:
///    - conditional-expression
///    - unary-expression assignment-operator assignment-expression
#[derive(Debug)]
pub struct Assignment<'context> {
  pub operator: Operator,
  pub left: Box<Expression<'context>>,
  pub right: Box<Expression<'context>>,
  pub span: SourceSpan,
}
impl<'context> Assignment<'context> {
  pub fn from_operator(
    operator: Operator,
    left: Expression<'context>,
    right: Expression<'context>,
    span: SourceSpan,
  ) -> Option<Self> {
    match operator.category() {
      OperatorCategory::Assignment => Some(Self {
        operator,
        left: Box::new(left),
        right: Box::new(right),
        span,
      }),
      _ => None,
    }
  }

  pub fn new(
    operator: Operator,
    left: Expression<'context>,
    right: Expression<'context>,
    span: SourceSpan,
  ) -> Self {
    Self::from_operator(operator, left, right, span).unwrap()
  }
}

impl<'context> Expression<'context> {
  /// 6.6.8: An integer constant expression shall have integer type and shall only have operands that are
  ///           integer constants, named and compound literal constants of integer type, character constants,
  ///           sizeof expressions whose results are integer constants, alignof expressions, and floating, named,
  ///           or compound literal constants of arithmetic type that are the immediate operands of casts. Cast
  ///           operators in an integer constant expression shall only convert arithmetic types to integer types,
  ///           except as part of an operand to the typeof operators, sizeof operator, or alignof operator.
  pub fn is_integer_constant(&self) -> bool {
    match self.raw_expr() {
      RawExpr::Constant(c) => c.is_integral() || c.is_char_array(),
      RawExpr::Variable(variable) =>
        Self::is_named_integer_constant_unchecked(variable),
      // either be folded or not an integer constant expression
      RawExpr::Empty(_)
      | RawExpr::Unary(_)
      | RawExpr::Binary(_)
      | RawExpr::Call(_)
      | RawExpr::Paren(_)
      | RawExpr::MemberAccess(_)
      | RawExpr::Ternary(_)
      | RawExpr::SizeOf(_)
      | RawExpr::CStyleCast(_)
      | RawExpr::ArraySubscript(_)
      | RawExpr::CompoundLiteral(_)
      | RawExpr::ImplicitCast(_)
      | RawExpr::Assignment(_) => false,
    }
  }

  // todo: enum constant
  fn is_named_integer_constant(&self) -> bool {
    match self.raw_expr() {
      RawExpr::Variable(variable) =>
        Self::is_named_integer_constant_unchecked(variable),
      _ => false,
    }
  }

  fn is_named_integer_constant_unchecked(
    variable: &Variable<'context>,
  ) -> bool {
    let sym = variable.name.borrow();

    (sym.qualified_type.unqualified_type.is_integer()
      || sym.qualified_type.unqualified_type.as_array().is_some())
      && matches!(sym.storage_class, Storage::Constexpr)
  }

  /// 6.6.7
  pub fn is_named_constant(&self) -> bool {
    self.is_named_integer_constant() // this is incorrect, but ill leave it for now
  }

  /// 6.6.11: An address constant is a null pointer, a pointer to an lvalue designating an object of static storage
  ///   duration, or a pointer to a function designator; it shall be created explicitly using the unary `&` operator
  ///   or an integer constant cast to pointer type, or implicitly using an expression of array or function type.
  pub fn is_address_constant(&self) -> bool {
    match self.raw_expr() {
      RawExpr::Constant(c) => c.is_nullptr() || c.is_address(),
      RawExpr::Unary(unary) if self.unqualified_type().is_pointer() =>
        unary.operand.is_lvalue()
          || matches!(unary.operand.unqualified_type(), Type::FunctionProto(_))
          || matches!(unary.operand.raw_expr(),
          RawExpr::Variable(var) if var.name.borrow().storage_class.is_static()),
      _ => false,
    }
  }

  /// 6.6.13: A structure or union constant is a named constant or compound literal constant with structure or union type, respectively.
  pub fn struct_or_union_constant(&self) -> bool {
    todo!()
  }

  /// 6.6.6
  pub fn compound_literal_constant(&self) -> bool {
    todo!()
  }
}

mod fmt {

  use ::std::fmt::Display;

  use super::{Assignment, Expression, ImplicitCast, Variable};

  impl<'context> Display for Assignment<'context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }

  impl<'context> Display for Expression<'context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.raw_expr)
    }
  }
  // the "specialization" for the smart pointer case
  impl<'context> Display for Variable<'context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name.borrow())
    }
  }
  impl<'context> Display for ImplicitCast<'context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.expr)
    }
  }
}

mod test {
  #[test]
  fn int_float() {
    // use ::rcc_utils::{Dummy, IntoWith};

    // use super::*;

    // let int_expr = Expression::new(
    //   RawExpr::Constant(
    //     ConstantLiteral::Integral(42.into()).into_with(Dummy::dummy()),
    //   ),
    //   QualifiedType::int(),
    //   RValue,
    // );
    // let float_expr = Expression::new(
    //   RawExpr::Constant(
    //     ConstantLiteral::Floating(::std::f32::consts::PI.into())
    //       .into_with(Dummy::dummy()),
    //   ),
    //   QualifiedType::float(),
    //   RValue,
    // );
    // let promoted_expr =
    //   Expression::usual_arithmetic_conversion(int_expr, float_expr)
    //     .unwrap()
    //     .2;
    // // type shall be
    // println!("Promoted expression: {:#?}", promoted_expr);
  }
}
