use ::rc_utils::{DisplayWith, IntoWith};
use ::thiserror::Error;

use crate::{
  common::{Keyword, Literal, Operator, SourceManager, SourceSpan, Storage},
  types::{QualifiedType, Qualifiers},
};

/// Custom message. would be printed as-is.
type CustomMessage = String;
/// Fixed custom message.
type CustomMsgFixed = &'static str;
/// Element, like `expect ')' after <elem>`
type Elem = String;

#[derive(Debug, Error)]
pub enum Data {
  #[error(
    "Unexpected character '{0}'{expected}",
    expected = format_expected(&.1)
  )]
  UnexpectedCharacter(Literal, Option<Literal>),
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
  #[error("Local extern variable '{0}' cannot have initializer")]
  LocalExternVarWithInitializer(Elem),
  #[error("expression '{0}' is not callable")]
  InvalidCallee(Elem),
  #[error("'{0}' is not a variable")]
  NotVariable(Elem),
  #[error("Variable '{0}' is not defined")]
  UndefinedVariable(Elem),
  #[error("Incompatible types '{0}' and '{1}' in ternary expression")]
  TenaryTypeIncompatible(Elem, Elem),
  #[error("Operand of unary operator '{0}' must be arithmetic type, got '{1}'")]
  NonArithmeticInUnaryOp(Operator, Elem),
  #[error(
    "Operands of binary operator '{2}' must be arithmetic types, got '{0}' and '{1}'"
  )]
  NonArithmeticInBinaryOp(Elem, Elem, Operator),
  #[error("Operand of bitwise operator '{0}' must be integer type, got '{1}'")]
  NonIntegerInBitwiseUnaryOp(Operator, Elem),
  #[error(
    "Operands of bitwise operator '{2}' must be integer types, got '{0}' and '{1}'"
  )]
  NonIntegerInBitwiseBinaryOp(Elem, Elem, Operator),
  #[error(
    "Operands of bitshift operator '{2}' must be integer types, got '{0}' and '{1}'"
  )]
  NonIntegerInBitshiftOp(Elem, Elem, Operator),
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
    "Incompatible types in declaration of '{0}': '{1}' is not compatible with '{2}'"
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
  #[error("{0}")]
  InvalidConversion(CustomMessage),
  #[error("{0}")]
  Placeholder(CustomMessage),
  #[error("{0}")]
  Custom(CustomMessage),
  #[error("{0}")]
  UnsupportedFeature(CustomMessage),
}

fn format_expected(expected: &Option<Literal>) -> String {
  match expected {
    Some(exp) => format!(", expected '{}'", exp),
    None => String::new(),
  }
}

#[derive(Debug)]
pub struct Error {
  pub span: SourceSpan,
  pub data: Data,
}
impl Error {
  pub fn new(span: SourceSpan, data: Data) -> Self {
    Self { span, data }
  }
}

impl IntoWith<SourceSpan, Error> for Data {
  fn into_with(self, span: SourceSpan) -> Error {
    Error::new(span, self)
  }
}
pub struct ErrorDisplay<'a> {
  error: &'a Error,
  source_manager: &'a SourceManager,
}

impl<'a> DisplayWith<'a, SourceManager, ErrorDisplay<'a>> for Error {
  fn display_with(
    &'a self,
    source_manager: &'a SourceManager,
  ) -> ErrorDisplay<'a> {
    ErrorDisplay {
      error: self,
      source_manager,
    }
  }
}
impl<'a> ::std::fmt::Display for ErrorDisplay<'a> {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    write!(
      f,
      "{}: {}",
      self.error.span.display_with(self.source_manager),
      self.error.data
    )
  }
}
