use ::rcc_ast::{
  SymbolRef, type_alias_expr,
  types::{CastType, Primitive, QualifiedType, Qualifiers, Type, TypeRef},
};
use ::rcc_shared::{Operator, SourceSpan, Storage};

type_alias_expr! {Expression<'c>, QualifiedType<'c>, Variable<'c> ImplicitCast<'c> CompoundAssign<'c> #[derive(Debug, Clone)]}
pub(super) type UnaryKind = ::rcc_ast::blueprints::UnaryKind;

#[derive(Debug, Clone, Copy, ::strum_macros::Display, PartialEq)]
pub enum ValueCategory {
  LValue,
  /// 6.3.2: "rvalue" is in this document described as the "value of an expression".
  ///        which, is different from the one defined in C++ standard.
  RValue,
}
use ValueCategory::{LValue, RValue};

#[derive(Debug, Clone)]
pub struct Expression<'c> {
  raw_expr: RawExpr<'c>,
  expr_type: QualifiedType<'c>,
  value_category: ValueCategory,
}
impl<'c> Expression<'c> {
  pub fn new(
    variant: impl Into<RawExpr<'c>>,
    expr_type: QualifiedType<'c>,
    value_category: ValueCategory,
  ) -> Self {
    Self {
      raw_expr: variant.into(),
      expr_type,
      value_category,
    }
  }

  pub fn new_rvalue(
    variant: impl Into<RawExpr<'c>>,
    expr_type: QualifiedType<'c>,
  ) -> Self {
    Self {
      raw_expr: variant.into(),
      expr_type,
      value_category: RValue,
    }
  }

  pub fn new_lvalue(
    variant: impl Into<RawExpr<'c>>,
    expr_type: QualifiedType<'c>,
  ) -> Self {
    Self {
      raw_expr: variant.into(),
      expr_type,
      value_category: LValue,
    }
  }

  pub fn new_error_node(expr_type: QualifiedType<'c>) -> Self {
    Self {
      raw_expr: RawExpr::Empty(Empty::default()),
      expr_type,
      value_category: RValue,
    }
  }

  pub fn unqualified_type(&self) -> TypeRef<'c> {
    self.expr_type.unqualified_type
  }

  pub fn qualifiers(&self) -> &Qualifiers {
    &self.expr_type.qualifiers
  }

  pub fn qualified_type(&self) -> &QualifiedType<'c> {
    &self.expr_type
  }

  pub fn raw_expr(&self) -> &RawExpr<'c> {
    &self.raw_expr
  }

  pub fn value_category(&self) -> ValueCategory {
    self.value_category
  }

  pub fn destructure(self) -> (RawExpr<'c>, QualifiedType<'c>, ValueCategory) {
    (self.raw_expr, self.expr_type, self.value_category)
  }
}

impl<'c> Expression<'c> {
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

impl<'c> ::core::default::Default for Expression<'c> {
  fn default() -> Self {
    const DUMMY_UNQUAL: TypeRef<'static> = &Type::Primitive(Primitive::Void);
    const DUMMY: QualifiedType<'static> = QualifiedType {
      qualifiers: Qualifiers::empty(),
      unqualified_type: DUMMY_UNQUAL,
    };
    Self {
      raw_expr: RawExpr::Empty(Empty::default()),
      expr_type: DUMMY,
      value_category: RValue,
    }
  }
}

#[derive(Debug, Clone)]
pub struct Variable<'c> {
  pub symbol: SymbolRef<'c>,
  pub span: SourceSpan,
}
impl<'c> Variable<'c> {
  pub fn new(name: SymbolRef<'c>, span: SourceSpan) -> Self {
    Self { symbol: name, span }
  }
}
#[derive(Debug, Clone)]
pub struct ImplicitCast<'c> {
  pub expr: Box<Expression<'c>>,
  pub cast_type: CastType,
  pub span: SourceSpan,
}
impl<'c> ImplicitCast<'c> {
  pub fn new(
    expr: Box<Expression<'c>>,
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
/// TODO: reduce the size of this struct.
#[derive(Debug, Clone)]
pub struct CompoundAssign<'c> {
  pub operator: Operator,
  pub left: Box<Expression<'c>>,
  pub right: Box<Expression<'c>>,
  /// the type of the left operand which underwent conversions as if it were the left operand of a [`Binary`].
  ///
  /// Called [`ComputationLHSType`](https://github.com/llvm/llvm-project/blob/23eec1216993f599f90e259e339228ba8b69c58a/clang/include/clang/AST/Expr.h#L4304) in clang's AST.
  pub intermediate_left_type: QualifiedType<'c>,
  /// the type of the result of the computation of the left and right as if they were the operands of a [`Binary`].
  ///
  /// Also called [`ComputationResultType`](https://github.com/llvm/llvm-project/blob/23eec1216993f599f90e259e339228ba8b69c58a/clang/include/clang/AST/Expr.h#L4305) in clang.
  pub intermediate_result_type: QualifiedType<'c>,
  pub span: SourceSpan,
}

impl<'c> CompoundAssign<'c> {
  pub fn new(
    operator: Operator,
    left: Expression<'c>,
    right: Expression<'c>,
    intermediate_left_type: QualifiedType<'c>,
    intermediate_result_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> Self {
    Self {
      operator,
      left: left.into(),
      right: right.into(),
      intermediate_result_type,
      intermediate_left_type,
      span,
    }
  }

  #[inline]
  pub fn associated_operator(&self) -> Operator {
    self.operator.associated_operator().unwrap()
  }
}
impl<'c> Expression<'c> {
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

  fn is_named_integer_constant_unchecked(variable: &Variable<'c>) -> bool {
    let sym = variable.symbol.borrow();

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
          RawExpr::Variable(var) if var.symbol.borrow().storage_class.is_static()),
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

  use super::{CompoundAssign, Expression, ImplicitCast, Variable};

  impl<'c> Display for Expression<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.raw_expr)
    }
  }
  // the "specialization" for the smart pointer case
  impl<'c> Display for Variable<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.symbol.borrow())
    }
  }
  impl<'c> Display for ImplicitCast<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.expr)
    }
  }
  impl<'c> Display for CompoundAssign<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }
}
