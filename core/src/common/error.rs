use ::rc_utils::{DisplayWith, IntoWith};

use super::{Operator, SourceManager, SourceSpan, Storage};
use crate::types::Qualifiers;

/// Custom message. would be printed as-is.
type CustomMessage = String;
/// Element, like `expect ')' after <elem>`
type Elem = String;
/// Error `Version 2`. Will replace the old `Error` type (which is just ()) soon.
#[derive(Debug)]
pub struct Error {
  pub span: SourceSpan,
  pub data: Data,
}
#[derive(Debug)]
pub enum Data {
  // lexing errors
  UnexpectedCharacter(char),
  UnterminatedString,
  InvalidNumberFormat(String),
  // parseing errors
  MissingOperator(Operator),
  MultipleStorageSpecs(Storage, Storage),
  MissingTypeSpecifier(CustomMessage),
  MissingIdentifier(CustomMessage),
  ExtraneousComma(CustomMessage),
  VoidVariableDecl(CustomMessage),
  ExtraneousStorageSpecs(Storage),
  UnclosedParameterList(CustomMessage),
  MissingOpenParen(Elem),
  MissingCloseParen(Elem),
  ExpressionNotConstant(CustomMessage),
  VarDeclUnclosed(CustomMessage),
  InvalidBlockItem,
  MissingFunctionName,
  InvalidStmt(CustomMessage),
  CaseLabelAfterDefault,
  MultipleDefaultLabels,
  MissingLabelInSwitch,
  CaseLabelNotWithinSwitch,
  DefaultLabelNotWithinSwitch,
  TopLevelLabel,
  MissingLabelAfterGoto,
  InvalidBreakStmt,
  InvalidContinueStmt,
  UnsupportedFeature(CustomMessage),
  LabelNotFound(Elem),
  FunctionSpecsInVariableDecl(Elem),
  VariableAlreadyDefined(Elem),
  LocalExternVarWithInitializer(Elem),
  InvalidCallee(Elem),
  NotVariable(Elem),
  UndefinedVariable(Elem),
  TenaryTypeIncompatible(Elem, Elem),
  NonArithmeticInUnaryOp(Operator, Elem),
  NonArithmeticInBinaryOp(Elem, Elem, Operator),
  NonIntegerInBitwiseUnaryOp(Operator, Elem),
  NonIntegerInBitwiseBinaryOp(Elem, Elem, Operator),
  NonIntegerInBitshiftOp(Elem, Elem, Operator),
  AddressofOperandNotLvalue(Elem),
  IndirectionOperandNotPointer(Elem),
  DereferenceOfVoidPointer(Elem),
  ExprNotAssignable(Elem),
  ReturnTypeMismatch(CustomMessage),
  DuplicateLabel(Elem),
  IncompatiblePointerTypes(Elem, Elem),

  // storage.rs
  StorageSpecsUnmergeable(Storage, Storage),
  // types.rs
  MainFunctionProtoMismatch(CustomMessage),
  // conversion.rs
  DiscardingQualifiers(Qualifiers),
  InvalidConversion(CustomMessage),
  // placeholder for future errors
  Placeholder(String),
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
    write!(f, "{}: ", self.error.span.display_with(self.source_manager))?;

