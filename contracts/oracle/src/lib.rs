#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Base,
    Decimals,
    Resolution,
    Price(Asset, Asset), // (base_asset, quote_asset)
}

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// Initialize the mock oracle with admin, base asset, decimals, and resolution
    pub fn initialize(env: Env, admin: Address, base: Asset, decimals: u32, resolution: u32) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Base, &base);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        env.storage().instance().set(&DataKey::Resolution, &resolution);
    }

    /// set_price for mock admin
    pub fn set_price(env: Env, base_asset: Asset, quote_asset: Asset, price: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if price <= 0 {
            panic!("Price must be positive");
        }

        let timestamp = env.ledger().timestamp();
        let price_data = PriceData { price, timestamp };

        env.storage()
            .instance()
            .set(&DataKey::Price(base_asset, quote_asset), &price_data);
    }

    // ========================================================================
    // SEP-40 Standard Interface
    // ========================================================================

    pub fn base(env: Env) -> Asset {
        env.storage().instance().get(&DataKey::Base).unwrap()
    }

    pub fn assets(env: Env) -> Vec<Asset> {
        Vec::new(&env)
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Decimals).unwrap()
    }

    pub fn resolution(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Resolution).unwrap()
    }

    pub fn price(env: Env, asset: Asset, _timestamp: u64) -> Option<PriceData> {
        let base_asset = Self::base(env.clone());
        env.storage().instance().get(&DataKey::Price(base_asset, asset))
    }

    pub fn prices(env: Env, asset: Asset, _records: u32) -> Option<Vec<PriceData>> {
        let price_opt = Self::price(env.clone(), asset, 0);
        if let Some(p) = price_opt {
            let mut vec = Vec::new(&env);
            vec.push_back(p);
            Some(vec)
        } else {
            None
        }
    }

    pub fn cross_price(env: Env, base_asset: Asset, quote_asset: Asset, _timestamp: u64) -> Option<PriceData> {
        env.storage().instance().get(&DataKey::Price(base_asset, quote_asset))
    }

    pub fn cross_prices(env: Env, base_asset: Asset, quote_asset: Asset, _records: u32) -> Option<Vec<PriceData>> {
        let price_opt = Self::cross_price(env.clone(), base_asset, quote_asset, 0);
        if let Some(p) = price_opt {
            let mut vec = Vec::new(&env);
            vec.push_back(p);
            Some(vec)
        } else {
            None
        }
    }
}
