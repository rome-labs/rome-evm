use {
    solana_program::{
        instruction::Instruction,
    },
    rome_evm::{
        pda::Seed, error::Result,
        non_evm::{Bind, NonEvmState, Program, EvmDiff}, Context
    },
};

pub struct AltProgram {}

impl Program for AltProgram {
    fn emulate(&self, _: &Instruction, _: &mut Vec<Bind>) -> Result<()> {
        Ok(())
    }
    fn ix_from_abi(&self, _: &[u8], _: &Context) -> Result<(Instruction, Seed, Vec<EvmDiff>)> {
        unimplemented!()
    }
    fn eth_call(&self, _: &[u8], _: &NonEvmState) -> Result<Vec<u8>> {
        unimplemented!()
    }
    fn found_eth_call(&self, _: &[u8]) -> bool {
        unimplemented!()
    }
    fn transfer_allowed(&self) -> bool {
        unimplemented!()
    }
}
