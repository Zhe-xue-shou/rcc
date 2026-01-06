use ::bitflags::bitflags;

bitflags! {
  #[derive(Debug,Clone,Copy,PartialEq,Eq)]
  pub struct FunctionSpecifier : u8 {
    const Inline = 0x01;
    const Noreturn = 0x10;
  }
}
