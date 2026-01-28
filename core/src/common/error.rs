use ::rc_utils::{DisplayWith, IntoWith};

use super::{Keyword, Literal, Operator, SourceManager, SourceSpan, Storage};
use crate::types::{QualifiedType, Qualifiers};

/// Custom message. would be printed as-is.
type CustomMessage = String;
type CustomMsgFixed = &'static str;
/// Element, like `expect ')' after <elem>`
type Elem = String;

#[derive(Debug)]
pub struct Error {
  pub span: SourceSpan,
  pub data: Data,
}
#[derive(Debug)]
pub enum Data {
  // lexing errors
  UnexpectedCharacter(
    Literal,         /* got */
    Option<Literal>, /* expected */
  ),
  UnterminatedString,
  InvalidNumberFormat(String),
  // parseing errors
  MissingOperator(Operator),
  MultipleStorageSpecs(Storage, Storage),
  MissingTypeSpecifier,
  MissingIdentifier(CustomMessage),
  ExtraneousComma(CustomMsgFixed),
  VoidVariableDecl(CustomMessage),
  ExtraneousStorageSpecs(Storage),
  UnclosedParameterList(CustomMessage),
  MissingOpenParen(Literal),
  MissingCloseParen(Literal),
  ExprNotConstant(CustomMessage),
  VarDeclUnclosed(CustomMessage),
  InvalidBlockItem,
  MissingFunctionName,
  InvalidStmt(CustomMessage),
  CaseLabelAfterDefault,
  MultipleDefaultLabels,
  MissingLabelInSwitch,
  LabelNotWithinSwitch(Keyword),
  TopLevelLabel,
  MissingLabelAfterGoto,
  InvalidControlFlowStmt(CustomMessage),
  LabelNotFound(Elem),
  FunctionSpecsInVariableDecl(Elem),
  // semantic errors
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
  DerefNonPtr(Elem),
  DerefVoidPtr(Elem),
  ExprNotAssignable(Elem),
  ReturnTypeMismatch(CustomMessage),
  DuplicateLabel(Elem),
  IncompatibleType(Elem, QualifiedType, QualifiedType),
  IncompatiblePointerTypes(Elem, Elem),

  // storage.rs
  StorageSpecsUnmergeable(Storage, Storage),
  // types.rs
  MainFunctionProtoMismatch(CustomMessage),
  // conversion.rs
  DiscardingQualifiers(Qualifiers),
  InvalidConversion(CustomMessage),
  // placeholder for future errors
  Placeholder(CustomMessage),
  Custom(CustomMessage),
  UnsupportedFeature(CustomMessage),
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

    use Data::*;

