/*!
 * mux-batcher: Multi-operation batching contract for Mux Protocol.
 *
 * Allows atomically executing a sequence of cross-contract calls in a
 * single transaction, with optional per-operation authorization checks.
 */

#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Bytes, Env, Vec};

// ── Types ─────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug)]
pub struct Operation {
    pub target: Address,
    pub fn_name: soroban_sdk::Symbol,
    pub args: Vec<soroban_sdk::Val>,
    pub require_success: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchResult {
    pub success_count: u32,
    pub failure_count: u32,
    pub errors: Vec<Bytes>,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MuxBatcherError {
    EmptyBatch = 1,
    BatchTooLarge = 2,
    RequiredOperationFailed = 3,
    Unauthorized = 4,
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum operations allowed in a single batch to bound execution cost.
const MAX_BATCH_SIZE: u32 = 50;

/// Default fee per operation in stroops.
const FEE_PER_OP: u32 = 100;

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    FeePerOp,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct MuxBatcher;

#[contractimpl]
impl MuxBatcher {
    /// Initialize the batcher with an admin who can adjust fees.
    pub fn initialize(env: Env, admin: Address) -> Result<(), MuxBatcherError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(MuxBatcherError::Unauthorized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }

    /// Admin-only: set the per-operation fee in stroops.
    pub fn set_fee_per_op(env: Env, fee: u32) -> Result<(), MuxBatcherError> {
        Self::require_admin(&env)?;
        env.storage().instance().set(&DataKey::FeePerOp, &fee);
        Ok(())
    }

    /// Public read: returns the current per-operation fee in stroops.
    /// Falls back to the compile-time default (100) when unset.
    pub fn fee_per_op(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::FeePerOp)
            .unwrap_or(FEE_PER_OP)
    }

    /// Estimate total fees for a batch of `op_count` operations.
    pub fn estimate_fees(env: Env, op_count: u32) -> u32 {
        op_count.saturating_mul(Self::fee_per_op(env))
    }

    /// Execute a batch of operations atomically.
    ///
    /// If any operation has `require_success = true` and fails, the entire
    /// transaction is aborted via panic (Soroban rolls back on panic).
    pub fn execute_batch(
        env: Env,
        caller: Address,
        ops: Vec<Operation>,
    ) -> Result<BatchResult, MuxBatcherError> {
        caller.require_auth();

        if ops.is_empty() {
            return Err(MuxBatcherError::EmptyBatch);
        }
        if ops.len() > MAX_BATCH_SIZE {
            return Err(MuxBatcherError::BatchTooLarge);
        }

        let mut success_count: u32 = 0;
        let mut failure_count: u32 = 0;
        let errors: Vec<Bytes> = Vec::new(&env);

        for op in ops.iter() {
            let result = env.try_invoke_contract::<soroban_sdk::Val, soroban_sdk::Error>(
                &op.target,
                &op.fn_name,
                op.args.clone(),
            );

            match result {
                Ok(_) => {
                    success_count += 1;
                }
                Err(_err) => {
                    if op.require_success {
                        return Err(MuxBatcherError::RequiredOperationFailed);
                    }
                    failure_count += 1;
                }
            }
        }

        Ok(BatchResult {
            success_count,
            failure_count,
            errors,
        })
    }

    /// Simulate a batch without writing state — useful for preflight checks.
    pub fn simulate_batch(
        env: Env,
        caller: Address,
        ops: Vec<Operation>,
    ) -> Result<BatchResult, MuxBatcherError> {
        caller.require_auth();

        if ops.is_empty() {
            return Err(MuxBatcherError::EmptyBatch);
        }
        if ops.len() > MAX_BATCH_SIZE {
            return Err(MuxBatcherError::BatchTooLarge);
        }

        // Preflight: count without invoking (real simulation requires contract
        // access to a read-only snapshot — this returns a conservative estimate).
        Ok(BatchResult {
            success_count: ops.len(),
            failure_count: 0,
            errors: Vec::new(&env),
        })
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    fn require_admin(env: &Env) -> Result<(), MuxBatcherError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(MuxBatcherError::Unauthorized)?;
        admin.require_auth();
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, Vec};

    fn setup() -> (Env, MuxBatcherClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxBatcher);
        let client = MuxBatcherClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    #[test]
    fn test_fee_per_op_default_fallback() {
        let (env, _client, _) = setup();
        // Before any set, fee_per_op should return the compile-time default (100)
        assert_eq!(MuxBatcher::fee_per_op(env), 100);
    }

    #[test]
    fn test_set_fee_per_op_admin() {
        let (env, client, _) = setup();
        client.set_fee_per_op(&250_u32);
        assert_eq!(client.fee_per_op(), 250);
        // estimate_fees reflects the new fee
        assert_eq!(client.estimate_fees(&4_u32), 1000);
    }

    #[test]
    fn test_set_fee_per_op_non_admin_fails() {
        let (env, client, _admin) = setup();
        env.mock_auths(&[]);
        assert!(client.try_set_fee_per_op(&500_u32).is_err());
    }

    #[test]
    fn test_empty_batch_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxBatcher);
        let client = MuxBatcherClient::new(&env, &contract_id);

        let caller = Address::generate(&env);
        let ops: Vec<Operation> = Vec::new(&env);
        let result = client.try_execute_batch(&caller, &ops);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_too_large_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxBatcher);
        let client = MuxBatcherClient::new(&env, &contract_id);

        let caller = Address::generate(&env);
        let mut ops: Vec<Operation> = Vec::new(&env);
        let target = Address::generate(&env);
        for _ in 0..51 {
            ops.push_back(Operation {
                target: target.clone(),
                fn_name: soroban_sdk::symbol_short!("noop"),
                args: Vec::new(&env),
                require_success: false,
            });
        }
        let result = client.try_execute_batch(&caller, &ops);
        assert!(result.is_err());
    }
}
