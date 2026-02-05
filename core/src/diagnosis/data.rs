//! Diagnostic data structures.
//!
//! Diagnostic and their corresponding messages are separated
//! in order to reduce the cyclic dependency and
//! chain-operation (like .a().b()...) as much as possible.
//!
//! The main struct [`Diag`] contains a [`SourceSpan`] and [`Meta`].
//! The [`Meta`] contains a [`Severity`] and [`Data`].
//! The [`Data`] enum contains all the diagnostic messages.
//!
//! A [`Data`] can be easily converted into a [`Meta`] with a given [`Severity`], like
//! ```rust
//! use ::rc_utils::IntoWith;
//! use ::rc_core::diagnosis::{Data, Meta, Severity};
//! let data = Data::MissingIdentifier("after type specifier".to_string());
//! let meta: Meta = data.into_with(Severity::Error);
//! let diag = meta.into_with(Default::default()); // default span
//! ```
//!
//! TODO: 1. make [`Data`] smaller. 2. consider alternatives of [`Dummy`] trait for testing.

/// Diagnostic message with [`SourceSpan`].
///
/// See module documentation for details.
#[derive(Debug)]
pub struct Diag {
  pub(crate) metadata: Meta,
  pub(crate) span: SourceSpan,
}
/// Message with [`Severity`].
#[derive(Debug)]
pub struct Meta {
  pub(crate) severity: Severity,
  pub(crate) data: Data,
}
impl Meta {
  pub fn new(severity: Severity, data: Data) -> Self {
    Self { severity, data }
  }
}
impl IntoWith<SourceSpan, Diag> for Meta {
  #[inline]
  fn into_with(self, span: SourceSpan) -> Diag {
    Diag::new(span, self.severity, self.data)
  }
}
#[derive(Debug, Clone, Copy, ::strum_macros::Display)]
pub enum Severity {
  Info,
  Warning,
  Error,
}

use ::rc_utils::{DisplayWith, IntoWith, static_assert};

use crate::{
  common::{Keyword, Literal, Operator, SourceManager, SourceSpan, Storage},
  types::{Constant, QualifiedType, Qualifiers},
};

/// Custom message. would be printed as-is.
type CustomMessage = String;
/// Fixed custom message.
type CustomMsgFixed = &'static str;
/// Element, like `expect ')' after <elem>`
type Elem = String;

