// use crate::{
//   analyzer::{declaration::TranslationUnit, expression::Expression},
//   session::Session,
// };

// /// Simplify some syntactic sugar, like `a[i]` to `*(a + i)`, drop redundant declarations, etc.
// pub struct Desugar<'session> {
//   translation_unit: TranslationUnit,
//   session: &'session Session,
// }

// impl<'session> Desugar<'session> {
//   pub fn new(
//     translation_unit: TranslationUnit,
//     session: &'session Session,
//   ) -> Self {
//     Self {
//       translation_unit,
//       session,
//     }
//   }
// }

// impl<'session> Desugar<'session> {
//   pub fn expression(&mut self, expression: &Expression) {
//     // let folded = expression
//   }
// }
