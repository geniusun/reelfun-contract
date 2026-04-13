use anchor_lang::prelude::*;

/// Stake Window for a specific episode
#[account]
#[derive(InitSpace, Debug)]
pub struct StakeWindow {
    pub drama_id: Pubkey,
    pub episode_id: u64,
    pub bonding_curve: Pubkey,
    pub creator: Pubkey,
    pub open_time: i64,
    pub close_time: i64,
    pub is_closed: bool,
    pub total_stake_weight: u128,
    pub bump: u8,
}

impl StakeWindow {
    pub const SEED_PREFIX: &'static str = "stake_window";

    pub fn is_open(&self, clock: &Clock) -> bool {
        !self.is_closed && clock.unix_timestamp >= self.open_time && clock.unix_timestamp < self.close_time
    }

    pub fn can_close(&self, clock: &Clock) -> bool {
        !self.is_closed && clock.unix_timestamp >= self.close_time
    }
}

/// User stake for a specific episode
#[account]
#[derive(InitSpace, Debug)]
pub struct UserStake {
    pub stake_window: Pubkey,
    pub user: Pubkey,
    pub bonding_curve: Pubkey,
    pub stake_amount: u64,
    pub lock_days: u16,
    pub stake_weight: u128,
    pub normalized_weight: u128, // After 30% cap applied
    #[max_len(500)]
    pub prompt: String,
    pub deposit_paid: u64, // 0.01 SOL in lamports
    pub stake_time: i64,
    pub unlock_time: i64,
    pub is_unlocked: bool,
    pub deposit_refunded: bool,
    pub bump: u8,
}

impl UserStake {
    pub const SEED_PREFIX: &'static str = "user_stake";
    pub const DEPOSIT_AMOUNT: u64 = 10_000_000; // 0.01 SOL

    pub fn calculate_weight(stake_amount: u64, lock_days: u16) -> u128 {
        (stake_amount as u128) * (lock_days as u128)
    }

    pub fn can_unlock(&self, clock: &Clock) -> bool {
        !self.is_unlocked && clock.unix_timestamp >= self.unlock_time
    }

    pub fn can_refund_deposit(&self, clock: &Clock, generation_complete_time: i64) -> bool {
        !self.deposit_refunded
            && generation_complete_time > 0
            && clock.unix_timestamp >= generation_complete_time + 86400 // 24 hours after generation
    }
}

/// Parameters for opening a stake window
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct OpenStakeWindowParams {
    pub drama_id: Pubkey,
    pub episode_id: u64,
    pub window_duration_hours: u16, // 24-48 hours
}

/// Parameters for staking
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct StakeParams {
    pub drama_id: Pubkey,
    pub episode_id: u64,
    pub stake_amount: u64,
    pub lock_days: u16, // 1, 3, 7, 30, or custom
    pub prompt: String,
}
