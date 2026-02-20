// use ::rcc_utils::{Dummy, IntoWith};

// use crate::{
//   analyzer::expression::{Binary, ConstantLiteral, Expression},
//   common::{Operator, SourceSpan},
//   types::QualifiedType,
// };

// impl<'context> Expression<'context> {
//   pub fn oneplusone() -> Self {
//     Self::new_rvalue(
//       Binary::new(
//         Operator::Plus,
//         Self::new_rvalue(
//           ConstantLiteral::Integral(1.into()).into_with(Dummy::dummy()),
//           QualifiedType::int(),
//         ),
//         Self::new_rvalue(
//           ConstantLiteral::Integral(1.into()).into_with(Dummy::dummy()),
//           QualifiedType::int(),
//         ),
//         SourceSpan::dummy(),
//       )
//       .into(),
//       QualifiedType::int(),
//     )
//   }
// }