    match &self.error.data {
      Data::UnexpectedCharacter(c) => write!(f, "Unexpected character '{}'", c),
      Data::UnterminatedString => write!(f, "Unterminated string literal"),
      Data::InvalidNumberFormat(s) =>
        write!(f, "Invalid number format '{}'", s),
      Data::MissingOperator(operator) => write!(f, "Expect '{}'", operator),
      Data::MultipleStorageSpecs(l, r) =>
        write!(f, "Cannot combine storage classes '{}' and '{}'", l, r),
      Data::MissingTypeSpecifier(_) => write!(
        f,
        "Expect a type specifier in declaration, default to 'int'"
      ),
      Data::MissingIdentifier(_) =>
        write!(f, "Expect identifier in declarator"),
      Data::ExtraneousComma(msg) => write!(f, "{msg}"),
      Data::VoidVariableDecl(msg) => write!(f, "{msg}"),
      Data::ExtraneousStorageSpecs(storage) =>
        write!(f, "Storage class specifier '{storage}' is not allowed here"),
      Data::UnclosedParameterList(msg) => write!(f, "{msg}"),
      Data::MissingOpenParen(msg) => write!(f, "Expect '(' after {msg}"),
      Data::MissingCloseParen(msg) => write!(f, "Expect ')' after {msg}"),
      Data::ExpressionNotConstant(msg) =>
        write!(f, "Expression '{msg}' is not a constant"),
      Data::VarDeclUnclosed(msg) => write!(f, "{msg}"),
      Data::InvalidBlockItem =>
        write!(f, "Block definition is not allowed here"),
      Data::MissingFunctionName => write!(f, "Expect function name"),
      Data::InvalidStmt(msg) => write!(f, "{msg}"),
      Data::CaseLabelAfterDefault =>
        write!(f, "Case label cannot appear after default label"),
      Data::MultipleDefaultLabels => write!(
        f,
        "Multiple default labels in one switchl ignoring the latter"
      ),
      Data::MissingLabelInSwitch =>
        write!(f, "Expect at least one case or default label in switch"),
      Data::CaseLabelNotWithinSwitch =>
        write!(f, "Case label not within switch"),
      Data::DefaultLabelNotWithinSwitch =>
        write!(f, "Default label not within switch"),
      Data::TopLevelLabel => write!(f, "Label cannot appear at top level"),
      Data::MissingLabelAfterGoto =>
        write!(f, "Expect label identifier after goto"),
      Data::InvalidBreakStmt =>
        write!(f, "Break statement not within loop or switch"),
      Data::InvalidContinueStmt =>
        write!(f, "Continue statement not within loop"),
      Data::UnsupportedFeature(msg) => write!(f, "{msg}"),
      Data::LabelNotFound(label) => write!(f, "Label '{}' not found", label),
      Data::FunctionSpecsInVariableDecl(name) =>
        write!(f, "Variable '{}' cannot have function specifiers", name),
      Data::VariableAlreadyDefined(name) =>
        write!(f, "Variable '{}' already defined", name),
      Data::LocalExternVarWithInitializer(name) => write!(
        f,
        "Local extern variable '{}' cannot have initializer",
        name
      ),
      Data::InvalidCallee(name) =>
        write!(f, "expression '{}' is not callable", name),
      Data::NotVariable(name) => write!(f, "'{}' is not a variable", name),
      Data::UndefinedVariable(name) =>
        write!(f, "Variable '{}' is not defined", name),
      Data::TenaryTypeIncompatible(l, r) => write!(
        f,
        "Incompatible types '{}' and '{}' in ternary expression",
        l, r
      ),
      Data::NonArithmeticInUnaryOp(operator, type_name) => write!(
        f,
        "Operand of unary operator '{}' must be arithmetic type, got '{}'",
        operator, type_name
      ),
      Data::NonArithmeticInBinaryOp(l, r, operator) => write!(
        f,
        "Operands of binary operator '{}' must be arithmetic types, got '{}' and '{}'",
        operator, l, r
      ),
      Data::NonIntegerInBitwiseUnaryOp(operator, type_name) => write!(
        f,
        "Operand of bitwise operator '{}' must be integer type, got '{}'",
        operator, type_name
      ),
      Data::NonIntegerInBitwiseBinaryOp(l, r, operator) => write!(
        f,
        "Operands of bitwise operator '{}' must be integer types, got '{}' and '{}'",
        operator, l, r
      ),
      Data::NonIntegerInBitshiftOp(l, r, operator) => write!(
        f,
        "Operands of bitshift operator '{}' must be integer types, got '{}' and '{}'",
        operator, l, r
      ),
      Data::AddressofOperandNotLvalue(type_name) => write!(
        f,
        "Operand of address-of operator must be lvalue, got '{}'",
        type_name
      ),
      Data::IndirectionOperandNotPointer(type_name) => write!(
        f,
        "Operand of indirection operator must be pointer type, got '{}'",
        type_name
      ),
      Data::DereferenceOfVoidPointer(type_name) =>
        write!(f, "Cannot dereference void pointer of type '{}'", type_name),
      Data::ExprNotAssignable(type_name) =>
        write!(f, "Expression of type '{}' is not assignable", type_name),
      Data::ReturnTypeMismatch(msg) => write!(f, "Return type mismatch: {msg}"),
      Data::DuplicateLabel(label) => write!(f, "Duplicate label '{}'", label),
      Data::IncompatiblePointerTypes(l, r) =>
        write!(f, "Incompatible pointer types '{}' and '{}'", l, r),
      Data::StorageSpecsUnmergeable(l, r) =>
        write!(f, "Cannot merge storage classes '{}' and '{}'", l, r),
      Data::MainFunctionProtoMismatch(msg) => write!(f, "{msg}"),
      Data::DiscardingQualifiers(qualifiers) => write!(
        f,
        "Discarding qualifiers '{qualifiers}' during conversion is not allowed",
      ),
      Data::InvalidConversion(msg) => write!(f, "{msg}"),
      Data::Placeholder(msg) => write!(
        f,
        "An error occurred: {msg}. This error was not categorized yet; consider create a error category for it at {}",
        file!()
      ),
    }
  }
}
