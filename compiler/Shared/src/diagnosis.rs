mod data {
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
  //! use ::rcc_utils::IntoWith;
  //! use ::rcc_shared::{DiagData, DiagMeta, Severity};
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
  pub struct Diag<'c> {
    pub(crate) metadata: Meta<'c>,
    pub(crate) span: SourceSpan,
  }
  /// Message with [`Severity`].
  #[derive(Debug)]
  pub struct Meta<'c> {
    pub(crate) severity: Severity,
    pub(crate) data: Data<'c>,
  }
  impl<'c> Meta<'c> {
    pub fn new(severity: Severity, data: Data<'c>) -> Self {
      Self { severity, data }
    }
  }
  impl<'c> IntoWith<SourceSpan, Diag<'c>> for Meta<'c> {
    #[inline]
    fn into_with(self, span: SourceSpan) -> Diag<'c> {
      Diag::new(span, self.severity, self.data)
    }
  }
  impl<'c> Add<SourceSpan> for Meta<'c> {
    type Output = Diag<'c>;

    #[inline]
    fn add(self, rhs: SourceSpan) -> Self::Output {
      self.into_with(rhs)
    }
  }
  #[derive(Debug, Clone, Copy, ::strum_macros::Display)]
  pub enum Severity {
    Hint,
    Info,
    Warning,
    Error,
  }
  use ::rcc_utils::{DisplayWith, IntoWith, StrRef, static_assert};
  use ::std::ops::Add;

  use crate::{
    Keyword, Literal, Number, Operator, SourceManager, SourceSpan, Storage,
  };
  /// Custom message. would be printed as-is.
  type CustomMessage = String;
  /// Fixed custom message.
  type CustomMsgFixed = &'static str;
  /// Element, like `expect ')' after <elem>`
  type Elem = String;
  type QualTyStr = String;

  /// Plain error/warning/other diagnostic messages.
  ///
  /// TODO: reduce the size of this enum.
  #[derive(Debug, ::thiserror::Error)]
  pub enum Data<'c> {
    #[error("Unexpected character '{}'{expected}", &.0.0, expected = format_expected(&.0.1))]
    UnexpectedCharacter(Box<(String, Option<String>)>),
    #[error("Unterminated string literal")]
    UnterminatedString,
    #[error("{0}")]
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
    MissingOpenParen(Literal<'c>),
    #[error("Expect ')' after {0}")]
    MissingCloseParen(Literal<'c>),
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
    #[error("Break statement cannot appear at top level")]
    TopLevelBreak,
    #[error("Continue statement cannot appear at top level")]
    TopLevelContinue,
    #[error(
      "Break statement can only appear inside body of while, do-while, for \
       loop, or switch statement."
    )]
    BreakNotWithinLoop,
    #[error(
      "Continue statement can only appear inside body of while, do-while, or \
       a for loop."
    )]
    ContinueNotWithinLoop,
    #[error("Expect label identifier after goto")]
    MissingLabelAfterGoto,
    #[error("{0}")]
    InvalidControlFlowStmt(CustomMessage),
    #[error("Label '{0}' not found")]
    LabelNotFound(StrRef<'c>),
    #[error("Variable '{0}' cannot have function specifiers")]
    FunctionSpecsInVariableDecl(Elem),
    #[error(
      "Declaration of variable '{0}' declared with deduced type '__auto_type' \
       requires an initializer"
    )]
    DeducedTypeWithNoInitializer(Elem),
    #[error("Variable '{0}' already defined")]
    VariableAlreadyDefined(Elem),
    #[error("Function '{0}' already defined")]
    FunctionAlreadyDefined(Elem),
    #[error("Local extern variable '{0}' cannot have initializer")]
    LocalExternVarWithInitializer(Elem),
    #[error("expression '{0}' is not callable")]
    InvalidCallee(QualTyStr),
    #[error("'{0}' is not a variable")]
    NotVariable(Elem),
    #[error("Variable '{0}' is not defined")]
    UndefinedVariable(StrRef<'c>),
    #[error("Incompatible types '{0}' and '{1}' in ternary expression")]
    TenaryTypeIncompatible(Elem, Elem),
    #[error(
      "Operand of unary operator '{0}' must be arithmetic type, got '{1}'"
    )]
    NonArithmeticInUnaryOp(Operator, Elem),
    #[error(
      "Operands of binary operator '{2}' must be arithmetic types, got '{0}' \
       and '{1}'"
    )]
    NonArithmeticInBinaryOp(Elem, Elem, Operator),
    #[error(
      "Operand of bitwise operator '{0}' must be integer type, got '{1}'"
    )]
    NonIntegerInBitwiseUnaryOp(Operator, Elem),
    #[error(
      "Operands of bitwise operator '{2}' must be integer types, got '{0}' \
       and '{1}'"
    )]
    NonIntegerInBitwiseBinaryOp(Elem, Elem, Operator),
    #[error(
      "Operands of bitshift operator '{2}' must be integer types, got '{0}' \
       and '{1}'"
    )]
    NonIntegerInBitshiftOp(Elem, Elem, Operator),
    #[error("Array subscript is not an integer, got '{0}'")]
    NonIntegerInArraySubscript(Elem),
    #[error("'static' may not be used without an array size")]
    StaticArrayWithoutBound,
    #[error("Operand of address-of operator must be lvalue, got '{0}'")]
    AddressofOperandNotLvalue(Elem),
    /// NOTE: `register` with implicit address-of, e.g., decay not handled.
    #[error(
      "variable '{0}' declared with 'register' cannot be taken address of"
    )]
    AddressofOperandRegVar(Elem),
    #[error("Global variable cannot be declared with 'register', ignoring")]
    GlobalRegVar(Elem),
    #[error(
      "variable declared with 'register' can only have pointer type, got '{0}'"
    )]
    InvalidRegVarDecl(QualTyStr),
    #[error("Operand of indirection operator must be pointer type, got '{0}'")]
    DerefNonPtr(Elem),
    #[error("Array subscript is not an integer, got '{0}'")]
    NonIntegerSubscript(Elem),
    #[error("Cannot dereference void pointer of type '{0}'")]
    DerefVoidPtr(Elem),
    #[error("Expression '{0}' is not assignable")]
    ExprNotAssignable(Elem),
    #[error("Return type mismatch: {0}")]
    ReturnTypeMismatch(CustomMessage),
    #[error("Duplicate label '{0}'")]
    DuplicateLabel(StrRef<'c>),
    #[error(
      "Incompatible types in declaration of '{0}': '{1}' is not compatible \
       with '{2}'"
    )]
    IncompatibleType(StrRef<'c>, QualTyStr, QualTyStr),
    #[error("Incompatible pointer types '{0}' and '{1}'")]
    IncompatiblePointerTypes(QualTyStr, QualTyStr),
    #[error("Cannot merge storage classes '{0}' and '{1}'")]
    StorageSpecsUnmergeable(Storage, Storage),
    #[error("{0}")]
    MainFunctionProtoMismatch(CustomMsgFixed),
    #[error("Discarding qualifiers '{0}' during conversion is not allowed")]
    DiscardingQualifiers(String),
    #[error("Case label expression '{0}' is not an integer")]
    NonIntegerInCaseStmt(String),
    #[error("Character '{0}' too long for it's corresponding type")]
    CharacterTooLong(String),
    #[error("{0}")]
    InvalidConversion(CustomMessage),
    #[error("Cannot apply operator '{2}' to types '{0}' and '{1}'")]
    InvalidOprand(QualTyStr, QualTyStr, Operator),
    #[error("Comparison of distinct pointer types '{0}' and '{1}'")]
    CompareDistinctPointerTypes(QualTyStr, QualTyStr),
    #[error("Comparison of pointer and integer types '{0}' and '{1}'")]
    CompareBetweenPointerAndInteger(QualTyStr, QualTyStr),
    #[error(
      "Invalid comparison between types '{0}' and '{1}' with operator '{2}'"
    )]
    InvalidComparison(QualTyStr, QualTyStr, Operator),
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
    RedundantQualifier(String),
    #[error("Extern global variable '{0}' should not have an initializer")]
    ExternVariableWithInitializer(Elem),
    #[error("{0}")]
    VariableUninitialized(CustomMessage),
    #[error("Left comma has no effect here; consider removing it")]
    LeftCommaNoEffect,
    #[error(
      "Function declarations without prototypes(e.g., int main()) are \
       deprecated and removed in C23. Please provide a prototype (e.g., int \
       main(void)) rather than leaving it empty."
    )]
    DeprecatedFunctionNoProto,
    #[error(
    "Applying unary operator '{}' may cause overflow on constant '{}'", &.0.1, &.0.0
  )]
    ArithmeticUnaryOpOverflow(Box<(Number, Operator)>),
    #[error(
    "Arithmetic overflow in operation '{}' between '{}' and '{}'", &.0.2, &.0.0, &.0.1
  )]
    ArithmeticBinOpOverflow(Box<(Number, Number, Operator)>),
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
    LogicalOpMisuse(
      Operator,         /* got */
      Option<Operator>, /* suggest */
    ),
    #[error("Possible data loss in implicit cast from '{0}' to '{1}'")]
    CastDown(QualTyStr, QualTyStr),
    #[error("Operation '{}' between '{}' and '{}' results in NaN", &.0.2, &.0.0, &.0.1)]
    NotANumber(Box<(String, String, Operator)>),
    #[error("Division by zero")]
    DivideByZero,
    #[error(
      "C standard pre C23 does not allow declaration after label, if/else, \
       while, do-while, for, and switch statements(e.g.`while(cond) int i = \
       0;` is invalid). If it's intended, please use surrounding braces to \
       form a block."
    )]
    DeprecatedStmtDeclCvt,
    #[error(
      "Line continuation with backslash should not be followed by whitespace"
    )]
    WhitespaceAfterLineEscape,
    #[error("Invalid escape sequence '{0}' in character literal")]
    InvalidEscapeSequence(String),
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

  impl<'c> IntoWith<Severity, Meta<'c>> for Data<'c> {
    #[inline]
    fn into_with(self, severity: Severity) -> Meta<'c> {
      Meta::new(severity, self)
    }
  }
  impl<'c> Add<Severity> for Data<'c> {
    type Output = Meta<'c>;

    #[inline]
    fn add(self, rhs: Severity) -> Self::Output {
      self.into_with(rhs)
    }
  }

  fn format_expected(expected: &Option<String>) -> String {
    match expected {
      Some(exp) => format!(", expected '{}'", exp),
      None => String::with_capacity(0),
    }
  }

  impl<'c> Diag<'c> {
    #[inline]
    pub fn new(span: SourceSpan, severity: Severity, data: Data<'c>) -> Self {
      Self {
        metadata: Meta::new(severity, data),
        span,
      }
    }
  }

  pub struct DiagDisplay<'a> {
    diag: &'a Diag<'a>,
    source_manager: &'a SourceManager,
  }

  impl<'a> DisplayWith<'a, SourceManager, DiagDisplay<'a>> for Diag<'a> {
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
}
use ::std::cell::{Ref, RefCell};

