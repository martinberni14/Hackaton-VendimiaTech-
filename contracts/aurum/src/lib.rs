#![no_std]

use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, token, Address, Env, String,
};

// ============================================================================
// Events
// ============================================================================

#[contractevent]
#[derive(Clone, Debug)]
pub struct InitEvent {
    pub admin: Address,
    pub gold_token: Address,
    pub oracle_price: i128,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct OraclePriceUpdated {
    pub new_price: i128,
    pub source: String,
    pub timestamp: u64,
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
    /// Oracle price: how many fiat units per 1 whole GOLD token (7 decimals).
    /// E.g. 1 gram gold = 90,000 ARS → stored as 90_000_0000000 (with 7 decimals)
    OraclePrice,
    /// Ledger sequence number of the last oracle price update.
    OracleLastUpdate,
    /// Descriptive source string of the price feed (e.g. "gold-api.com").
    OracleSource,
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
    /// - `admin`: The administrator address (can update oracle price).
    /// - `gold_token`: The contract address of the GOLD token (SAC).
    /// - `oracle_price_fiat`: Price of 1 GOLD in fiat (with 7 decimals).
    ///   Example: 1 gram gold = 90,000 ARS → pass 90_000_0000000
    pub fn initialize(
        env: Env,
        admin: Address,
        gold_token: Address,
        oracle_price_fiat: i128,
    ) {
        // Prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }

        assert!(oracle_price_fiat > 0, "oracle price must be positive");

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::GoldToken, &gold_token);
        env.storage().instance().set(&DataKey::OraclePrice, &oracle_price_fiat);
        
        let ledger_timestamp = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::OracleLastUpdate, &ledger_timestamp);
        
        // Initial price comes from the initialization script
        let source = String::from_str(&env, "genesis/init");
        env.storage().instance().set(&DataKey::OracleSource, &source);

