use std::mem::size_of;

use anchor_lang::prelude::*;

use crate::MAX_VALIDATORS_IN_LIST;

#[account]
#[derive(Default)]
pub struct Pool {
    pub admin: Pubkey,
    pub start_slot: u64,
    pub start_epoch: u64,
    pub bump: u8,
}

static_assertions::const_assert_eq!(size_of::<Pool>(), 56);

impl Pool {
    pub const SEED: &'static [u8] = b"pool";
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn pubkey(admin: Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[Self::SEED, admin.as_ref()], &crate::ID).0
    }

    pub fn init(
        &mut self,
        admin: &Pubkey,
        start_slot: u64,
        start_epoch: u64,
        bump: u8,
    ) -> Result<()> {
        self.admin = *admin;
        self.start_slot = start_slot;
        self.start_epoch = start_epoch;
        self.bump = bump;

        Ok(())
    }
}

#[account]
pub struct VList {
    pub validators: [VListEntry; MAX_VALIDATORS_IN_LIST], // validators that are in top 10%
    pub idx: usize,
    pub admin: Pubkey,
    pub pool: Pubkey,
    pub bump: u8,
    padding: [u8; 7],
}

#[derive(AnchorDeserialize, AnchorSerialize, Default, Clone, Copy)]
pub struct VListEntry {
    pub validator: Pubkey,
    pub last_scored_epoch: u64,
    pub score: u32,
}

static_assertions::const_assert_eq!(size_of::<VList>(), 4880);

impl VList {
    pub const SEED: &'static [u8] = b"v_list";
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn pubkey(admin: Pubkey, pool: Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[Self::SEED, admin.as_ref(), pool.as_ref()], &crate::ID).0
    }

    pub fn init(&mut self, admin: &Pubkey, pool: Pubkey, bump: u8) -> Result<()> {
        self.admin = *admin;
        self.bump = bump;
        self.validators = [VListEntry::default(); MAX_VALIDATORS_IN_LIST];
        self.idx = 0;
        self.pool = pool;

        Ok(())
    }

    pub fn insert_or_update(
        &mut self,
        validator_pubkey: Pubkey,
        new_score: u32,
        current_epoch: u64,
    ) {
        // Search for the validator in the list
        let mut found = false;
        for i in 0..self.idx {
            if self.validators[i].validator == validator_pubkey {
                self.validators[i].score = new_score;
                self.validators[i].last_scored_epoch = current_epoch;
                found = true;
                break;
            }
        }

        // If not found, add a new validator entry
        if !found && self.idx < MAX_VALIDATORS_IN_LIST {
            self.validators[self.idx] = VListEntry {
                validator: validator_pubkey,
                last_scored_epoch: current_epoch,
                score: new_score,
            };

            self.idx += 1;
        }

        self.validators[0..self.idx].sort_by(|a, b| b.score.cmp(&a.score));
    }
}

impl TryFrom<Vec<u8>> for VList {
    type Error = Error;
    fn try_from(data: Vec<u8>) -> std::result::Result<Self, Self::Error> {
        VList::try_deserialize(&mut data.as_slice())
    }
}
