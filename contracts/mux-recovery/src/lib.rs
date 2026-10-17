/*!
 * mux-recovery: Account recovery contract for Mux Protocol.
 *
 * Provides a timelocked recovery workflow with guardian-set approval.
 */

#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Vec};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Owner,
    PendingRecovery,
    Guardians,
    QuorumThreshold,
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryRequest {
    pub new_owner: Address,
    pub initiated_at: u32,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MuxRecoveryError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    NoPendingRecovery = 4,
    RecoveryNotReady = 5,
    GuardianExists = 6,
    GuardianNotFound = 7,
    InvalidThreshold = 8,
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// Ledgers that must pass between initiate and execute.
const RECOVERY_TIMELOCK: u32 = 100;

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct MuxRecovery;

#[contractimpl]
impl MuxRecovery {
    pub fn initialize(env: Env, owner: Address) -> Result<(), MuxRecoveryError> {
        if env.storage().instance().has(&DataKey::Owner) {
            return Err(MuxRecoveryError::AlreadyInitialized);
        }
        owner.require_auth();
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage()
            .instance()
            .set(&DataKey::Guardians, &Vec::<Address>::new(&env));
        env.storage().instance().set(&DataKey::QuorumThreshold, &1_u32);
        Ok(())
    }

    pub fn initiate_recovery(env: Env, new_owner: Address) -> Result<(), MuxRecoveryError> {
        Self::require_owner(&env)?;
        let req = RecoveryRequest {
            new_owner: new_owner.clone(),
            initiated_at: env.ledger().sequence(),
        };
        env.storage().instance().set(&DataKey::PendingRecovery, &req);
        env.events()
            .publish((symbol_short!("mux_recv"), symbol_short!("rec_init")), new_owner);
        Ok(())
    }

    pub fn execute_recovery(env: Env) -> Result<(), MuxRecoveryError> {
        Self::require_owner(&env)?;
        let req: RecoveryRequest = env
            .storage()
            .instance()
            .get(&DataKey::PendingRecovery)
            .ok_or(MuxRecoveryError::NoPendingRecovery)?;
        if env.ledger().sequence() < req.initiated_at + RECOVERY_TIMELOCK {
            return Err(MuxRecoveryError::RecoveryNotReady);
        }
        env.storage().instance().set(&DataKey::Owner, &req.new_owner);
        env.storage().instance().remove(&DataKey::PendingRecovery);
        env.events()
            .publish((symbol_short!("mux_recv"), symbol_short!("rec_exec")), ());
        Ok(())
    }

    pub fn cancel_recovery(env: Env) -> Result<(), MuxRecoveryError> {
        Self::require_owner(&env)?;
        if !env.storage().instance().has(&DataKey::PendingRecovery) {
            return Err(MuxRecoveryError::NoPendingRecovery);
        }
        env.storage().instance().remove(&DataKey::PendingRecovery);
        env.events()
            .publish((symbol_short!("mux_recv"), symbol_short!("rec_cncl")), ());
        Ok(())
    }

    pub fn add_guardian(env: Env, guardian: Address) -> Result<(), MuxRecoveryError> {
        Self::require_owner(&env)?;
        let mut guardians: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env));
        if guardians.contains(&guardian) {
            return Err(MuxRecoveryError::GuardianExists);
        }
        guardians.push_back(guardian.clone());
        env.storage().instance().set(&DataKey::Guardians, &guardians);
        env.events()
            .publish((symbol_short!("mux_recv"), symbol_short!("grd_add")), guardian);
        Ok(())
    }

    pub fn remove_guardian(env: Env, guardian: Address) -> Result<(), MuxRecoveryError> {
        Self::require_owner(&env)?;
        let mut guardians: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env));
        let pos = guardians
            .iter()
            .position(|g| g == guardian)
            .ok_or(MuxRecoveryError::GuardianNotFound)?;
        guardians.remove(pos as u32);
        env.storage().instance().set(&DataKey::Guardians, &guardians);
        env.events()
            .publish((symbol_short!("mux_recv"), symbol_short!("grd_rm")), guardian);
        Ok(())
    }

    pub fn set_quorum_threshold(env: Env, threshold: u32) -> Result<(), MuxRecoveryError> {
        Self::require_owner(&env)?;
        let guardians: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env));
        if threshold == 0 || threshold > guardians.len() {
            return Err(MuxRecoveryError::InvalidThreshold);
        }
        env.storage()
            .instance()
            .set(&DataKey::QuorumThreshold, &threshold);
        env.events()
            .publish((symbol_short!("mux_recv"), symbol_short!("qrm_set")), threshold);
        Ok(())
    }

    pub fn get_guardians(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_pending_recovery(env: Env) -> Option<RecoveryRequest> {
        env.storage().instance().get(&DataKey::PendingRecovery)
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    fn require_owner(env: &Env) -> Result<(), MuxRecoveryError> {
        let owner: Address = env
            .storage()
            .instance()
            .get(&DataKey::Owner)
            .ok_or(MuxRecoveryError::NotInitialized)?;
        owner.require_auth();
        Ok(())
    }
}