        InitEvent {
            admin,
            gold_token,
            oracle_price: oracle_price_fiat,
        }
        .publish(&env);
    }

    // ========================================================================
    // Oracle Management (simulated)
    // ========================================================================

    /// Update the oracle price. Only admin can call this.
    ///
    /// - `new_price`: New price of 1 GOLD in fiat (with 7 decimals).
    /// - `source`: Descriptive origin of the price (e.g. "gold-api.com").
    pub fn set_oracle_price(env: Env, admin: Address, new_price: i128, source: String) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        assert!(admin == stored_admin, "unauthorized: not admin");
        assert!(new_price > 0, "price must be positive");

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let ledger_timestamp = env.ledger().timestamp();

        env.storage().instance().set(&DataKey::OraclePrice, &new_price);
        env.storage().instance().set(&DataKey::OracleLastUpdate, &ledger_timestamp);
        env.storage().instance().set(&DataKey::OracleSource, &source);

        OraclePriceUpdated {
            new_price,
            source,
            timestamp: ledger_timestamp,
        }
        .publish(&env);
    }

    /// Get the current oracle price (1 GOLD in fiat, 7 decimals).
    pub fn get_oracle_price(env: Env) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        env.storage().instance().get(&DataKey::OraclePrice).unwrap()
    }

    /// Get full oracle information: (price, last_update_timestamp, source).
    ///
    /// Returns a Vec with 3 elements for simplicity:
    /// [0] = price (i128), [1] = last_update ledger timestamp (i128), [2] = source as i128 (0)
    /// For richer data, use individual getters.
    pub fn get_oracle_last_update(env: Env) -> u64 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        env.storage()
            .instance()
            .get(&DataKey::OracleLastUpdate)
            .unwrap_or(0)
    }

    /// Get the oracle data source description.
    pub fn get_oracle_source(env: Env) -> String {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        env.storage()
            .instance()
            .get(&DataKey::OracleSource)
            .unwrap_or(String::from_str(&env, "not set"))
    }

    // ========================================================================
    // Payment Logic (the star function)
    // ========================================================================

    /// Calculate how much GOLD is needed to pay a given fiat amount.
    ///
    /// Returns the amount of GOLD (with 7 decimals) required.
    /// Example: fiat=5000_0000000 (5000 ARS), price=90000_0000000 (90k ARS/GOLD)
    ///   → gold_needed = 5000/90000 * 10^7 = 555555 (~0.0555555 GOLD)
    pub fn get_payment_preview(env: Env, amount_fiat: i128) -> i128 {
        assert!(amount_fiat > 0, "fiat amount must be positive");

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let oracle_price: i128 = env.storage().instance().get(&DataKey::OraclePrice).unwrap();

        // gold_needed = (amount_fiat * TOKEN_DECIMALS) / oracle_price
        // Both amount_fiat and oracle_price are in 7-decimal format,
        // so we multiply by TOKEN_DECIMALS to keep precision.
        let gold_needed = amount_fiat
            .checked_mul(TOKEN_DECIMALS)
            .expect("overflow in gold calculation")
            .checked_div(oracle_price)
            .expect("division by zero");

        assert!(gold_needed > 0, "gold amount rounds to zero");
        gold_needed
    }

    /// Pay with RWA (GOLD). This is the star function.
    ///
    /// The sender pays `amount_fiat` worth of goods/services by transferring
    /// the equivalent amount of GOLD tokens to the destination.
    ///
    /// Flow:
    /// 1. Sender authorizes the transaction
    /// 2. Contract reads oracle price
    /// 3. Calculates exact GOLD needed for the fiat amount
    /// 4. Transfers GOLD from sender to destination via token contract
    /// 5. Emits event with full payment details
    /// 6. Returns the amount of GOLD transferred
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

        // 2. Read oracle price
        let oracle_price: i128 = env.storage().instance().get(&DataKey::OraclePrice).unwrap();

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
        // The sender has already authorized this contract call,
        // which includes the sub-invocation of the token transfer.
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

    /// Get the admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    /// Get the GOLD token contract address.
    pub fn get_gold_token(env: Env) -> Address {
        env.storage().instance().get(&DataKey::GoldToken).unwrap()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{token::StellarAssetClient, Env};

    fn setup_test() -> (Env, Address, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let merchant = Address::generate(&env);

        // Create a test token (simulates the GOLD SAC)
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let gold_token_address = token_contract.address();

        // Mint GOLD to user (100 GOLD = 100_0000000 with 7 decimals)
        let sac_client = StellarAssetClient::new(&env, &gold_token_address);
        sac_client.mint(&user, &1_000_0000000); // 1000 GOLD

        (env, admin, user, merchant, gold_token_address, token_admin)
    }

    #[test]
    fn test_initialize() {
        let (env, admin, _user, _merchant, gold_token, _token_admin) = setup_test();
        let contract_id = env.register(AurumContract, ());
        let client = AurumContractClient::new(&env, &contract_id);

        // 1 GOLD = 90,000 ARS (with 7 decimals)
        let oracle_price: i128 = 90_000_0000000;
        client.initialize(&admin, &gold_token, &oracle_price);

        assert_eq!(client.get_admin(), admin);
        assert_eq!(client.get_gold_token(), gold_token);
        assert_eq!(client.get_oracle_price(), oracle_price);
    }

    #[test]
    fn test_payment_preview() {
        let (env, admin, _user, _merchant, gold_token, _token_admin) = setup_test();
        let contract_id = env.register(AurumContract, ());
        let client = AurumContractClient::new(&env, &contract_id);

        let oracle_price: i128 = 90_000_0000000; // 1 GOLD = 90k ARS
        client.initialize(&admin, &gold_token, &oracle_price);

        // Preview: how much GOLD for 5,000 ARS?
        let fiat_amount: i128 = 5_000_0000000; // 5000 ARS
        let gold_needed = client.get_payment_preview(&fiat_amount);

        // Expected: 5000/90000 * 10^7 = 555555 (0.0555555 GOLD)
        let expected_gold: i128 = 555555;
        assert_eq!(gold_needed, expected_gold);
    }

    #[test]
    fn test_pay_with_rwa() {
        let (env, admin, user, merchant, gold_token, _token_admin) = setup_test();
        let contract_id = env.register(AurumContract, ());
        let client = AurumContractClient::new(&env, &contract_id);

        // Initialize with 1 GOLD = 90,000 ARS
        let oracle_price: i128 = 90_000_0000000;
        client.initialize(&admin, &gold_token, &oracle_price);

        // Check balances before
        let gold_client = token::TokenClient::new(&env, &gold_token);
        let user_balance_before = gold_client.balance(&user);
        let merchant_balance_before = gold_client.balance(&merchant);

        assert_eq!(user_balance_before, 1_000_0000000); // 1000 GOLD
        assert_eq!(merchant_balance_before, 0);

        // Pay 5,000 ARS with GOLD
        let fiat_amount: i128 = 5_000_0000000;
        let gold_used = client.pay_with_rwa(&user, &merchant, &fiat_amount);

        let expected_gold: i128 = 555555; // ~0.0555555 GOLD
        assert_eq!(gold_used, expected_gold);

        // Check balances after
        let user_balance_after = gold_client.balance(&user);
        let merchant_balance_after = gold_client.balance(&merchant);

        assert_eq!(user_balance_after, user_balance_before - gold_used);
        assert_eq!(merchant_balance_after, gold_used);
    }

    #[test]
    fn test_update_oracle_price() {
        let (env, admin, _user, _merchant, gold_token, _token_admin) = setup_test();
        let contract_id = env.register(AurumContract, ());
        let client = AurumContractClient::new(&env, &contract_id);

        let oracle_price: i128 = 90_000_0000000;
        client.initialize(&admin, &gold_token, &oracle_price);

        // Update price to 95,000 ARS
        let new_price: i128 = 95_000_0000000;
        let source = String::from_str(&env, "gold-api.com");
        client.set_oracle_price(&admin, &new_price, &source);
        assert_eq!(client.get_oracle_price(), new_price);
        assert_eq!(client.get_oracle_source(), source);
    }

    #[test]
    fn test_multiple_payments() {
        let (env, admin, user, merchant, gold_token, _token_admin) = setup_test();
        let contract_id = env.register(AurumContract, ());
        let client = AurumContractClient::new(&env, &contract_id);

        let oracle_price: i128 = 90_000_0000000;
        client.initialize(&admin, &gold_token, &oracle_price);

        let gold_client = token::TokenClient::new(&env, &gold_token);

        // Payment 1: 1,500 ARS (a coffee and sandwich)
        let gold_1 = client.pay_with_rwa(&user, &merchant, &1_500_0000000);

        // Payment 2: 25,000 ARS (grocery shopping)
        let gold_2 = client.pay_with_rwa(&user, &merchant, &25_000_0000000);

        // Payment 3: 3,200 ARS (taxi)
        let gold_3 = client.pay_with_rwa(&user, &merchant, &3_200_0000000);

        let total_gold = gold_1 + gold_2 + gold_3;
        let user_balance = gold_client.balance(&user);
        let merchant_balance = gold_client.balance(&merchant);

        assert_eq!(user_balance, 1_000_0000000 - total_gold);
        assert_eq!(merchant_balance, total_gold);
    }
}
