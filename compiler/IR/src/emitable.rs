use ::rcc_ast::types::{self as ast, TypeInfo};
use ::rcc_shared::Constant;
use ::rcc_utils::RefEq;

use super::{
  Argument, Emitter, Value, ValueData, ValueID,
  instruction::{self as inst, Instruction},
  module,
};

/// Overload helper. I love overloading.
///
/// Also, this `emit` function acts like the ctor but with assertions
pub trait Emitable<'a, ValueType> {
  #[must_use = "Usually the return value_id shall not be ignored; one such \
                exception is for `store` instruction, which returns void. use \
                `_` to explicitly` ignore the return value_id if you don't \
                need it."]
  fn emit(&mut self, value: ValueType, ast_type: ast::TypeRef<'a>) -> ValueID;
}

impl<'c> Emitable<'c, inst::Binary> for Emitter<'c> {
  fn emit(
    &mut self,
    binary: inst::Binary,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit_common_instruction(binary, ast_type)
  }
}

impl<'c> Emitable<'c, inst::Call> for Emitter<'c> {
  fn emit(&mut self, call: inst::Call, ast_type: ast::TypeRef<'c>) -> ValueID {
    self.emit_common_instruction(call, ast_type)
  }
}

impl<'c> Emitable<'c, module::Function<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: module::Function<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit_globals(value, ast_type)
  }
}
impl<'c> Emitable<'c, module::Variable<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: module::Variable<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.emit_globals(value, ast_type)
  }
}
impl<'c> Emitable<'c, Constant<'c>> for Emitter<'c> {
  fn emit(
    &mut self,
    value: Constant<'c>,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    self.ir().intern_constant(value, ast_type)
  }
}
impl<'c> Emitable<'c, Argument> for Emitter<'c> {
  fn emit(&mut self, value: Argument, ast_type: ast::TypeRef<'c>) -> ValueID {
    self.ir().insert(Value::new(
      ast_type,
      ty!(self, ast_type),
      value,
      self.current_function,
    ))
  }
}

// Strengthened vvv
mod instruction {
  use inst::*;

  use super::*;
  mod terminator {
    use super::*;
    impl<'c> Emitable<'c, Jump> for Emitter<'c> {
      fn emit(&mut self, jump: Jump, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          ast_type.is_void(),
          "Jump instructions must have void type."
        );
        debug_assert!(
          jump.target().is_null()
            || self.visit(jump.target(), |value| value.ir_type.is_label()),
          "Jump target must be a basic block, or unset (null) for backpatching"
        );
        self.emit_terminator(jump, ast_type, self.current_block)
      }
    }
    impl<'c> Emitable<'c, Branch> for Emitter<'c> {
      fn emit(
        &mut self,
        branch: Branch,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type.is_void(),
          "Branch instructions must have void type."
        );
        debug_assert!(
          branch.condition().is_null()
            || self.visit(branch.condition(), |value| value
              .ir_type
              .as_integer()
              .is_some_and(|&width| width == 1)),
          "Branch condition must be an i1 value, or unset (null)."
        );
        debug_assert!(
          branch.then_branch().is_null()
            || self
              .visit(branch.then_branch(), |value| value.ir_type.is_label()),
          "Branch then target must be a basic block, or unset (null)."
        );
        debug_assert!(
          branch.else_branch().is_null()
            || self
              .visit(branch.else_branch(), |value| value.ir_type.is_label()),
          "Branch else target must be a basic block, or unset (null)."
        );
        self.emit_terminator(branch, ast_type, self.current_block)
      }
    }
    impl<'c> Emitable<'c, Return> for Emitter<'c> {
      fn emit(&mut self, ret: Return, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          ast_type.is_void(),
          "Return instructions must have void type."
        );

        self.emit_terminator(ret, ast_type, self.current_block)
      }
    }
    impl<'c> Emitable<'c, Unreachable> for Emitter<'c> {
      fn emit(
        &mut self,
        unreachable: Unreachable,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type.is_void(),
          "Unreachable instructions must have void type."
        );
        self.emit_terminator(unreachable, ast_type, self.current_block)
      }
    }
  }
  mod cmp {
    use super::*;
    impl<'c> Emitable<'c, ICmp> for Emitter<'c> {
      fn emit(&mut self, icmp: ICmp, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          RefEq::ref_eq(ast_type, self.ast().i1_bool_type()),
          "ICmp inst must have boolean as return type."
        );
        debug_assert!(
          self.visit(icmp.lhs(), |value| value.ir_type.is_integer()
            || value.ir_type.is_pointer()),
          "ICmp lhs must be an integer"
        );
        debug_assert!(
          self.visit(icmp.rhs(), |value| value.ir_type.is_integer()
            || value.ir_type.is_pointer()),
          "ICmp rhs must be an integer"
        );

        let cmp = self.emit_common_instruction(Cmp::from(icmp), {
          let this = &self;
          this.ast().i1_bool_type()
        });
        if !RefEq::ref_eq(ast_type, self.ast().converted_bool()) {
          cmp
        } else {
          self.emit(Zext::new(cmp), self.ast().converted_bool())
        }
      }
    }

    impl<'c> Emitable<'c, FCmp> for Emitter<'c> {
      fn emit(&mut self, fcmp: FCmp, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          RefEq::ref_eq(ast_type, self.ast().i1_bool_type()),
          "FCmp inst must have boolean as return type."
        );
        debug_assert!(
          self.visit(fcmp.lhs(), |value| value.ir_type.is_floating()),
          "FCmp lhs must be a floating-point type"
        );
        debug_assert!(
          self.visit(fcmp.rhs(), |value| value.ir_type.is_floating()),
          "FCmp rhs must be a floating-point type"
        );
        let cmp = self.emit_common_instruction(Cmp::from(fcmp), {
          let this = &self;
          this.ast().i1_bool_type()
        });
        if !RefEq::ref_eq(ast_type, self.ast().converted_bool()) {
          cmp
        } else {
          self.emit(Zext::new(cmp), self.ast().converted_bool())
        }
      }
    }
  }
  // Cast vvv
  mod cast {
    use super::*;
    impl<'c> Emitable<'c, Zext> for Emitter<'c> {
      fn emit(&mut self, zext: Zext, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          ast_type.as_primitive().is_some_and(|p| p.is_unsigned()),
          "Zext target type must be an unsigned integer"
        );
        debug_assert!(
          self.visit(zext.operand(), |value| value
            .ir_type
            .as_integer()
            .is_some_and(|&width| width < ast_type.size_bits() as u8)),
          "Zext operand must be an integer"
        );
        self.emit_common_instruction(Cast::Zext(zext), ast_type)
      }
    }
    impl<'c> Emitable<'c, Sext> for Emitter<'c> {
      fn emit(&mut self, sext: Sext, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          ast_type
            .as_primitive()
            .is_some_and(|p| p.is_signed_integer()),
          "Sext target type must be a signed integer"
        );
        debug_assert!(
          self.visit(sext.operand(), |value| value
            .ir_type
            .as_integer()
            .is_some_and(|&width| width < ast_type.size_bits() as u8)),
          "Sext operand must be an integer"
        );
        self.emit_common_instruction(Cast::Sext(sext), ast_type)
      }
    }
    impl<'c> Emitable<'c, Trunc> for Emitter<'c> {
      fn emit(&mut self, trunc: Trunc, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          ast_type.is_integer(),
          "Trunc target type must be an integer"
        );
        debug_assert!(
          self.visit(trunc.operand(), |value| value
            .ir_type
            .as_integer()
            .is_some_and(|&width| width > ast_type.size_bits() as u8)),
          "Trunc operand must be an integer"
        );
        self.emit_common_instruction(Cast::Trunc(trunc), ast_type)
      }
    }
    impl<'c> Emitable<'c, FPExt> for Emitter<'c> {
      fn emit(&mut self, fpext: FPExt, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          ast_type.is_floating_point(),
          "FPExt target type must be a floating-point type"
        );
        debug_assert!(
          self.visit(fpext.operand(), |value| value
            .ir_type
            .as_floating()
            .is_some_and(|&format| format.size_bits() < ast_type.size_bits())),
          "FPExt operand must be a floating-point type"
        );
        self.emit_common_instruction(Cast::FPExt(fpext), ast_type)
      }
    }
    impl<'c> Emitable<'c, FPTrunc> for Emitter<'c> {
      fn emit(
        &mut self,
        fptrunc: FPTrunc,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type.is_floating_point(),
          "FPTrunc target type must be a floating-point type"
        );
        debug_assert!(
          self.visit(fptrunc.operand(), |value| value
            .ir_type
            .as_floating()
            .is_some_and(|&format| format.size_bits() > ast_type.size_bits())),
          "FPTrunc operand must be a floating-point type"
        );
        self.emit_common_instruction(Cast::FPTrunc(fptrunc), ast_type)
      }
    }

    impl<'c> Emitable<'c, FPToSI> for Emitter<'c> {
      fn emit(
        &mut self,
        fptosi: FPToSI,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type
            .as_primitive()
            .is_some_and(|p| p.is_signed_integer()),
          "FPToSI target type must be a signed integer"
        );
        debug_assert!(
          self.visit(fptosi.operand(), |value| value
            .ir_type
            .as_floating()
            .is_some()),
          "FPToSI operand must be a floating-point type"
        );
        self.emit_common_instruction(Cast::FPToSI(fptosi), ast_type)
      }
    }

    impl<'c> Emitable<'c, FPToUI> for Emitter<'c> {
      fn emit(
        &mut self,
        fptoui: FPToUI,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type.as_primitive().is_some_and(|p| p.is_unsigned()),
          "FPToUI target type must be an unsigned integer"
        );
        debug_assert!(
          self.visit(fptoui.operand(), |value| value
            .ir_type
            .as_floating()
            .is_some()),
          "FPToUI operand must be a floating-point type"
        );
        self.emit_common_instruction(Cast::FPToUI(fptoui), ast_type)
      }
    }

    impl<'c> Emitable<'c, UIToFP> for Emitter<'c> {
      fn emit(
        &mut self,
        uitofp: UIToFP,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type.is_floating_point(),
          "UIToFP target type must be a floating-point type"
        );
        debug_assert!(
          self.visit(uitofp.operand(), |value| value
            .ir_type
            .as_integer()
            .is_some_and(|&width| width <= ast_type.size_bits() as u8)),
          "Cannot convert an integer to a floating-point type that cannot \
           represent all values of the integer type"
        );
        debug_assert!(
          self.visit(uitofp.operand(), |value| value
            .ast_type
            .as_primitive_unchecked()
            .is_unsigned()),
          "UIToFP operand must be an unsigned integer (this would be wrong \
           though, in IR level the signedness does not matter."
        );
        self.emit_common_instruction(Cast::UIToFP(uitofp), ast_type)
      }
    }
    impl<'c> Emitable<'c, IntToPtr> for Emitter<'c> {
      fn emit(
        &mut self,
        inttoptr: IntToPtr,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type.is_pointer(),
          "IntToPtr target type must be a pointer type"
        );
        debug_assert!(
          self.visit(inttoptr.operand(), |value| value.ir_type.is_integer()),
          "IntToPtr operand must be an integer."
        );
        self.emit_common_instruction(Cast::IntToPtr(inttoptr), ast_type)
      }
    }

    impl<'c> Emitable<'c, PtrToInt> for Emitter<'c> {
      fn emit(
        &mut self,
        ptrtoint: PtrToInt,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type.is_integer(),
          "PtrToInt target type must be an integer type"
        );
        debug_assert!(
          self.visit(ptrtoint.operand(), |value| value.ir_type.is_pointer()),
          "PtrToInt operand must be a pointer."
        );
        self.emit_common_instruction(Cast::PtrToInt(ptrtoint), ast_type)
      }
    }

    impl<'c> Emitable<'c, SIToFP> for Emitter<'c> {
      fn emit(
        &mut self,
        sitofp: SIToFP,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          ast_type.is_floating_point(),
          "SIToFP target type must be a floating-point type"
        );
        debug_assert!(
          self.visit(sitofp.operand(), |value| value
            .ir_type
            .as_integer()
            .is_some_and(|&width| width <= ast_type.size_bits() as u8)),
          "Cannot convert an integer to a floating-point type that cannot \
           represent all values of the integer type"
        );
        debug_assert!(
          self.visit(sitofp.operand(), |value| value
            .ast_type
            .as_primitive_unchecked()
            .is_signed_integer()),
          "SIToFP operand must be a signed integer (this would be wrong \
           though)."
        );
        self.emit_common_instruction(Cast::SIToFP(sitofp), ast_type)
      }
    }

    impl<'c> Emitable<'c, BitCast> for Emitter<'c> {
      fn emit(
        &mut self,
        bitcast: BitCast,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          self.visit(bitcast.operand(), |value| value.ir_type.size_bits()
            == ast_type.size_bits()),
          "BitCast operand and target type must have the same size"
        );
        self.emit_common_instruction(Cast::BitCast(bitcast), ast_type)
      }
    }
  }
  mod memory {
    use super::*;
    impl<'c> Emitable<'c, Alloca> for Emitter<'c> {
      fn emit(
        &mut self,
        alloca: Alloca,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        if self.current_block.is_null() {
          panic!("no block to emit into")
        }
        let value_id = self.ir().insert(Value::new(
          ast_type,
          self.ir().pointer_type(),
          Instruction::from(Memory::from(alloca)),
          self.current_block,
        ));

        let entry_id = self
          .visit(self.current_function, |value| {
            value.data.as_function_unchecked().entry()
          })
          // shall only be called if current block is entry block
          .unwrap_or(self.current_block);
        self.apply(entry_id, |value| {
          value
            .data
            .as_basicblock_mut_unchecked()
            .instructions
            .push(value_id);
          value_id
        })
      }
    }
    impl<'c> Emitable<'c, Load> for Emitter<'c> {
      fn emit(&mut self, load: Load, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          self.visit(load.addr(), |value| value.ir_type.is_pointer()),
          "Load address must be a pointer"
        );
        self.emit_common_instruction(Memory::Load(load), ast_type)
      }
    }
    impl<'c> Emitable<'c, Store> for Emitter<'c> {
      fn emit(&mut self, store: Store, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          ast_type.is_void(),
          "Store instruction must have void type."
        );
        debug_assert!(
          self.visit(store.dest(), |value| value.ir_type.is_pointer()),
          "Store address must be a pointer"
        );
        self.emit_common_instruction(Memory::Store(store), ast_type)
      }
    }
  }
  mod misc {
    use super::*;
    impl<'c> Emitable<'c, Phi> for Emitter<'c> {
      fn emit(&mut self, phi: Phi, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          self.visit(self.current_block, |value| {
            value.data.as_basicblock_unchecked().is_empty()
          }),
          "Phi nodes must be the first instruction in a block"
        );
        debug_assert_eq!(
          self.ir().get_use_list(self.current_block).len() * 2,
          phi.flat_view().len(),
          "Phi node must cover all incoming edges of the block"
        );
        self.emit_common_instruction(phi, ast_type)
      }
    }
    impl<'c> Emitable<'c, GetElementPtr> for Emitter<'c> {
      fn emit(
        &mut self,
        gep: GetElementPtr,
        ast_type: ast::TypeRef<'c>,
      ) -> ValueID {
        debug_assert!(
          self.visit(gep.base(), |value| { value.ir_type.is_pointer() }),
          "GEP base must be a pointer"
        );
        debug_assert!(
          gep.indices().iter().all(|&idx| self.visit(idx, |value| {
            RefEq::ref_eq(value.ast_type, self.ast().ptrdiff_type())
          })),
          "GEP indices must be ptrdiff_t."
        );
        self.emit_common_instruction(gep, ast_type)
      }
    }
    impl<'c> Emitable<'c, Unary> for Emitter<'c> {
      fn emit(&mut self, unary: Unary, ast_type: ast::TypeRef<'c>) -> ValueID {
        debug_assert!(
          self.visit(unary.operand(), |value| value.ir_type.is_floating()),
          "there had only exactly one operand: FNeg, which obviously suggests \
           that the operand must be a floating-point type"
        );
        self.emit_common_instruction(unary, ast_type)
      }
    }
  }
}
impl<'c> Emitter<'c> {
  fn emit_common_instruction<T: Into<Instruction>>(
    &self,
    value: T,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    if self.current_block.is_null() {
      panic!("no block to emit into")
    }
    let value_id = self.ir().insert(Value::new(
      ast_type,
      ty!(self, ast_type),
      value.into(),
      self.current_block,
    ));
    self.apply(self.current_block, |value| {
      value
        .data
        .as_basicblock_mut_unchecked()
        .instructions
        .push(value_id);
      value_id
    })
  }

  fn emit_globals<T: Into<ValueData<'c>>>(
    &mut self,
    value: T,
    ast_type: ast::TypeRef<'c>,
  ) -> ValueID {
    let value_id = self.ir().insert(Value::new(
      ast_type,
      ty!(self, ast_type),
      value.into(),
      Default::default(),
    ));
    self.module.globals.push(value_id);
    value_id
  }

  fn emit_terminator<T: Into<inst::Terminator>>(
    &self,
    terminator: T,
    ast_type: ast::TypeRef<'c>,
    block_id: ValueID,
  ) -> ValueID {
    if block_id.is_null() {
      panic!("no block to emit terminator into")
    }

    let value_id = self.ir().insert(Value::new(
      ast_type,
      ty!(self, ast_type),
      Instruction::from(terminator.into()),
      self.current_block,
    ));

    self.apply(block_id, |value| {
      let mutref = value.data.as_basicblock_mut_unchecked();
      debug_assert!(
        mutref.terminator.is_null(),
        "block already has a terminator"
      );
      mutref.terminator = value_id;
      value_id
    })
  }
}
