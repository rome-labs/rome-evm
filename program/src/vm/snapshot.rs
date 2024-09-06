use {
    crate::error::Result,
    borsh::{BorshDeserialize, BorshSerialize},
    evm::{Runtime, H160},
};

#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
pub enum Reason {
    Call,
    Create(H160),
}

pub struct Snapshot {
    pub evm: Runtime,
    pub reason: Reason,
    pub mutable: bool,
    pub parent: Option<Box<Snapshot>>,
}

impl Snapshot {
    pub fn is_mut(&self) -> bool {
        if self.mutable {
            if let Some(parent) = self.parent.as_ref() {
                parent.is_mut()
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn depth(&self) -> usize {
        let depth = if let Some(parent) = self.parent.as_ref() {
            parent.depth()
        } else {
            0
        };
        depth + 1
    }
    pub fn serialize_recursive(&self, into: &mut &mut [u8]) -> Result<()> {
        if let Some(parent) = self.parent.as_ref() {
            parent.serialize_recursive(into)?
        }

        self.evm.serialize(into)?;
        self.reason.serialize(into)?;
        self.mutable.serialize(into)?;
        Ok(())
    }
    pub fn serialize(snapshot: &Option<Box<Self>>, into: &mut &mut [u8]) -> Result<()> {
        if let Some(snapshot) = snapshot {
            let depth = snapshot.depth();
            depth.serialize(into)?;
            snapshot.serialize_recursive(into)?;
        } else {
            0_usize.serialize(into)?;
        }

        Ok(())
    }

    pub fn deserialize(from: &mut &[u8]) -> Result<Option<Box<Self>>> {
        let mut snapshot = None;
        let depth: usize = BorshDeserialize::deserialize(from)?;

        for _ in 0..depth {
            let evm: Runtime = BorshDeserialize::deserialize(from)?;
            let reason: Reason = BorshDeserialize::deserialize(from)?;
            let mutable: bool = BorshDeserialize::deserialize(from)?;

            snapshot = Some(Box::new(Self {
                evm,
                reason,
                mutable,
                parent: snapshot,
            }));
        }
        Ok(snapshot)
    }
}
