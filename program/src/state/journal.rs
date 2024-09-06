use solana_program::msg;
use {
    crate::{allocate::Allocate, error::Result, origin::Origin},
    borsh::{BorshDeserialize, BorshSerialize},
    evm::{H160, H256, U256},
    std::collections::BTreeMap,
};

/// Journal entries that are used to track changes to the state and are used to revert it.
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(dead_code)]
#[derive(BorshSerialize, BorshDeserialize)]
pub enum Diff {
    /// Used to mark account that is warm inside EVM in regards to EIP-2929 AccessList.
    /// Action: We will add Account to state.
    /// Revert: we will remove account from state.
    // AccountLoaded { address: Address },
    /// Mark account to be destroyed and journal balance to be reverted
    /// Action: Mark account and transfer the balance
    /// Revert: Unmark the account and transfer balance back
    // AccountDestroyed {
    //     address: Address,
    //     target: Address,
    //     was_destroyed: bool, // if account had already been destroyed before this journal entry
    //     had_balance: U256,
    // },
    /// Loading account does not mean that account will need to be added to MerkleTree (touched).
    /// Only when account is called (to execute contract or transfer balance) only then account is made touched.
    /// Action: Mark account touched
    /// Revert: Unmark account touched
    // AccountTouched { address: Address },
    /// Transfer balance between two accounts
    /// Action: Transfer balance
    /// Revert: Transfer balance back
    TransferFrom {
        balance: U256,
    },
    TransferTo {
        balance: U256,
    },
    /// Increment nonce
    /// Action: Increment nonce by one
    /// Revert: Decrement nonce by one
    NonceChange,
    /// Create account:
    /// Actions: Mark account as created
    /// Revert: Unmart account as created and reset nonce to zero.
    // AccountCreated { address: H160 },
    /// It is used to track both storage change and warm load of storage slot. For warm load in regard
    /// to EIP-2929 AccessList had_value will be None
    /// Action: Storage change or warm load
    /// Revert: Revert to previous value or remove slot from storage
    StorageChange {
        key: U256,
        value: U256,
    },
    /// It is used to track an EIP-1153 transient storage change.
    /// Action: Transient storage changed.
    /// Revert: Revert to previous value.
    // TransientStorageChange {
    //     key: U256,
    //     had_value: U256,
    // },
    /// Code changed
    /// Action: Account code changed
    /// Revert: Revert to previous bytecode.
    CodeChange {
        code: Vec<u8>,
        valids: Vec<u8>,
    },
    Suicide,
    Event {
        topics: Vec<H256>,
        data: Vec<u8>,
    },
}
#[derive(Default)]
pub struct Journal {
    pub diff: BTreeMap<H160, Vec<Diff>>,
    pub parent: Option<Box<Journal>>,
}

impl Journal {
    pub fn new() -> Self {
        Self {
            diff: BTreeMap::new(),
            parent: None,
        }
    }

    pub fn next_page(parent: Box<Journal>) -> Self {
        Self {
            diff: BTreeMap::new(),
            parent: Some(parent),
        }
    }

    pub fn nonce_diff(&self, address: &H160) -> u64 {
        let mut nonce = if let Some(parent) = &self.parent {
            parent.nonce_diff(address)
        } else {
            0
        };

        if let Some(items) = self.diff.get(address) {
            for item in items {
                if let Diff::NonceChange = item {
                    nonce += 1;
                }
            }
        }
        nonce
    }

    pub fn transfer_from(&self, address: &H160) -> U256 {
        let mut value = if let Some(parent) = &self.parent {
            parent.transfer_from(address)
        } else {
            U256::zero()
        };

        if let Some(items) = self.diff.get(address) {
            for item in items {
                if let Diff::TransferFrom { balance } = item {
                    value += *balance
                }
            }
        }

        value
    }

    pub fn transfer_to(&self, address: &H160) -> U256 {
        let mut value = if let Some(parent) = &self.parent {
            parent.transfer_to(address)
        } else {
            U256::zero()
        };

        if let Some(items) = self.diff.get(address) {
            for item in items {
                if let Diff::TransferTo { balance } = item {
                    value += *balance
                }
            }
        }

        value
    }

