use ::rc_utils::{Dummy, IntoWith};

use self::ValueCategory::{LValue, RValue};
use crate::{
  common::{Operator, OperatorCategory, SourceSpan, Storage, SymbolRef},
  diagnosis::Diag,
  type_alias_expr,
  types::{
    CastType::{self, *},
    Primitive, QualifiedType, Qualifiers, Type, TypeInfo,
  },
};

type_alias_expr! {Expression, QualifiedType, Variable ImplicitCast Assignment}
#[derive(Debug, Clone, Copy, ::strum_macros::Display, PartialEq)]
pub enum ValueCategory {
  #[strum(serialize = "lvalue")]
  LValue,
  /// 6.3.2: "rvalue" is in this document described as the "value of an expression".
  ///        which, is different from the one defined in C++ standard.
  #[strum(serialize = "rvalue")]
  RValue,
}

#[derive(Debug)]
pub struct Expression {
  raw_expr: RawExpr,
  expr_type: QualifiedType,
  value_category: ValueCategory,
}
impl Expression {
  pub fn new(
    raw_expr: RawExpr,
    expr_type: QualifiedType,
    value_category: ValueCategory,
  ) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category,
    }
  }

  pub fn new_rvalue(raw_expr: RawExpr, expr_type: QualifiedType) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category: RValue,
    }
  }

  pub fn new_lvalue(raw_expr: RawExpr, expr_type: QualifiedType) -> Self {
    Self {
      raw_expr,
      expr_type,
      value_category: LValue,
    }
  }

  pub fn new_error_node(expr_type: QualifiedType) -> Self {
    Self {
      raw_expr: RawExpr::Empty(Empty::default()),
      expr_type,
      value_category: RValue,
    }
  }

  pub fn unqualified_type(&self) -> &Type {
    self.expr_type.unqualified_type()
  }

  pub fn qualifiers(&self) -> &Qualifiers {
    self.expr_type.qualifiers()
  }

  pub fn qualified_type(&self) -> &QualifiedType {
    &self.expr_type
  }

  pub fn raw_expr(&self) -> &RawExpr {
    &self.raw_expr
  }

  pub fn value_category(&self) -> ValueCategory {
    self.value_category
  }

  pub(super) fn destructure(self) -> (RawExpr, QualifiedType, ValueCategory) {
    (self.raw_expr, self.expr_type, self.value_category)
  }

  pub(super) fn into_raw(self) -> RawExpr {
    self.raw_expr
  }
}
impl TryFrom<Expression> for usize {
  type Error = Diag;

  fn try_from(value: Expression) -> Result<Self, Self::Error> {
    match value.raw_expr {
      RawExpr::Constant(c) =>
        Self::try_from(c.constant).map_err(|m| m.into_with(c.span)),
      _ => todo!(),
    }
  }
}
impl Primitive {
  #[must_use]
  pub fn common_type(lhs: &Self, rhs: &Self) -> (Self, CastType, CastType) {
    // If both operands have the same type, then no further conversion is needed.
    // first: _Decimal types ignored
    // also, complex types ignored
    if lhs == rhs {
      return (lhs.clone(), Noop, Noop);
    }
    if matches!(lhs, Self::Void | Self::Nullptr)
      || matches!(rhs, Self::Void | Self::Nullptr)
    {
      panic!("Invalid types for common type: {:?}, {:?}", lhs, rhs);
    }
    // otherwise, if either operand is of some floating type, the other operand is converted to it.
    // Otherwise, if any of the two types is an enumeration, it is converted to its underlying type. - handled upstream
    match (lhs.is_floating_point(), rhs.is_floating_point()) {
      (true, false) => (lhs.clone(), Noop, IntegralToFloating),
      (false, true) => (rhs.clone(), IntegralToFloating, Noop),
      (true, true) => Self::common_floating_rank(lhs.clone(), rhs.clone()),
      (false, false) => Self::common_integer_rank(lhs.clone(), rhs.clone()),
    }
  }

  #[must_use]
  fn common_floating_rank(lhs: Self, rhs: Self) -> (Self, CastType, CastType) {
    assert!(lhs.is_floating_point() && rhs.is_floating_point());
    if lhs.floating_rank() > rhs.floating_rank() {
      (lhs, Noop, FloatingCast)
    } else {
      (rhs, FloatingCast, Noop)
    }
  }