pub use self::data::{Data, Diag, Meta, Severity};
use super::SourceSpan;

pub trait Diagnosis<'c> {
  #[must_use]
  fn has_errors(&self) -> bool;
  #[must_use]
  fn has_warnings(&self) -> bool;
  #[must_use]
  fn errors(&self) -> Ref<'_, Vec<Diag<'_>>>;
  #[must_use]
  fn warnings(&self) -> Ref<'_, Vec<Diag<'_>>>;
  fn add_error(&self, error: Data<'c>, span: SourceSpan);
  fn add_warning(&self, warning: Data<'c>, span: SourceSpan);
  fn add_diag(&self, diag: Diag<'c>) {
    match diag.metadata.severity {
      Severity::Error => self.add_error(diag.metadata.data, diag.span),
      Severity::Warning => self.add_warning(diag.metadata.data, diag.span),
      Severity::Info | Severity::Hint => {}, // ignore info for now
    }
  }
}

#[derive(Default, Debug)]

pub struct Operational<'c> {
  warnings: RefCell<Vec<Diag<'c>>>,
  errors: RefCell<Vec<Diag<'c>>>,
}

impl<'c> Diagnosis<'c> for Operational<'c> {
  #[inline]
  fn has_errors(&self) -> bool {
    !self.errors.borrow().is_empty()
  }

  #[inline]
  fn has_warnings(&self) -> bool {
    !self.warnings.borrow().is_empty()
  }

  #[inline]
  fn errors(&self) -> Ref<'_, Vec<Diag<'_>>> {
    self.errors.borrow()
  }

  #[inline]
  fn warnings(&self) -> Ref<'_, Vec<Diag<'_>>> {
    self.warnings.borrow()
  }

  #[inline]
  fn add_error(&self, error: Data<'c>, span: SourceSpan) {
    self
      .errors
      .borrow_mut()
      .push(Diag::new(span, Severity::Error, error));
  }

  #[inline]
  fn add_warning(&self, data: Data<'c>, span: SourceSpan) {
    self
      .warnings
      .borrow_mut()
      .push(Diag::new(span, Severity::Warning, data));
  }
}

pub struct NoOp {
  /// rust strict rules w.r.t. thread safety(!Sync)
  /// and lifetime issues makes it difficult to just create a dummmy noop struct.
  idk: RefCell<Vec<Diag<'static>>>,
}
impl ::std::default::Default for NoOp {
  #[inline]
  fn default() -> Self {
    Self {
      idk: RefCell::new(Vec::with_capacity(0)),
    }
  }
}

impl Diagnosis<'_> for NoOp {
  #[inline]
  fn has_errors(&self) -> bool {
    false
  }

  #[inline]
  fn has_warnings(&self) -> bool {
    false
  }

  #[inline]
  fn errors(&self) -> Ref<'_, Vec<Diag<'_>>> {
    self.idk.borrow()
  }

  #[inline]
  fn warnings(&self) -> Ref<'_, Vec<Diag<'_>>> {
    self.idk.borrow()
  }

  #[inline]
  fn add_error(&self, _error: Data, _span: SourceSpan) {}

  #[inline]
  fn add_warning(&self, _warning: Data, _span: SourceSpan) {}
}