    pub fn get_mut(&mut self, address: &H160) -> &mut Vec<Diff> {
        self.diff.entry(*address).or_default()
    }

    pub fn code_valids_diff(&self, address: &H160) -> Option<(&Vec<u8>, &Vec<u8>)> {
        if let Some(items) = self.diff.get(address) {
            for item in items.iter().rev() {
                if let Diff::CodeChange { code, valids } = item {
                    return Some((code, valids));
                }
            }
        }

        self.parent
            .as_ref()
            .and_then(|parent| parent.code_valids_diff(address))
    }

    pub fn storage_diff(&self, address: &H160, index: &U256) -> Option<U256> {
        if let Some(items) = self.diff.get(address) {
            for item in items.iter().rev() {
                if let Diff::StorageChange { key, value } = item {
                    if key == index {
                        return Some(*value);
                    }
                }
            }
        }

        self.parent
            .as_ref()
            .and_then(|parent| parent.storage_diff(address, index))
    }

    pub fn depth(&self) -> usize {
        let depth = if let Some(parent) = self.parent.as_ref() {
            parent.depth()
        } else {
            0
        };
        depth + 1
    }

    fn serialize_recursive(&self, into: &mut &mut [u8]) -> Result<()> {
        if let Some(parent) = self.parent.as_ref() {
            parent.serialize_recursive(into)?
        }

        self.diff.serialize(into)?;
        Ok(())
    }
    pub fn serialize(&self, into: &mut &mut [u8]) -> Result<()> {
        let depth = self.depth();
        depth.serialize(into)?;
        self.serialize_recursive(into)
    }
    pub fn deserialize(from: &mut &[u8]) -> Result<Self> {
        let mut journal = None;
        let depth: usize = BorshDeserialize::deserialize(from)?;
        assert!(depth > 0);

        for _ in 0..depth {
            let diff: BTreeMap<H160, Vec<Diff>> = BorshDeserialize::deserialize(from)?;

            journal = Some(Box::new(Self {
                diff,
                parent: journal,
            }));
        }

        Ok(*journal.expect("journal expected"))
    }
    pub fn commit<T: Origin>(&self, state: &T) -> Result<()> {
        if let Some(parent) = self.parent.as_ref() {
            parent.commit(state)?
        }

        // todo: replace BtreeMap by Vec
        // state's diffs should be applied in original order. They should not mix
        for (address, diffs) in &self.diff {
            for diff in diffs {
                match diff {
                    Diff::NonceChange => {
                        msg!("NonceChange");
                        state.inc_nonce(address)?;
                    }
                    Diff::CodeChange { code, valids } => {
                        msg!("CodeChange");
                        state.set_code(address, code, valids)?;
                        msg!("contract is deployed");
                    }
                    Diff::StorageChange { key, value } => {
                        msg!("StorageChange");
                        state.set_storage(address, key, value)?;
                    }
                    Diff::Suicide => {
                        todo!()
                    }
                    Diff::Event { topics, data } => {
                        msg!("SetLogs");
                        state.set_logs(address, topics, data)?;
                    }
                    Diff::TransferFrom { balance } => {
                        msg!("TransferFrom");
                        state.sub_balance(address, balance)?;
                    }
                    Diff::TransferTo { balance } => {
                        msg!("TransferTo");
                        state.add_balance(address, balance)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn allocate<T: Allocate>(&self, state: &T) -> Result<bool> {
        if let Some(parent) = self.parent.as_ref() {
            if !parent.allocate(state)? {
                return Ok(false);
            }
        }

        for (address, diffs) in &self.diff {
            for diff in diffs {
                match diff {
                    // TODO check allocation limit
                    Diff::NonceChange
                    | Diff::TransferFrom { balance: _ }
                    | Diff::TransferTo { balance: _ } => {
                        state.allocate_balance(address)?;
                    }
                    Diff::StorageChange { key, value: _ } => {
                        state.allocate_storage(address, key)?;
                    }
                    Diff::CodeChange { code, valids } => {
                        if !state.allocate_contract(address, code, valids)? {
                            return Ok(false);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(true)
    }
}
