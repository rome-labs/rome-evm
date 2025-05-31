use {
    solana_program::{
        instruction::{Instruction,},
        system_instruction::SystemInstruction::{
            CreateAccount, Assign, Allocate, Transfer,
        },
        program_utils::limited_deserialize,
    },
    crate::{
        H160, pda::Seed, error::{Result, RomeProgramError::*}, origin::Origin,
    },
    super::{
        Program, CreateA, Bind, Allocate as Allocate_, Assign as Assign_, Transfer as Transfer_,
        EvmDiff,
    },
    evm::Context,
};
//  0x27e3edda       find_program_address(bytes32,(bytes)[])
//  0xd5683ff7       create_account(bytes32,uint64,address,bytes32) owner, len, derived_from, salt
//  0xfb2ca9b1       allocate(bytes32,uint64)        key, len
//  0x8ac00bdc       assign(bytes32,bytes32)      key, owner
//  0x875abfc0       transfer(bytes32,uint64,bytes32)   to, amount, salt
//  0x77764881       program_id()
//  0xb76fd45b       rome_evm_program_id()
//  0xfa2b1a5f       bytes32_to_base58(bytes32)
//  0x5df01b72       base58_to_bytes32(bytes)

pub const CREATE_ACCOUNT_ID: &[u8] = &[0xd5, 0x68, 0x3f, 0xf7];
pub const ALLOCATE_ID: &[u8] = &[0xfb, 0x2c, 0xa9, 0xb1];
pub const ASSIGN_ID: &[u8] = &[0x8a, 0xc0, 0x0b, 0xdc];
pub const TRANSFER_ID: &[u8] = &[0x87, 0x5a, 0xbf, 0xc0];


pub struct System<'a, T: Origin> {
    state: &'a T,
}

impl<'a, T: Origin> System<'a, T> {
    pub const ADDRESS: H160 = H160([
        0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x07,
    ]);
    pub fn new(state: &'a T) -> Self {
        Self {
            state
        }
    }
}

impl<'a, T: Origin> Program for System<'a, T> {
    fn emulate(&self, ix: &Instruction, binds: Vec<Bind>) -> Result<()>  {

        match limited_deserialize(&ix.data, u64::MAX).map_err(|_| InvalidNonEvmInstructionData)? {

            CreateAccount{lamports, space, owner} =>
                CreateA::emulate(lamports, space, &owner, binds ),
            Allocate {space} => Allocate_::emulate(space, binds),
            Assign {owner} => Assign_::emulate(&owner, binds),
            Transfer {lamports} => Transfer_::emulate(ix, lamports, binds),
            _ => unimplemented!()
        }
    }

    fn ix_from_abi(&self, abi: &[u8], context: &Context) ->Result<(Instruction, Seed, Vec<EvmDiff>)> {
        let (func, rest) = abi.split_at(4);
        match func {
            CREATE_ACCOUNT_ID => {
                let (ix, seed) = CreateA::new_from_abi(self.state, rest)?;
                Ok((ix, seed, vec![]))
            },
            ALLOCATE_ID => {
                let ix = Allocate_::new_from_abi(rest)?;
                Ok((ix, Seed::default(), vec![]))
            }
            ASSIGN_ID => {
                let ix = Assign_::new_from_abi(rest)?;
                Ok((ix, Seed::default(), vec![]))
            }
            TRANSFER_ID => {
                let (ix, seed) = Transfer_::new_from_abi(self.state, &context.caller, rest)?;
                Ok((ix, seed, vec![]))
            },
            _ => unimplemented!()
        }
    }
    #[cfg(feature = "single-state")]
    fn eth_call(&self, input: &[u8], state: &NonEvmState) -> Result<Vec<u8>> {
        self.eth_call_(input, state)
    }
    #[cfg(feature = "single-state")]
    fn found_eth_call(&self, input: &[u8]) -> bool {
        self.found_eth_call_(input)
    }
}

#[cfg(feature = "single-state")]
mod single_state_mod {
    use {
        solana_program::system_program,
        crate::{
            non_evm::{
                System, system::{
                    ALLOCATE_ID, ASSIGN_ID, CREATE_ACCOUNT_ID, TRANSFER_ID,
                }
            },
            error::Result,
            origin::Origin,
        },
        crate::non_evm::{
            system_ix::{find_pda, bytes32_to_base58, base58_to_bytes32,},
        }
    };
    pub use crate::non_evm::NonEvmState;

    pub const FIND_PDA_ID: &[u8] = &[0x27, 0xe3, 0xed, 0xda];
    pub const PROGRAM_ID_ID: &[u8] = &[0x77, 0x76, 0x48, 0x81];
    pub const ROME_EVM_PROGRAM_ID_ID: &[u8] = &[0xb7, 0x6f, 0xd4, 0x5b];
    pub const BYTES32_TO_BASE58_ID: &[u8] = &[0xfa, 0x2b, 0x1a, 0x5f];
    pub const BASE58_TO_BYTES32_ID: &[u8] = &[0x5d, 0xf0, 0x1b, 0x72];

    impl<'a, T: Origin> System<'a, T> {
        pub fn eth_call_(&self, args: &[u8], _: &NonEvmState) -> Result<Vec<u8>> {
            let (func, rest) = args.split_at(4);

            match func {
                FIND_PDA_ID => find_pda(rest),
                PROGRAM_ID_ID => Ok(system_program::ID.to_bytes().to_vec()),
                ROME_EVM_PROGRAM_ID_ID => Ok(self.state.base().program_id.to_bytes().to_vec()),
                BYTES32_TO_BASE58_ID => bytes32_to_base58(rest),
                BASE58_TO_BYTES32_ID => base58_to_bytes32(rest),
                _ => unimplemented!()
            }
        }

        pub fn found_eth_call_(&self, input: &[u8]) -> bool {
            let (func, _) = input.split_at(4);

            match func {
                FIND_PDA_ID | PROGRAM_ID_ID | ROME_EVM_PROGRAM_ID_ID | BYTES32_TO_BASE58_ID
                | BASE58_TO_BYTES32_ID => true,
                CREATE_ACCOUNT_ID | ALLOCATE_ID | ASSIGN_ID | TRANSFER_ID => false,
                _ => unimplemented!()
            }
        }
    }
}
#[cfg(feature = "single-state")]
pub use single_state_mod::*;