  #[must_use]
  fn common_integer_rank(lhs: Self, rhs: Self) -> (Self, CastType, CastType) {
    assert!(lhs.is_integer() && rhs.is_integer());

    let (lhs, _) = lhs.integer_promotion();
    let (rhs, _) = rhs.integer_promotion();
    if lhs == rhs {
      // done
      return (lhs, Noop, Noop);
    }
    if lhs.is_unsigned() == rhs.is_unsigned() {
      return if lhs.integer_rank() > rhs.integer_rank() {
        (lhs, Noop, IntegralCast)
      } else {
        (rhs, IntegralCast, Noop)
      };
    }
    fn signed_and_unsigned(
      lhs: Primitive,
      rhs: Primitive,
    ) -> (Primitive, CastType, CastType) {
      debug_assert!(!lhs.is_unsigned());
      debug_assert!(rhs.is_unsigned());
      if lhs.integer_rank() >= rhs.integer_rank() {
        (lhs, Noop, IntegralCast)
      } else if rhs.size() > lhs.size() {
        (rhs, IntegralCast, Noop)
      } else {
        // if the signed type cannot represent all values of the unsigned type, return the unsigned version of the signed type
        // the signed type is always larger than the corresponding unsigned type on my x86_64 architecture
        // so this branch is unlikely to be taken
        let promoted_rhs = rhs.into_unsigned();
        (promoted_rhs, IntegralCast, IntegralCast)
      }
    }

    if lhs.is_unsigned() {
      signed_and_unsigned(rhs, lhs)
    } else {
      signed_and_unsigned(lhs, rhs)
    }
  }
}
impl Expression {
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

impl ::core::default::Default for Expression {
  fn default() -> Self {
    Self {
      raw_expr: RawExpr::Empty(Empty::default()),
      expr_type: Type::void().into(),
      value_category: RValue,
    }
  }
}
impl Dummy for Expression {
  fn dummy() -> Self {
    Self::default()
  }
}
#[derive(Debug)]
pub struct Variable {
  pub name: SymbolRef,
  pub span: SourceSpan,
}
impl Variable {
  pub fn new(name: SymbolRef, span: SourceSpan) -> Self {
    Self { name, span }
  }
}
#[derive(Debug)]
pub struct ImplicitCast {
  pub expr: Box<Expression>,
  pub cast_type: CastType,
  pub span: SourceSpan,
}
impl ImplicitCast {
  pub fn new(
    expr: Box<Expression>,
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
pub struct Assignment {
  pub operator: Operator,
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub span: SourceSpan,
}
impl Assignment {
  pub fn from_operator(
    operator: Operator,
    left: Expression,
    right: Expression,
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
    left: Expression,
    right: Expression,
    span: SourceSpan,
  ) -> Self {
    Self::from_operator(operator, left, right, span).unwrap()
  }
}

impl Expression {
  /// 6.6.8: An integer constant expression shall have integer type and shall only have operands that are
  ///           integer constants, named and compound literal constants of integer type, character constants,
  ///           sizeof expressions whose results are integer constants, alignof expressions, and floating, named,
  ///           or compound literal constants of arithmetic type that are the immediate operands of casts. Cast
  ///           operators in an integer constant expression shall only convert arithmetic types to integer types,
  ///           except as part of an operand to the typeof operators, sizeof operator, or alignof operator.
  pub fn is_integer_constant(&self) -> bool {
    match self.raw_expr() {
      RawExpr::Constant(c) => c.is_integer() || c.is_char_array(),
      // ignore VLA
      RawExpr::SizeOf(sizeof) =>
        if let SizeOfKind::Expression(e) = &sizeof.sizeof {
          e.unqualified_type().is_integer()
        } else {
          true // sizeof(type) is always constant
        },
      RawExpr::CStyleCast(cast) => cast.expr.is_integer_constant(),
      RawExpr::Unary(unary) =>
        matches!(unary.operator, Operator::Plus | Operator::Minus)
          && unary.operand.is_integer_constant(),
      RawExpr::Variable(variable) =>
        Self::is_named_integer_constant_unchecked(variable),
      _ => false,
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

  fn is_named_integer_constant_unchecked(variable: &Variable) -> bool {
    let sym = variable.name.borrow();

    (sym.qualified_type.unqualified_type().is_integer()
      || sym.qualified_type.unqualified_type().as_array().is_some())
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
      RawExpr::Constant(c) => c.is_nullptr(),
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

  impl Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }

  impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.raw_expr)
    }
  }
  // the "specialization" for the smart pointer case
  impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.name.borrow())
    }
  }
  impl Display for ImplicitCast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.expr)
    }
  }
}

mod test {

  #[test]
  fn int_float() {
    use super::*;

    let int_expr = Expression::new(
      RawExpr::Constant(ConstantLiteral::Int(42).into()),
      QualifiedType::int(),
      RValue,
    );
    let float_expr = Expression::new(
      RawExpr::Constant(ConstantLiteral::Float(::std::f32::consts::PI).into()),
      QualifiedType::float(),
      RValue,
    );
    let promoted_expr =
      Expression::usual_arithmetic_conversion(int_expr, float_expr)
        .unwrap()
        .2;
    // type shall be
    println!("Promoted expression: {:#?}", promoted_expr);
  }
}
