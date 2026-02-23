use ::std::fmt::Display;

use super::{
  Module,
  instruction::{self as inst, Instruction},
};
use crate::ir::value::{LookUp, Value};

impl Display for Module<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.globals.iter().try_for_each(|(.., global)| {
      writeln!(f, "global {}: {}", global.name, global.qualified_type)
    })?;
    self.functions.iter().try_for_each(|(.., func)| {
      let mut counter = 0;
      writeln!(
        f,
        "{} @{} {}: ({})",
        if func.blocks.is_empty() {
          "declare"
        } else {
          "define"
        },
        func.name,
        func.return_type,
        func
          .params
          .iter()
          .map(|param_id| {
            format!(
              "%{}: {}",
              {
                let id = counter;
                counter += 1;
                id
              },
              self.lookup(param_id).qualified_type
            )
          })
          .collect::<Vec<_>>()
          .join(", "),
      )?;
      func.blocks.iter().try_for_each(|block_id| {
        writeln!(f, "  {}:", {
          let id = counter;
          counter += 1;
          id
        })?;
        self
          .lookup(block_id)
          .instructions
          .iter()
          .try_for_each(|inst_id| {
            let inst = self
              .lookup(*inst_id)
              .as_call()
              .expect("not implemented for others!");
            writeln!(
              f,
              "    call @{}({})",
              match self.lookup(inst.callee).value {
                Value::Function(f) => {
                  self.functions[f].name
                },
                _ => todo!(),
              },
              inst
                .args
                .iter()
                .map(|arg_id| {
                  let id = counter;
                  counter += 1;
                  match &self.lookup(arg_id).value {
                    Value::Instruction(inst_id) => todo!(),
                    Value::Argument(func_id, _) => todo!(),
                    Value::Constant(constant) => format!("{}", constant),
                    Value::Function(func_id) => todo!(),
                    Value::Global(global_id) => todo!(),
                  }
                })
                .collect::<Vec<_>>()
                .join(", ")
            )
          })
      })
    })
  }
}
