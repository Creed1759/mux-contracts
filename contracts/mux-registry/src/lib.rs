/*!
 * mux-registry: Contract version registry for Mux Protocol.
 */

#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol, Vec};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    Version(Symbol),
    Names,
    Metadata(Symbol),
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MuxRegistryError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    ContractNotFound = 4,
    TooManyContracts = 5,
}

// ── Constants ─────────────────────────────────────────────────────────────────

const MAX_CONTRACTS: u32 = 128;

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct MuxRegistry;

#[contractimpl]
impl MuxRegistry {
    pub fn initialize(env: Env, admin: Address) -> Result<(), MuxRegistryError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(MuxRegistryError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Names, &Vec::<Symbol>::new(&env));
        env.events()
            .publish((symbol_short!("mux_reg"), symbol_short!("init")), admin);
        Ok(())
    }

    /// Register or update a contract version. Admin only.
    pub fn register(env: Env, name: Symbol, version: String) -> Result<(), MuxRegistryError> {
        Self::require_admin(&env)?;
        let mut names: Vec<Symbol> = env
            .storage()
            .instance()
            .get(&DataKey::Names)
            .unwrap_or_else(|| Vec::new(&env));
        if !names.contains(&name) {
            if names.len() >= MAX_CONTRACTS {
                return Err(MuxRegistryError::TooManyContracts);
            }
            names.push_back(name.clone());
            env.storage().instance().set(&DataKey::Names, &names);
        }
        env.storage()
            .instance()
            .set(&DataKey::Version(name.clone()), &version);
        env.events()
            .publish((symbol_short!("mux_reg"), symbol_short!("reg")), (name, version));
        Ok(())
    }

    /// Register a contract with metadata (description and author). Admin only.
    pub fn register_with_metadata(
        env: Env,
        name: Symbol,
        version: String,
        description: String,
        author: String,
    ) -> Result<(), MuxRegistryError> {
        Self::register(env.clone(), name.clone(), version.clone())?;
        env.storage()
            .instance()
            .set(&DataKey::Metadata(name.clone()), &(description, author));
        env.events()
            .publish(
                (symbol_short!("mux_reg"), symbol_short!("regmeta")),
                (name, version),
            );
        Ok(())
    }

    /// Get the version string for a registered contract.
    pub fn get_version(env: Env, name: Symbol) -> Result<String, MuxRegistryError> {
        env.storage()
            .instance()
            .get(&DataKey::Version(name))
            .ok_or(MuxRegistryError::ContractNotFound)
    }

    /// Check whether a contract's version matches the expected string.
    /// Read-only: does not mutate storage.
    pub fn check_version(env: Env, name: Symbol, expected: String) -> bool {
        match Self::get_version(env, name) {
            Ok(v) => v == expected,
            Err(_) => false,
        }
    }

    /// Get the metadata (description, author) for a registered contract.
    pub fn get_metadata(env: Env, name: Symbol) -> Result<(String, String), MuxRegistryError> {
        env.storage()
            .instance()
            .get(&DataKey::Metadata(name))
            .ok_or(MuxRegistryError::ContractNotFound)
    }

    /// List all registered contract names.
    pub fn list_contracts(env: Env) -> Vec<Symbol> {
        env.storage()
            .instance()
            .get(&DataKey::Names)
            .unwrap_or_else(|| Vec::new(&env))
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    fn require_admin(env: &Env) -> Result<(), MuxRegistryError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(MuxRegistryError::NotInitialized)?;
        admin.require_auth();
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{symbol_short, testutils::Address as _, Env, String};

    fn setup() -> (Env, MuxRegistryClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxRegistry);
        let client = MuxRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    #[test]
    fn test_initialize_emits_init_and_rejects_double() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxRegistry);
        let client = MuxRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        assert!(client.try_initialize(&admin).is_ok());
        assert!(client.try_initialize(&admin).is_err());
        let events = env.events().all();
        assert!(events.iter().any(|e| e.topics.iter().any(|t| *t == symbol_short!("mux_reg").into())));
    }

    #[test]
    fn test_register_and_get() {
        let (env, client, _) = setup();
        let name = symbol_short!("account");
        let version = String::from_str(&env, "1.0.0");
        client.register(&name, &version);
        assert_eq!(client.get_version(&name), version);
        assert!(client.list_contracts().contains(&name));
    }

    #[test]
    fn test_non_admin_register_fails() {
        let (env, client, _admin) = setup();
        // Stop mocking auths so require_auth fails for any caller
        env.mock_auths(&[]);
        let name = symbol_short!("acc");
        let version = String::from_str(&env, "1.0.0");
        assert!(client.try_register(&name, &version).is_err());
    }

    #[test]
    fn test_get_unknown_fails() {
        let (_env, client, _) = setup();
        let result = client.try_get_version(&symbol_short!("ghost"));
        assert!(result.is_err());
    }

    #[test]
    fn test_check_version_no_mutation() {
        let (env, client, _) = setup();
        let name = symbol_short!("batcher");
        let version = String::from_str(&env, "2.1.0");
        client.register(&name, &version);

        // check_version with wrong version returns false
        let wrong = String::from_str(&env, "9.9.9");
        assert!(!client.check_version(&name, &wrong));

        // Storage is unchanged
        assert_eq!(client.get_version(&name), version);

        // check_version with correct version returns true
        assert!(client.check_version(&name, &version));
        assert_eq!(client.get_version(&name), version);
    }

    #[test]
    fn test_register_cap_128() {
        let (env, client, _) = setup();
        for i in 0..128u32 {
            let name = Symbol::new(&env, &format!("c{}", i));
            let version = String::from_str(&env, "1.0.0");
            assert!(client.try_register(&name, &version).is_ok());
        }
        // 129th should fail
        let name = Symbol::new(&env, "c129");
        let version = String::from_str(&env, "1.0.0");
        let result = client.try_register(&name, &version);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_with_metadata_and_get() {
        let (env, client, _) = setup();
        let name = symbol_short!("recover");
        let version = String::from_str(&env, "1.0.0");
        let desc = String::from_str(&env, "Recovery contract");
        let author = String::from_str(&env, "mux-labs");
        client.register_with_metadata(&name, &version, &desc, &author);
        let (d, a) = client.get_metadata(&name);
        assert_eq!(d, desc);
        assert_eq!(a, author);

        // Verify regmeta event emitted
        let events = env.events().all();
        assert!(events.iter().any(|e| {
            e.topics.iter().any(|t| *t == symbol_short!("regmeta").into())
        }));
    }
}
