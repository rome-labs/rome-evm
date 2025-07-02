use {
    solana_program::{
        instruction::{Instruction,},
        system_instruction::{transfer, SystemInstruction::Transfer},
        program_utils::limited_deserialize,
        pubkey::Pubkey,
    },
    crate::{
        H160, pda::Seed, error::{Result, RomeProgramError::*}, origin::Origin,
        state::Diff, non_evm::NonEvmState, RSOL_DECIMALS,
    },
    super::{
        Program, Bind, Transfer as Transfer_, EvmDiff,
    },
    std::convert::TryFrom,
    evm::{Context, U256},
};

//  0x4d8b0ea4       withdrawal(bytes32)
pub const WITHDRAWAL_ID: &[u8] = &[0x4d, 0x8b, 0x0e, 0xa4];

pub struct Withdraw<'a, T: Origin> {
    state: &'a T,
}

impl<'a, T: Origin> Withdraw<'a, T> {
    pub const ADDRESS: H160 = H160([
        0x42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x16,
    ]);
    pub fn new(state: &'a T) -> Self {
        Self {
            state
        }
    }
}

impl<'a, T: Origin> Program for Withdraw<'a, T> {
    fn emulate(&self, ix: &Instruction, binds: &mut Vec<Bind>) -> Result<()>  {

        match limited_deserialize(&ix.data, u64::MAX).map_err(|_| InvalidNonEvmInstructionData)? {
            Transfer {lamports} => Transfer_::emulate(&ix.accounts, lamports, binds),
            _ => Err(Unimplemented("instruction is not supported by WithdrawProgram".to_string()))
        }
    }

    fn ix_from_abi(&self, abi: &[u8], context: &Context) -> Result<(Instruction, Seed, Vec<EvmDiff>)> {
        let (func, rest) = abi.split_at(4);
        match func {
            WITHDRAWAL_ID => {
                let (wallet, seed) = self.state.base().pda.sol_wallet();
                let to = Pubkey::try_from(rest).unwrap();
                let value = context.apparent_value;

                let (lamports, remainder) = value.div_mod(U256::exp10(RSOL_DECIMALS - 9));

                if !remainder.is_zero() {
                    return Err(TxValueNotMultipleOf10_9)
                }

                if lamports > U256::from(u64::MAX) {
                    return Err(TxValueExceedsU64)
                }

                let ix = transfer(&wallet, &to, lamports.as_u64());
                
                let mut diff = vec![];
                diff.push((context.caller, Diff::TransferFrom {balance: context.apparent_value}));
                diff.push((Withdraw::<'a, T>::ADDRESS, Diff::TransferTo {balance: context.apparent_value}));
                
                Ok((ix, seed, diff))
            },
            _ => Err(Unimplemented(format!("method is not supported by WithdrawProgram {}", hex::encode(func))))
        }
    }

    fn eth_call(&self, _: &[u8], _: &NonEvmState) -> Result<Vec<u8>> {
        Err(Unimplemented("eth_call is not supported by WithdrawProgram".to_string()))
    }

    fn found_eth_call(&self, _: &[u8]) -> bool {
        false
    }

    fn transfer_allowed(&self) -> bool {
        true
    }
}