/// Plain error/warning/other diagnostic messages.
///
/// TODO: reduce the size of this enum.
#[derive(Debug, ::thiserror::Error)]
pub enum Data {
  #[error("Unexpected character '{}'{expected}", &.0.0, expected = format_expected(&.0.1))]
  UnexpectedCharacter(Box<(Literal, Option<Literal>)>),
  #[error("Unterminated string literal")]
  UnterminatedString,
  #[error("Invalid number format '{0}'")]
  InvalidNumberFormat(String),
  #[error("Expect '{0}'")]
  MissingOperator(Operator),
  #[error("Cannot combine storage classes '{0}' and '{1}'")]
  MultipleStorageSpecs(Storage, Storage),
  #[error("Expect a type specifier in declaration, default to 'int'")]
  MissingTypeSpecifier,
  #[error("Expect identifier in declarator")]
  MissingIdentifier(CustomMessage),
  #[error("{0}")]
  ExtraneousComma(CustomMsgFixed),
  #[error("{0}")]
  VoidVariableDecl(CustomMessage),
  #[error("Storage class specifier '{0}' is not allowed here")]
  ExtraneousStorageSpecs(Storage),
  #[error("{0}")]
  UnclosedParameterList(CustomMessage),
  #[error("Expect '(' after {0}")]
  MissingOpenParen(Literal),
  #[error("Expect ')' after {0}")]
  MissingCloseParen(Literal),
  #[error("{0}")]
  ExprNotConstant(CustomMessage),
  #[error("{0}")]
  VarDeclUnclosed(CustomMessage),
  #[error("Block definition is not allowed here")]
  InvalidBlockItem,
  #[error("Expect function name")]
  MissingFunctionName,
  #[error("{0}")]
  InvalidStmt(CustomMessage),
  #[error("Case label cannot appear after default label")]
  CaseLabelAfterDefault,
  #[error("Multiple default labels in one switchl ignoring the latter")]
  MultipleDefaultLabels,
  #[error("Expect at least one case or default label in switch")]
  MissingLabelInSwitch,
  #[error("{0} label not within switch")]
  LabelNotWithinSwitch(Keyword),
  #[error("Label cannot appear at top level")]
  TopLevelLabel,
  #[error("Expect label identifier after goto")]
  MissingLabelAfterGoto,
  #[error("{0}")]
  InvalidControlFlowStmt(CustomMessage),
  #[error("Label '{0}' not found")]
  LabelNotFound(Elem),
  #[error("Variable '{0}' cannot have function specifiers")]
  FunctionSpecsInVariableDecl(Elem),
  #[error("Variable '{0}' already defined")]
  VariableAlreadyDefined(Elem),
  #[error("Function '{0}' already defined")]
  FunctionAlreadyDefined(Elem),
  #[error("Local extern variable '{0}' cannot have initializer")]
  LocalExternVarWithInitializer(Elem),
  #[error("expression '{0}' is not callable")]
  InvalidCallee(QualifiedType),
  #[error("'{0}' is not a variable")]
  NotVariable(Elem),
  #[error("Variable '{0}' is not defined")]
  UndefinedVariable(Elem),
  #[error("Incompatible types '{0}' and '{1}' in ternary expression")]
  TenaryTypeIncompatible(Elem, Elem),
  #[error("Operand of unary operator '{0}' must be arithmetic type, got '{1}'")]
  NonArithmeticInUnaryOp(Operator, Elem),
  #[error(
    "Operands of binary operator '{2}' must be arithmetic types, got '{0}' \
     and '{1}'"
  )]
  NonArithmeticInBinaryOp(Elem, Elem, Operator),
  #[error("Operand of bitwise operator '{0}' must be integer type, got '{1}'")]
  NonIntegerInBitwiseUnaryOp(Operator, Elem),
  #[error(
    "Operands of bitwise operator '{2}' must be integer types, got '{0}' and \
     '{1}'"
  )]
  NonIntegerInBitwiseBinaryOp(Elem, Elem, Operator),
  #[error(
    "Operands of bitshift operator '{2}' must be integer types, got '{0}' and \
     '{1}'"
  )]
  NonIntegerInBitshiftOp(Elem, Elem, Operator),
  #[error("Array subscript is not an integer, got '{0}'")]
  NonIntegerInArraySubscript(Elem),
  #[error("'static' may not be used without an array size")]
  StaticArrayWithoutBound,
  #[error("Operand of address-of operator must be lvalue, got '{0}'")]
  AddressofOperandNotLvalue(Elem),
  #[error("Operand of indirection operator must be pointer type, got '{0}'")]
  DerefNonPtr(Elem),
  #[error("Cannot dereference void pointer of type '{0}'")]
  DerefVoidPtr(Elem),
  #[error("Expression of type '{0}' is not assignable")]
  ExprNotAssignable(Elem),
  #[error("Return type mismatch: {0}")]
  ReturnTypeMismatch(CustomMessage),
  #[error("Duplicate label '{0}'")]
  DuplicateLabel(Elem),
  #[error(
    "Incompatible types in declaration of '{0}': '{1}' is not compatible with \
     '{2}'"
  )]
  IncompatibleType(Elem, QualifiedType, QualifiedType),
  #[error("Incompatible pointer types '{0}' and '{1}'")]
  IncompatiblePointerTypes(Elem, Elem),
  #[error("Cannot merge storage classes '{0}' and '{1}'")]
  StorageSpecsUnmergeable(Storage, Storage),
  #[error("{0}")]
  MainFunctionProtoMismatch(CustomMsgFixed),
  #[error("Discarding qualifiers '{0}' during conversion is not allowed")]
  DiscardingQualifiers(Qualifiers),
  #[error("Case label expression '{0}' is not an integer")]
  NonIntegerInCaseStmt(Constant),
  #[error("{0}")]
  InvalidConversion(CustomMessage),
  #[error("{0}")]
  Placeholder(CustomMessage),
  #[error("{0}")]
  Custom(CustomMessage),
  #[error("{0}")]
  UnsupportedFeature(CustomMessage),

  // errors ^^^ / vvv warnings
  #[error("Unused variable '{0}'")]
  UnusedVariable(Elem),
  #[error("Redundant storage specifiers '{0}'")]
  RedundantStorageSpecs(Storage),
  #[error("Redundant type qualifiers '{0}'")]
  RedundantQualifier(Qualifiers),
  #[error("Extern global variable '{0}' should not have an initializer")]
  ExternVariableWithInitializer(Elem),
  #[error("{0}")]
  VariableUninitialized(CustomMessage),
  #[error(
    "Function declarations without prototypes(e.g., int main()) are \
     deprecated and removed in C23. Please provide a prototype (e.g., int \
     main(void)) rather than leaving it empty."
  )]
  DeprecatedFunctionNoProto,
  #[error(
    "Applying unary operator '{}' may cause overflow on constant '{}'", &.0.1, &.0.0
  )]
  ArithmeticUnaryOpOverflow(Box<(Constant, Operator)>),
  #[error(
    "Arithmetic overflow in operation '{}' between '{}' and '{}'", &.0.2, &.0.0, &.0.1
  )]
  ArithmeticBinOpOverflow(Box<(Constant, Constant, Operator)>),
  #[error(
    "'{}' is used in a logical operation, {}", &.0, if let Some(suggest) = &.1 {
      format!(
        "you may want to use '{}' instead",
        suggest
      )
    } else {
      "which may not be the operation you intended".to_string()
    }
  )]
  LogicalOpMisuse(Operator /* got */, Option<Operator> /* suggest */),
  #[error("Possible data loss in implicit cast from '{0}' to '{1}'")]
  CastDown(QualifiedType, QualifiedType),
  #[error("Operation '{}' between '{}' and '{}' results in NaN", &.0.2, &.0.0, &.0.1)]
  NotANumber(Box<(Constant, Constant, Operator)>),
  #[error("Division by zero")]
  DivideByZero,
  #[error(
    "C standard pre C23 does not allow declaration after label, if/else, \
     while, do-while, for, and switch statements(e.g.`while(cond) int i = 0;` \
     is invalid). If it's intended, please use surrounding braces to form a \
     block."
  )]
  DeprecatedStmtDeclCvt,
  #[error("Typedef defines nothing")]
  EmptyTypedef,
  #[error("Empty statement")]
  EmptyStatement,
}
// TODO: reduce the size to 64 and lower vbytes.
static_assert!(
  ::std::mem::size_of::<Data>() <= 64,
  "Diagnostic Data too large!"
);

impl IntoWith<Severity, Meta> for Data {
  #[inline]
  fn into_with(self, severity: Severity) -> Meta {
    Meta::new(severity, self)
  }
}

fn format_expected(expected: &Option<Literal>) -> String {
  match expected {
    Some(exp) => format!(", expected '{}'", exp),
    None => String::with_capacity(0),
  }
}

impl Diag {
  #[inline]
  pub fn new(span: SourceSpan, severity: Severity, data: Data) -> Self {
    Self {
      metadata: Meta::new(severity, data),
      span,
    }
  }
}

pub struct DiagDisplay<'a> {
  diag: &'a Diag,
  source_manager: &'a SourceManager,
}

impl<'a> DisplayWith<'a, SourceManager, DiagDisplay<'a>> for Diag {
  fn display_with(
    &'a self,
    source_manager: &'a SourceManager,
  ) -> DiagDisplay<'a> {
    DiagDisplay {
      diag: self,
      source_manager,
    }
  }
}
impl<'a> ::std::fmt::Display for DiagDisplay<'a> {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    write!(
      f,
      "{}: {}: {}",
      self.diag.metadata.severity,
      self.diag.span.display_with(self.source_manager),
      self.diag.metadata.data
    )
  }
}
