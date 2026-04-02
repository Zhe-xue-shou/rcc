use ::rcc_ast::{
  Context,
  blueprints::Placeholder,
  types::{CastType, Primitive, QualifiedType, Qualifiers, Type, TypeRef},
};
use ::rcc_shared::{ArenaVec, CollectIn, Operator, SourceSpan, Storage};
use ::rcc_utils::StrRef;

use crate::declref::DeclRef;

pub(super) type UnaryKind = ::rcc_ast::blueprints::UnaryKind;

pub type ExprRef<'c> = &'c Expression<'c>;

pub type Empty = Placeholder;
pub use ::rcc_ast::Constant;

#[derive(Debug, Clone)]
pub enum RawExpr<'c> {
  // no-op for error recovery; for empty expr should use Option<Expression> instead
  Empty(Empty),
  Constant(Constant<'c>),
  Unary(Unary<'c>),
  Binary(Binary<'c>),
  Call(Call<'c>),
  Paren(Paren<'c>),
  MemberAccess(MemberAccess<'c>),
  Ternary(Ternary<'c>),
  SizeOf(SizeOf<'c>),
  CStyleCast(CStyleCast<'c>),
  ArraySubscript(ArraySubscript<'c>),
  CompoundLiteral(CompoundLiteral),
  Variable(Variable<'c>),
  ImplicitCast(ImplicitCast<'c>),
  CompoundAssign(CompoundAssign<'c>),
}

::rcc_utils::interconvert!(Empty, RawExpr<'c>);
::rcc_utils::interconvert!(Constant, RawExpr, 'c);
::rcc_utils::interconvert!(Unary, RawExpr, 'c);
::rcc_utils::interconvert!(Binary, RawExpr, 'c);
::rcc_utils::interconvert!(Call, RawExpr, 'c);
::rcc_utils::interconvert!(Paren, RawExpr, 'c);
::rcc_utils::interconvert!(MemberAccess, RawExpr, 'c);
::rcc_utils::interconvert!(Ternary, RawExpr, 'c);
::rcc_utils::interconvert!(SizeOf, RawExpr, 'c);
::rcc_utils::interconvert!(CStyleCast, RawExpr, 'c);
::rcc_utils::interconvert!(ArraySubscript, RawExpr, 'c);
::rcc_utils::interconvert!(CompoundLiteral, RawExpr<'c>);
::rcc_utils::interconvert!(Variable, RawExpr, 'c);
::rcc_utils::interconvert!(ImplicitCast, RawExpr, 'c);
::rcc_utils::interconvert!(CompoundAssign, RawExpr, 'c);

::rcc_utils::make_trio_for!(Empty, RawExpr<'c>);
::rcc_utils::make_trio_for!(Constant, RawExpr, 'c);
::rcc_utils::make_trio_for!(Unary, RawExpr, 'c);
::rcc_utils::make_trio_for!(Binary, RawExpr, 'c);
::rcc_utils::make_trio_for!(Call, RawExpr, 'c);
::rcc_utils::make_trio_for!(Paren, RawExpr, 'c);
::rcc_utils::make_trio_for!(MemberAccess, RawExpr, 'c);
::rcc_utils::make_trio_for!(Ternary, RawExpr, 'c);
::rcc_utils::make_trio_for!(SizeOf, RawExpr, 'c);
::rcc_utils::make_trio_for!(CStyleCast, RawExpr, 'c);
::rcc_utils::make_trio_for!(ArraySubscript, RawExpr, 'c);
::rcc_utils::make_trio_for!(CompoundLiteral, RawExpr<'c>);
::rcc_utils::make_trio_for!(Variable, RawExpr, 'c);
::rcc_utils::make_trio_for!(ImplicitCast, RawExpr, 'c);
::rcc_utils::make_trio_for!(CompoundAssign, RawExpr, 'c);

#[derive(Debug, Clone)]
pub struct Unary<'c> {
  pub operator: Operator,
  pub operand: ExprRef<'c>,
  pub kind: UnaryKind,
}

#[derive(Debug, Clone)]
pub struct Binary<'c> {
  pub operator: Operator,
  pub left: ExprRef<'c>,
  pub right: ExprRef<'c>,
}

#[derive(Debug, Clone)]
pub struct Call<'c> {
  pub callee: ExprRef<'c>,
  pub arguments: &'c [ExprRef<'c>],
}

#[derive(Debug, Clone)]
pub struct Paren<'c> {
  pub expr: ExprRef<'c>,
}

#[derive(Debug, Clone)]
pub struct MemberAccess<'c> {
  pub object: ExprRef<'c>,
  pub member: StrRef<'c>,
}

#[derive(Debug, Clone)]
pub struct Ternary<'c> {
  pub condition: ExprRef<'c>,
  pub then_expr: Option<ExprRef<'c>>,
  pub else_expr: ExprRef<'c>,
}

#[derive(Debug, Clone)]
pub enum SizeOfKind<'c> {
  Type(QualifiedType<'c>),
  Expression(ExprRef<'c>),
}

#[derive(Debug, Clone)]
pub struct SizeOf<'c> {
  pub sizeof: SizeOfKind<'c>,
}

#[derive(Debug, Clone)]
pub struct CStyleCast<'c> {
  pub expr: ExprRef<'c>,
}

#[derive(Debug, Clone)]
pub struct ArraySubscript<'c> {
  pub array: ExprRef<'c>,
  pub index: ExprRef<'c>,
}

#[derive(Debug, Clone)]
pub struct CompoundLiteral {}

impl<'c> Unary<'c> {
  pub fn new(
    operator: Operator,
    operand: ExprRef<'c>,
    kind: UnaryKind,
  ) -> Self {
    debug_assert!(operator.unary(), "not a unary operator! got {:?}", operator);
    Self {
      operator,
      operand,
      kind,
    }
  }

  #[inline(always)]
  pub fn prefix(operator: Operator, operand: ExprRef<'c>) -> Self {
    debug_assert!(operator.unary(), "not a unary operator! got {:?}", operator);
    Self::new(operator, operand, UnaryKind::Prefix)
  }

  #[inline(always)]
  pub fn postfix(operator: Operator, operand: ExprRef<'c>) -> Self {
    debug_assert!(operator.unary(), "not a unary operator! got {:?}", operator);
    Self::new(operator, operand, UnaryKind::Postfix)
  }
}

impl<'c> Binary<'c> {
  pub fn from_operator(
    operator: Operator,
    left: ExprRef<'c>,
    right: ExprRef<'c>,
  ) -> Option<Self> {
    match operator.binary() {
      true => Some(Self {
        operator,
        left,
        right,
      }),
      false => None,
    }
  }

  pub fn from_operator_unchecked(
    operator: Operator,
    left: ExprRef<'c>,
    right: ExprRef<'c>,
  ) -> Self {
    debug_assert!(operator.binary());
    Self {
      operator,
      left,
      right,
    }
  }

  #[inline]
  pub fn new(
    operator: Operator,
    left: ExprRef<'c>,
    right: ExprRef<'c>,
  ) -> Self {
    Self::from_operator(operator, left, right)
      .expect("not a binary operator! use from_operator instead")
  }
}

impl<'c> Ternary<'c> {
  pub fn new(
    condition: ExprRef<'c>,
    then_expr: ExprRef<'c>,
    else_expr: ExprRef<'c>,
  ) -> Self {
    Self {
      condition,
      then_expr: Some(then_expr),
      else_expr,
    }
  }

  pub fn elvis(condition: ExprRef<'c>, else_expr: ExprRef<'c>) -> Self {
    Self {
      condition,
      then_expr: None,
      else_expr,
    }
  }

  #[inline]
  pub fn is_elvis(&self) -> bool {
    self.then_expr.is_some()
  }
}

impl<'c> ArraySubscript<'c> {
  pub fn new(array: ExprRef<'c>, index: ExprRef<'c>) -> Self {
    Self { array, index }
  }
}

impl<'c> SizeOf<'c> {
  pub fn new(sizeof: SizeOfKind<'c>) -> Self {
    Self { sizeof }
  }
}

impl<'c> Call<'c> {
  pub fn new<I>(
    context: &'c Context<'c>,
    callee: ExprRef<'c>,
    arguments: I,
  ) -> Self
  where
    I: IntoIterator<Item = ExprRef<'c>>,
  {
    let arguments = arguments
      .into_iter()
      .collect_in::<ArenaVec<_>>(context.arena())
      .into_bump_slice();
    Self { callee, arguments }
  }
}

impl<'c> Paren<'c> {
  pub fn new(expr: ExprRef<'c>) -> Self {
    Self { expr }
  }
}

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
  span: SourceSpan,
}

::rcc_utils::ensure_is_pod!(RawExpr<'_>);
::rcc_utils::ensure_is_pod!(Expression<'_>);

impl<'c> Expression<'c> {
  pub fn new(
    context: &'c Context<'c>,
    variant: impl Into<RawExpr<'c>>,
    expr_type: QualifiedType<'c>,
    value_category: ValueCategory,
    span: SourceSpan,
  ) -> ExprRef<'c> {
    context.arena().alloc(Self {
      raw_expr: variant.into(),
      expr_type,
      value_category,
      span,
    })
  }

  pub fn new_rvalue(
    context: &'c Context<'c>,
    variant: impl Into<RawExpr<'c>>,
    expr_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ExprRef<'c> {
    context.arena().alloc(Self {
      raw_expr: variant.into(),
      expr_type,
      value_category: RValue,
      span,
    })
  }

  pub fn new_lvalue(
    context: &'c Context<'c>,
    variant: impl Into<RawExpr<'c>>,
    expr_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> ExprRef<'c> {
    context.arena().alloc(Self {
      raw_expr: variant.into(),
      expr_type,
      value_category: LValue,
      span,
    })
  }

  pub fn new_error_node(
    context: &'c Context<'c>,
    expr_type: QualifiedType<'c>,
  ) -> ExprRef<'c> {
    context.arena().alloc(Self {
      raw_expr: RawExpr::Empty(Empty::default()),
      expr_type,
      value_category: RValue,
      ..Default::default()
    })
  }

  pub fn unqualified_type(&self) -> TypeRef<'c> {
    self.expr_type.unqualified_type
  }

  pub fn qualifiers(&self) -> Qualifiers {
    self.expr_type.qualifiers
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
    self.span
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
      span: Default::default(),
    }
  }
}

#[derive(Debug, Clone)]
pub struct Variable<'c> {
  pub declaration: DeclRef<'c>,
}

impl<'c> Variable<'c> {
  pub fn new(declaration: DeclRef<'c>) -> Self {
    Self { declaration }
  }
}

#[derive(Debug, Clone)]
pub struct ImplicitCast<'c> {
  pub expr: ExprRef<'c>,
  pub cast_type: CastType,
}

impl<'c> ImplicitCast<'c> {
  pub fn new(expr: ExprRef<'c>, cast_type: CastType) -> Self {
    Self { expr, cast_type }
  }
}

/// TODO: reduce the size of this struct.
#[derive(Debug, Clone)]
pub struct CompoundAssign<'c> {
  pub operator: Operator,
  pub left: ExprRef<'c>,
  pub right: ExprRef<'c>,
  /// the type of the left operand which underwent conversions as if it were the left operand of a [`Binary`].
  ///
  /// Called [`ComputationLHSType`](https://github.com/llvm/llvm-project/blob/23eec1216993f599f90e259e339228ba8b69c58a/clang/include/clang/AST/RawExpr.h#L4304) in clang's AST.
  pub intermediate_left_type: QualifiedType<'c>,
  /// the type of the result of the computation of the left and right as if they were the operands of a [`Binary`].
  ///
  /// Also called [`ComputationResultType`](https://github.com/llvm/llvm-project/blob/23eec1216993f599f90e259e339228ba8b69c58a/clang/include/clang/AST/RawExpr.h#L4305) in clang.
  pub intermediate_result_type: QualifiedType<'c>,
}

impl<'c> CompoundAssign<'c> {
  pub fn new(
    operator: Operator,
    left: ExprRef<'c>,
    right: ExprRef<'c>,
    intermediate_left_type: QualifiedType<'c>,
    intermediate_result_type: QualifiedType<'c>,
  ) -> Self {
    Self {
      operator,
      left,
      right,
      intermediate_result_type,
      intermediate_left_type,
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
    let declaration = variable.declaration;

    (declaration.qualified_type().unqualified_type.is_integer()
      || declaration.qualified_type().unqualified_type.is_array())
      && matches!(declaration.storage_class(), Storage::Constexpr)
  }

  /// 6.6.7
  pub fn is_named_constant(&self) -> bool {
    self.is_named_integer_constant() // this is incorrect, but ill leave it for now
  }

  /// 6.6.11: An address constant is a null pointer, a pointer to an lvalue designating an object of static storage
  /// duration, or a pointer to a function designator.
  pub fn is_address_constant(&self) -> bool {
    match self.raw_expr() {
      RawExpr::Constant(c) => c.is_nullptr(),
      RawExpr::Unary(unary) if self.unqualified_type().is_pointer() =>
        unary.operand.is_lvalue()
          || matches!(unary.operand.unqualified_type(), Type::FunctionProto(_))
          || matches!(unary.operand.raw_expr(),
          RawExpr::Variable(var) if var.declaration.storage_class().is_static()),
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

  use super::*;

  impl<'c> Display for RawExpr<'c> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
      ::rcc_utils::static_dispatch!(
        self,
        |variant| variant.fmt(f) =>
        Empty Constant Unary Binary Call Paren MemberAccess Ternary SizeOf
        CStyleCast ArraySubscript CompoundLiteral Variable ImplicitCast CompoundAssign
      )
    }
  }

  impl<'c> Display for Call<'c> {
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

  impl<'c> Display for Unary<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      if matches!(self.operator, Operator::PlusPlus | Operator::MinusMinus) {
        write!(f, "({} {}{})", self.operand, self.kind, self.operator)
      } else {
        write!(f, "({} {})", self.operand, self.operator)
      }
    }
  }

  impl<'c> Display for Binary<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({} {} {})", self.left, self.right, self.operator)
    }
  }

  impl<'c> Display for Ternary<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match &self.then_expr {
        Some(then_expr) => write!(
          f,
          "({} ? {} : {})",
          self.condition, then_expr, self.else_expr
        ),
        None => write!(f, "({} ?: {})", self.condition, self.else_expr),
      }
    }
  }

  impl<'c> Display for Paren<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({})", self.expr)
    }
  }

  impl<'c> Display for SizeOf<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.sizeof)
    }
  }

  impl<'c> Display for SizeOfKind<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        SizeOfKind::Type(typ) => write!(f, "sizeof({})", typ),
        SizeOfKind::Expression(expr) => write!(f, "sizeof({})", expr),
      }
    }
  }

  impl<'c> Display for ArraySubscript<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "(({})[{}])", self.array, self.index)
    }
  }

  impl Display for CompoundLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "<compound literal - not implemented>")
    }
  }

  impl<'c> Display for CStyleCast<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "<C-style cast - not implemented>")
    }
  }

  impl<'c> Display for MemberAccess<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "({}.{})", self.object, self.member)
    }
  }

  impl<'c> Display for Expression<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.raw_expr)
    }
  }

  // the specialization for the smart pointer case
  impl<'c> Display for Variable<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.declaration)
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