    match &self.error.data {
      UnexpectedCharacter(got, expected) => write!(
        f,
        "Unexpected character '{}'{}",
        got,
        match expected {
          Some(exp) => format!(", expected '{}'", exp),
          None => "".to_string(),
        }
      ),
      UnterminatedString => write!(f, "Unterminated string literal"),
      InvalidNumberFormat(s) => write!(f, "Invalid number format '{}'", s),
      MissingOperator(operator) => write!(f, "Expect '{}'", operator),
      MultipleStorageSpecs(l, r) =>
        write!(f, "Cannot combine storage classes '{}' and '{}'", l, r),
      MissingTypeSpecifier => write!(
        f,
        "Expect a type specifier in declaration, default to 'int'"
      ),
      MissingIdentifier(_) => write!(f, "Expect identifier in declarator"),
      ExtraneousComma(msg) => write!(f, "{msg}"),
      VoidVariableDecl(msg) => write!(f, "{msg}"),
      ExtraneousStorageSpecs(storage) =>
        write!(f, "Storage class specifier '{storage}' is not allowed here"),
      UnclosedParameterList(msg) => write!(f, "{msg}"),
      MissingOpenParen(msg) => write!(f, "Expect '(' after {msg}"),
      MissingCloseParen(msg) => write!(f, "Expect ')' after {msg}"),
      ExprNotConstant(msg) => write!(f, "{msg}"),
      VarDeclUnclosed(msg) => write!(f, "{msg}"),
      InvalidBlockItem => write!(f, "Block definition is not allowed here"),
      MissingFunctionName => write!(f, "Expect function name"),
      InvalidStmt(msg) => write!(f, "{msg}"),
      CaseLabelAfterDefault =>
        write!(f, "Case label cannot appear after default label"),
      MultipleDefaultLabels => write!(
        f,
        "Multiple default labels in one switchl ignoring the latter"
      ),
      MissingLabelInSwitch =>
        write!(f, "Expect at least one case or default label in switch"),
      LabelNotWithinSwitch(elem) => write!(f, "{elem} label not within switch"),
      TopLevelLabel => write!(f, "Label cannot appear at top level"),
      MissingLabelAfterGoto => write!(f, "Expect label identifier after goto"),
      InvalidControlFlowStmt(msg) => write!(f, "{msg}"),
      UnsupportedFeature(msg) => write!(f, "{msg}"),
      LabelNotFound(label) => write!(f, "Label '{}' not found", label),
      FunctionSpecsInVariableDecl(name) =>
        write!(f, "Variable '{}' cannot have function specifiers", name),
      VariableAlreadyDefined(name) =>
        write!(f, "Variable '{}' already defined", name),
      LocalExternVarWithInitializer(name) => write!(
        f,
        "Local extern variable '{}' cannot have initializer",
        name
      ),
      InvalidCallee(name) => write!(f, "expression '{}' is not callable", name),
      NotVariable(name) => write!(f, "'{}' is not a variable", name),
      UndefinedVariable(name) =>
        write!(f, "Variable '{}' is not defined", name),
      TenaryTypeIncompatible(l, r) => write!(
        f,
        "Incompatible types '{}' and '{}' in ternary expression",
        l, r
      ),
      NonArithmeticInUnaryOp(operator, type_name) => write!(
        f,
        "Operand of unary operator '{}' must be arithmetic type, got '{}'",
        operator, type_name
      ),
      NonArithmeticInBinaryOp(l, r, operator) => write!(
        f,
        "Operands of binary operator '{}' must be arithmetic types, got '{}' and '{}'",
        operator, l, r
      ),
      NonIntegerInBitwiseUnaryOp(operator, type_name) => write!(
        f,
        "Operand of bitwise operator '{}' must be integer type, got '{}'",
        operator, type_name
      ),
      NonIntegerInBitwiseBinaryOp(l, r, operator) => write!(
        f,
        "Operands of bitwise operator '{}' must be integer types, got '{}' and '{}'",
        operator, l, r
      ),
      NonIntegerInBitshiftOp(l, r, operator) => write!(
        f,
        "Operands of bitshift operator '{}' must be integer types, got '{}' and '{}'",
        operator, l, r
      ),
      AddressofOperandNotLvalue(type_name) => write!(
        f,
        "Operand of address-of operator must be lvalue, got '{}'",
        type_name
      ),
      DerefNonPtr(type_name) => write!(
        f,
        "Operand of indirection operator must be pointer type, got '{}'",
        type_name
      ),
      DerefVoidPtr(type_name) =>
        write!(f, "Cannot dereference void pointer of type '{}'", type_name),
      ExprNotAssignable(type_name) =>
        write!(f, "Expression of type '{}' is not assignable", type_name),
      ReturnTypeMismatch(msg) => write!(f, "Return type mismatch: {msg}"),
      DuplicateLabel(label) => write!(f, "Duplicate label '{}'", label),
      IncompatiblePointerTypes(l, r) =>
        write!(f, "Incompatible pointer types '{}' and '{}'", l, r),
      StorageSpecsUnmergeable(l, r) =>
        write!(f, "Cannot merge storage classes '{}' and '{}'", l, r),
      MainFunctionProtoMismatch(msg) => write!(f, "{msg}"),
      DiscardingQualifiers(qualifiers) => write!(
        f,
        "Discarding qualifiers '{qualifiers}' during conversion is not allowed",
      ),
      InvalidConversion(msg) => write!(f, "{msg}"),
      IncompatibleType(name, l, r) => write!(
        f,
        "Incompatible types in declaration of '{}': '{}' is not compatible with '{}'",
        name, l, r
      ),
      Custom(msg) => write!(f, "{msg}"),
      Placeholder(msg) => write!(
        f,
        "An error occurred: {msg}. This error was not categorized yet; consider create a error category for it at {}",
        file!()
      ),
    }
  }
}
