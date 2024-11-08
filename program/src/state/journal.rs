use {
    crate::{
        allocate::Allocate, context::account_lock::AccountLock, error::Result, origin::Origin,
        NUMBER_ALLOC_DIFF_PER_TX,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    evm::{H160, H256, U256},
    solana_program::msg,
    std::collections::{BTreeMap, HashSet},
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
    pub fn commit<T: Origin, L: AccountLock>(&self, state: &T, context: &L) -> Result<()> {
        if let Some(parent) = self.parent.as_ref() {
            parent.commit(state, context)?
        }

        // todo: replace BtreeMap by Vec
        // state's diffs should be applied in original order. They should not mix
        for (address, diffs) in &self.diff {
            for diff in diffs {
                match diff {
                    Diff::NonceChange => {
                        msg!("NonceChange");
                        state.inc_nonce(address, context)?;
                    }
                    Diff::CodeChange { code, valids } => {
                        state.set_code(address, code, valids, context)?;
                        msg!("contract is deployed");
                    }
                    Diff::StorageChange { key, value } => {
                        state.set_storage(address, key, value, context)?;
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
                        state.sub_balance(address, balance, context)?;
                    }
                    Diff::TransferTo { balance } => {
                        msg!("TransferTo");
                        state.add_balance(address, balance, context)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn alloc_balances<T: Allocate + Origin, L: AccountLock>(
        &self,
        state: &T,
        context: &L,
    ) -> Result<bool> {
        if !self
            .parent
            .as_ref()
            .map_or(Ok(true), |parent| parent.alloc_balances(state, context))?
        {
            return Ok(false);
        }

        for (address, diffs) in &self.diff {
            for diff in diffs {
                // alloc_limit should be enough to allocate the AccountState
                if state.syscalls() >= NUMBER_ALLOC_DIFF_PER_TX || state.alloc_limit() < 500 {
                    return Ok(false);
                }

                match diff {
                    // TODO check allocation limit
                    Diff::NonceChange
                    | Diff::TransferFrom { balance: _ }
                    | Diff::TransferTo { balance: _ } => {
                        state.alloc_balance(address, context)?;
                    }
                    Diff::CodeChange { code, valids } => {
                        if !state.alloc_contract(address, code, valids, context)? {
                            return Ok(false);
                        }
                    }
                    Diff::StorageChange { key, value: _ } => {
                        // just to calculate and cache the hash
                        let (_, _, _) = state.slot_to_key(address, key);
                    }
                    _ => {}
                };
            }
        }

        Ok(true)
    }
    pub fn diff_len(&self) -> usize {
        let parent = self.parent.as_ref().map_or(0, |parent| parent.diff_len());

        parent + self.diff.values().fold(0, |s, a| s + a.len())
    }

    pub fn merge_slots<T: Origin>(&self, state: &T) -> Result<BTreeMap<H160, HashSet<U256>>> {
        let parent = self
            .parent
            .as_ref()
            .map_or(Ok(BTreeMap::new()), |parent| parent.merge_slots(state))?;

        let mut new = self
            .diff
            .iter()
            .map(|(address, diffs)| {
                let slots = diffs
                    .iter()
                    .filter_map(|diff| match diff {
                        Diff::StorageChange {
                            key: slot,
                            value: _,
                        } => {
                            if let Ok(Some(_)) = state.storage(address, slot) {
                                None
                            } else {
                                Some(*slot)
                            }
                        }
                        _ => None,
                    })
                    .collect::<HashSet<U256>>();

                (*address, slots)
            })
            .collect::<BTreeMap<H160, HashSet<U256>>>();

        // merge from parent
        parent.into_iter().for_each(|(address, set)| {
            if let Some(new_set) = new.get_mut(&address) {
                new_set.extend(set);
            } else {
                new.insert(address, set);
            }
        });

        Ok(new)
    }
}
