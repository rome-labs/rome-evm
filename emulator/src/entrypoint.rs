#[macro_export]
macro_rules! entrypoint {
    { $($row:ident => $fn:ident),+ $(,)* } => {
        pub mod instruction {
            use super::*;
            use {
                rome_evm::{
                    error::{
                        Result,
                        RomeProgramError::UnknownInstruction,
                    },
                },
                $crate::api::Emulation,
                solana_program::{pubkey::Pubkey, msg},
                solana_client::rpc_client::RpcClient,
                std::sync::Arc,
            };

            $(
                #[allow(non_snake_case)]
                pub mod $row {
                    use super::*;

                    #[inline(never)]
                    pub fn execute<'a>(p: &'a Pubkey, d: &'a [u8], s: &'a Pubkey, c: Arc<RpcClient>) -> Result<Emulation> {
                        $fn(p, d, s, c)
                    }
                }
            )*

            #[repr(u8)]
            #[derive(Clone)]
            pub enum Instruction {
                $($row,)*
            }

            pub fn dispatch<'a>(p: &'a Pubkey, d: &'a [u8], s: &'a Pubkey, c: Arc<RpcClient>) -> Result<Emulation> {
                match d[0] {
                    $(
                        n if n == Instruction::$row as u8 => $row::execute(p, &d[1..], s, c),
                    )*

                    other => {
                        Err(UnknownInstruction(other))
                    }
                }
            }

            pub fn emulate<'a>(p: &'a Pubkey, d: &'a [u8], s: &'a Pubkey, c: Arc<RpcClient>) -> Result<Emulation> {
                msg!(">> emulator started ..");
                let res = dispatch(p, d, s, c)?;
                msg!(">> emulator finished");
                Ok(res)
            }
        }

        pub use instruction::{emulate, Instruction};
    }
}
