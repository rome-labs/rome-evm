use {
    solana_program::{
        instruction::Instruction,
    },
    crate::{
        H160, pda::Seed, error::{Result, RomeProgramError::Unimplemented}, origin::Origin,
        non_evm::{Bind, NonEvmState,},
    },
    super::{
        Program,
        spl_token_ix::{Transfer as Transfer, InitAccount, balance_ge}, EvmDiff,
    },
    spl_token::{
        instruction::TokenInstruction::{self, Transfer as TransferIx, InitializeAccount3},
    },
    evm::Context,
};
#[cfg(feature = "single-state")]
use {
    super::spl_token_ix::{account_raw_state, spl_account_state}
};
//  0xae9f75e3      transfer(bytes32,bytes32,uint256) // to, mint, amount
//  0x7292100c      initialize_account3(bytes32,bytes32,bytes32)
//  0x66c1cf1e      balance_ge(address,bytes32,uint256) // caller, mint, balance

// single-state
//  0x602c2565      account_state(bytes32)
//  0x77764881      program_id()
//  0xa8109672      transfer_from(bytes32,bytes32,uint64,(bytes)[])

// TODO: fix and impl
// pub const TRANSFER_FROM_ID: &[u8] = &[0xa8, 0x10, 0x96, 0x72];
pub const TRANSFER_ID: &[u8] = &[0xae, 0x9f, 0x75, 0xe3];
pub const INIT_ACCOUNT: &[u8] = &[0x72, 0x92, 0x10, 0x0c];
pub const BALANCE_GE: &[u8] = &[0x66, 0xc1, 0xcf, 0x1e];
#[cfg(feature = "single-state")]
pub const ACCOUNT_STATE_ID: &[u8] = &[0x60, 0x2c, 0x25, 0x65];
#[cfg(feature = "single-state")]
pub const PROGRAM_ID_ID: &[u8] = &[0x77, 0x76, 0x48, 0x81];

pub struct SplToken<'a, T: Origin> {
    state: &'a T,
}

impl<'a, T: Origin> SplToken<'a, T> {
    pub const ADDRESS: H160 = H160([
        0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x05,
    ]);

    pub fn new(state: &'a T) -> Self {
        Self {
            state
        }
    }
}

impl <'a, T: Origin>Program for SplToken<'a, T> {
    fn emulate(&self, ix: &Instruction, binds: &mut Vec<Bind>) -> Result<()>  {
        match TokenInstruction::unpack(&ix.data)? {
            TransferIx { amount } => Transfer::emulate(&ix.accounts, binds, amount),
            InitializeAccount3 { owner } => InitAccount::emulate(&ix.accounts, binds, &owner),
            _ => Err(Unimplemented("instruction is not supported by SplProgram".to_string())),
        }
    }
    fn ix_from_abi(&self, abi: &[u8], context: &Context) ->Result<(Instruction, Seed, Vec<EvmDiff>)> {
        let (func, rest) = abi.split_at(4);

        match func {
            // TODO: impl TRANSFER_FROM_ID, fix parsing auth_seed using get_vec_slice()
            TRANSFER_ID => {
                let (auth, seed) = self.state.base().pda.balance_key(&context.caller);
                let ix = Transfer::new_from_abi(&rest, &auth)?;

                Ok((ix, seed, vec![]))
            },
            INIT_ACCOUNT => {
                let ix = InitAccount::new_from_abi(rest)?;
                Ok((ix, Seed::default(), vec![]))
            },
            _ => Err(Unimplemented(format!("method is not supported by SplProgram {}", hex::encode(func))))
        }
    }
    fn eth_call(&self, args: &[u8], non_evm_state: &NonEvmState) -> Result<Vec<u8>> {
        let (func, rest) = args.split_at(4);

        match func {
            BALANCE_GE => balance_ge(rest, self.state, non_evm_state),
            #[cfg(feature = "single-state")]
            ACCOUNT_STATE_ID => {
                let spl_acc = spl_account_state(rest, self.state, non_evm_state)?;
                account_raw_state(spl_acc)
            },
            #[cfg(feature = "single-state")]
            PROGRAM_ID_ID => Ok(spl_token::ID.to_bytes().to_vec()),
            _ => Err(
                Unimplemented(
                    format!("eth_call is not supported by SplProgram: {}", hex::encode(func))
                )
            )
        }

    }
    fn found_eth_call(&self, input: &[u8]) -> bool {
        let (func, _) = input.split_at(4);

        match func {
            TRANSFER_ID | INIT_ACCOUNT => false,
            BALANCE_GE => true,
            #[cfg(feature = "single-state")]
            ACCOUNT_STATE_ID | PROGRAM_ID_ID => true,
            // TODO: return revert with message
            _ => unimplemented!()
        }
    }
    fn transfer_allowed(&self) -> bool {
        false
    }
}
