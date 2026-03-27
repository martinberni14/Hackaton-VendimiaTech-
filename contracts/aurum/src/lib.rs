#![no_std]

use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, token, vec, Address, Env, Symbol, IntoVal, Vec
};

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

// ============================================================================
// Events
// ============================================================================

#[contractevent]
#[derive(Clone, Debug)]
pub struct InitEvent {
    pub admin: Address,
    pub gold_token: Address,
    pub oracle_address: Address,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PaymentExecuted {
    pub sender: Address,
    pub destination: Address,
    pub amount_fiat: i128,
    pub gold_used: i128,
}

// ============================================================================
// Storage Keys
// ============================================================================

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    GoldToken,
    OracleAddress,
}

// ============================================================================
// Constants
// ============================================================================

/// Stellar tokens use 7 decimals by default (SAC standard).
const TOKEN_DECIMALS: i128 = 10_000_000; // 10^7

// TTL management constants
const INSTANCE_LIFETIME_THRESHOLD: u32 = 17280; // ~1 day
const INSTANCE_BUMP_AMOUNT: u32 = 518400; // ~30 days

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct AurumContract;

#[contractimpl]
impl AurumContract {
    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initialize the AURUM payment contract.
    ///
    /// - `admin`: The administrator address.
    /// - `gold_token`: The contract address of the GOLD token (SAC).
    /// - `oracle_address`: Address of the mock On-Chain Oracle.
    pub fn initialize(
        env: Env,
        admin: Address,
        gold_token: Address,
        oracle_address: Address,
    ) {
        // Prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::GoldToken, &gold_token);
        env.storage().instance().set(&DataKey::OracleAddress, &oracle_address);
        
        InitEvent {
            admin,
            gold_token,
            oracle_address,
        }
        .publish(&env);
    }

    // ========================================================================
    // Oracle Reader Logic
    // ========================================================================

    /// Reads the XAU -> ARS conversion strictly from the On-Chain Oracle
    fn get_cross_oracle_price(env: &Env) -> i128 {
        let oracle_addr: Address = env.storage().instance().get(&DataKey::OracleAddress).unwrap();

        let xau_asset = Asset::Other(Symbol::new(env, "XAU"));
        let usd_asset = Asset::Other(Symbol::new(env, "USD"));
        let ars_asset = Asset::Other(Symbol::new(env, "ARS"));
        
        // 1. Fetch XAU -> USD price from Oracle
        let args_xau_usd = vec![env, xau_asset.into_val(env), usd_asset.clone().into_val(env), 0u64.into_val(env)];
        let xau_usd_opt: Option<PriceData> = env.invoke_contract(&oracle_addr, &Symbol::new(env, "cross_price"), args_xau_usd);
        let xau_usd = xau_usd_opt.expect("XAU/USD price not found").price;

        // 2. Fetch USD -> ARS price from Oracle
        let args_usd_ars = vec![env, usd_asset.into_val(env), ars_asset.into_val(env), 0u64.into_val(env)];
        let usd_ars_opt: Option<PriceData> = env.invoke_contract(&oracle_addr, &Symbol::new(env, "cross_price"), args_usd_ars);
        let usd_ars = usd_ars_opt.expect("USD/ARS price not found").price;

        // 3. Compute XAU -> ARS cross rate.
        // Both returned values have 7 decimals. So 1 XAU = (xau_usd * usd_ars) / 10^7
        let xau_ars = xau_usd
            .checked_mul(usd_ars)
            .expect("overflow reading fiat price")
            .checked_div(TOKEN_DECIMALS)
            .expect("precision error");

        assert!(xau_ars > 0, "cross price must be positive");
        xau_ars
    }

    // ========================================================================
    // Payment Logic (the star function)
    // ========================================================================

    /// Calculate how much GOLD is needed to pay a given fiat amount.
    ///
    /// Returns the amount of GOLD (with 7 decimals) required.
    pub fn get_payment_preview(env: Env, amount_fiat: i128) -> i128 {
        assert!(amount_fiat > 0, "fiat amount must be positive");

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let oracle_price = Self::get_cross_oracle_price(&env);

        let gold_needed = amount_fiat
            .checked_mul(TOKEN_DECIMALS)
            .expect("overflow in gold calculation")
            .checked_div(oracle_price)
            .expect("division by zero");

        assert!(gold_needed > 0, "gold amount rounds to zero");
        gold_needed
    }

    /// Pay with RWA (GOLD). This is the star function.
    pub fn pay_with_rwa(
        env: Env,
        sender: Address,
        destination: Address,
        amount_fiat: i128,
    ) -> i128 {
        // 1. Authorize sender
        sender.require_auth();

        assert!(amount_fiat > 0, "fiat amount must be positive");

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // 2. Read oracle prices and calculate cross rate
        let oracle_price = Self::get_cross_oracle_price(&env);

        // 3. Calculate GOLD needed
        let gold_needed = amount_fiat
            .checked_mul(TOKEN_DECIMALS)
            .expect("overflow in gold calculation")
            .checked_div(oracle_price)
            .expect("division by zero");

        assert!(gold_needed > 0, "gold amount rounds to zero");

        // 4. Get the GOLD token contract and execute transfer
        let gold_token: Address = env.storage().instance().get(&DataKey::GoldToken).unwrap();
        let token_client = token::TokenClient::new(&env, &gold_token);

        // Transfer GOLD from sender to destination
        token_client.transfer(&sender, &destination, &gold_needed);

        // 5. Emit payment event
        PaymentExecuted {
            sender,
            destination,
            amount_fiat,
            gold_used: gold_needed,
        }
        .publish(&env);

        // 6. Return gold used
        gold_needed
    }

    // ========================================================================
    // View functions
    // ========================================================================

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    pub fn get_gold_token(env: Env) -> Address {
        env.storage().instance().get(&DataKey::GoldToken).unwrap()
    }

    pub fn get_oracle_address(env: Env) -> Address {
        env.storage().instance().get(&DataKey::OracleAddress).unwrap()
    }
}
