#![cfg(test)]

use mux_recovery::{MuxRecovery, MuxRecoveryError};
use soroban_sdk::{symbol_short, testutils::Address as _, Env, IntoVal};

fn setup() -> (Env, MuxRecoveryClient<'static>, soroban_sdk::Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MuxRecovery);
    let client = MuxRecoveryClient::new(&env, &contract_id);
    let owner = soroban_sdk::Address::generate(&env);
    client.initialize(&owner);
    (env, client, owner)
}

#[test]
fn test_cancel_recovery_clears_pending() {
    let (env, client, _owner) = setup();
    let new_owner = soroban_sdk::Address::generate(&env);
    client.initiate_recovery(&new_owner);
    assert!(client.get_pending_recovery().is_some());

    client.cancel_recovery();
    assert!(client.get_pending_recovery().is_none());

    // Execute after cancel should fail
    env.ledger().set_sequence_number(200);
    assert!(client.try_execute_recovery().is_err());

    // Verify rec_cncl event emitted
    let events = env.events().all();
    assert!(events.iter().any(|e| {
        e.topics
            .iter()
            .any(|t| *t == symbol_short!("rec_cncl").into_val(&env))
    }));
}

#[test]
fn test_cancel_recovery_non_owner_fails() {
    let (env, client, _owner) = setup();
    let new_owner = soroban_sdk::Address::generate(&env);
    client.initiate_recovery(&new_owner);

    // Stop mocking auths — require_owner will fail
    env.mock_auths(&[]);
    assert!(client.try_cancel_recovery().is_err());
}

#[test]
fn test_add_and_remove_guardian() {
    let (env, client, _owner) = setup();
    let g1 = soroban_sdk::Address::generate(&env);

    // Add guardian
    client.add_guardian(&g1);
    assert!(client.get_guardians().contains(&g1));
    let events = env.events().all();
    assert!(events.iter().any(|e| {
        e.topics
            .iter()
            .any(|t| *t == symbol_short!("grd_add").into_val(&env))
    }));

    // Remove guardian
    client.remove_guardian(&g1);
    assert!(!client.get_guardians().contains(&g1));
    let events = env.events().all();
    assert!(events.iter().any(|e| {
        e.topics
            .iter()
            .any(|t| *t == symbol_short!("grd_rm").into_val(&env))
    }));
}

#[test]
fn test_set_quorum_threshold_rejects_above_count() {
    let (env, client, _owner) = setup();
    // No guardians yet — threshold of 1 should fail
    assert_eq!(
        client.try_set_quorum_threshold(&1_u32).unwrap_err(),
        MuxRecoveryError::InvalidThreshold.into()
    );

    // Add one guardian, then threshold of 1 succeeds
    let g = soroban_sdk::Address::generate(&env);
    client.add_guardian(&g);
    client.set_quorum_threshold(&1_u32);

    // Threshold of 2 (above guardian count) fails
    assert_eq!(
        client.try_set_quorum_threshold(&2_u32).unwrap_err(),
        MuxRecoveryError::InvalidThreshold.into()
    );

    // Verify qrm_set event emitted
    let events = env.events().all();
    assert!(events.iter().any(|e| {
        e.topics
            .iter()
            .any(|t| *t == symbol_short!("qrm_set").into_val(&env))
    }));
}
