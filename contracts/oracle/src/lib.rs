#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Price(Symbol, Symbol), // base, quote
}

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// Initialize the oracle with an admin
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Set a price for a pair (base, quote). Only admin can do this.
    /// Price should be in 7 decimals. E.g. 1 XAU = $2500 -> 25000000000
    pub fn set_price(env: Env, base: Symbol, quote: Symbol, price: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if price <= 0 {
            panic!("Price must be positive");
        }

        env.storage()
            .instance()
            .set(&DataKey::Price(base, quote), &price);
    }

    /// Get the latest price for a pair. Panics if not found.
    pub fn get_price(env: Env, base: Symbol, quote: Symbol) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Price(base, quote))
            .unwrap_or_else(|| panic!("Price not found"))
    }
}
