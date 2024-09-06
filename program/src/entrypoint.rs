#[macro_export]
macro_rules! entrypoint {
    { $($row:ident => $fn:ident),+ $(,)* } => {
        pub mod instruction {
            use super::*;
            use error::{
                Result,
                RomeProgramError,
            };
            use $crate::assert::asserts;
            use solana_program::{
                account_info::AccountInfo,
                entrypoint::ProgramResult,
                pubkey::Pubkey,
            };

            $(
                #[allow(non_snake_case)]
                pub mod $row {
                    use super::*;

                    #[inline(never)]
                    pub fn execute<'a>(p: &'a Pubkey, a: &'a [AccountInfo<'a>], d: &'a [u8]) -> Result<()> {
                        $fn(p, a, d)?;
                        Ok(())
                    }
                }
            )*

            #[repr(u8)]
            #[derive(Clone)]
            pub enum Instruction {
                $($row,)*
            }

            pub fn dispatch<'a>(p: &'a Pubkey, a: &'a [AccountInfo<'a>], d: &'a [u8]) -> Result<()> {
                match d[0] {
                    $(
                        n if n == Instruction::$row as u8 => $row::execute(p, a, &d[1..]),
                    )*

                    other => {
                        Err(RomeProgramError::UnknownInstruction(other))
                    }
                }
            }

            pub fn declare<'a>(p: &'a Pubkey, a: &'a [AccountInfo<'a>], d: &'a [u8]) -> ProgramResult {
                asserts();
                if let Err(err) = dispatch(p, a, d) {
                    solana_program::msg!("Error: {:?}", err);
                    return Err(err.into());
                }
                Ok(())
            }
        }

        pub use instruction::{declare, Instruction};
        #[cfg(not(feature = "no-entrypoint"))]
        solana_program::entrypoint!(declare);
    }
}
