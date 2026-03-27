#!/bin/bash
# ============================================================================
# AURUM & ORACLE - Build, Optimize, Deploy, and Initialize Contracts
# ============================================================================

set -e
source "$HOME/.cargo/env" 2>/dev/null || true

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CONTRACT_DIR="$PROJECT_DIR/contracts"
KEYS_DIR="$PROJECT_DIR/.keys"

echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  AURUM & ORACLE - Build, Deploy & Initialize${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"

# ============================================================================
# 1. Build the contracts
# ============================================================================

echo -e "\n${YELLOW}[1/5] Building workspace contracts...${NC}"
cd "$CONTRACT_DIR"
cargo build --target wasm32v1-none --release
echo -e "  ✅ Contracts compiled"

# ============================================================================
# 2. Optimize WASM binaries
# ============================================================================
echo -e "\n${YELLOW}[2/5] Optimizing WASM binaries...${NC}"

# Oracle
ORACLE_WASM="$CONTRACT_DIR/target/wasm32v1-none/release/oracle.wasm"
if [ ! -f "$ORACLE_WASM" ]; then
    echo -e "  ${RED}❌ Oracle WASM not found${NC}"
    exit 1
fi
stellar contract optimize --wasm "$ORACLE_WASM"
ORACLE_OPT_WASM="${ORACLE_WASM%.wasm}.optimized.wasm"
echo -e "  ✅ Oracle WASM optimized"

# Aurum
AURUM_WASM="$CONTRACT_DIR/target/wasm32v1-none/release/aurum.wasm"
if [ ! -f "$AURUM_WASM" ]; then
    echo -e "  ${RED}❌ Aurum WASM not found${NC}"
    exit 1
fi
stellar contract optimize --wasm "$AURUM_WASM"
AURUM_OPT_WASM="${AURUM_WASM%.wasm}.optimized.wasm"
echo -e "  ✅ Aurum WASM optimized"

# ============================================================================
# 3. Deploy Oracle to Testnet
# ============================================================================
echo -e "\n${YELLOW}[3/5] Deploying Oracle Contract...${NC}"

ORACLE_CONTRACT_ID=$(stellar contract deploy \
    --wasm "$ORACLE_OPT_WASM" \
    --source-account issuer \
    --network testnet 2>/dev/null)

echo "$ORACLE_CONTRACT_ID" > "$KEYS_DIR/oracle_contract_id.txt"
echo -e "  📋 Oracle Contract ID: ${CYAN}$ORACLE_CONTRACT_ID${NC}"

 stellar contract optimize --wasm "$AURUM_WASM"

ORACLE_OPT="${ORACLE_WASM%.wasm}.optimized.wasm"
AURUM_OPT="${AURUM_WASM%.wasm}.optimized.wasm"

echo -e "  ✅ WASMs optimized"

# ============================================================================
# 3. Deploy to Testnet
# ============================================================================

echo -e "\n${YELLOW}[3/5] Deploying to Testnet...${NC}"

ORACLE_ID=$(stellar contract deploy --wasm "$ORACLE_OPT" --source-account issuer --network testnet)
echo "$ORACLE_ID" > "$KEYS_DIR/oracle_contract_id.txt"
echo -e "  📋 Oracle Contract ID: ${CYAN}$ORACLE_ID${NC}"

AURUM_ID=$(stellar contract deploy --wasm "$AURUM_OPT" --source-account issuer --network testnet)
echo "$AURUM_ID" > "$KEYS_DIR/aurum_contract_id.txt"
echo -e "  📋 Aurum Contract ID: ${CYAN}$AURUM_ID${NC}"

# ============================================================================
# 4. Initialize Oracle and Set Prices
# ============================================================================

echo -e "\n${YELLOW}[4/5] Initializing SEP-40 Oracle...${NC}"
source "$KEYS_DIR/addresses.env"

# Init Oracle: Admin, Base="USD" (Other variant), Decimals=7, Resolution=300
stellar contract invoke --id "$ORACLE_ID" --source-account issuer --network testnet \
    -- initialize \
    --admin "$ISSUER_ADDR" \
    --base '{"Other":["USD"]}' \
    --decimals 7 \
    --resolution 300

# Set Price: XAU -> USD = 2000 USD (2000_0000000)
stellar contract invoke --id "$ORACLE_ID" --source-account issuer --network testnet \
    -- set_price \
    --base_asset '{"Other":["XAU"]}' \
    --quote_asset '{"Other":["USD"]}' \
    --price 20000000000

# Set Price: USD -> ARS = 1000 ARS (1000_0000000)
stellar contract invoke --id "$ORACLE_ID" --source-account issuer --network testnet \
    -- set_price \
    --base_asset '{"Other":["USD"]}' \
    --quote_asset '{"Other":["ARS"]}' \
    --price 10000000000

echo -e "  ✅ Oracle initialized with XAU=\$2000 and USD=\$1000 ARS"

# ============================================================================
# 5. Initialize AURUM
# ============================================================================

echo -e "\n${YELLOW}[5/5] Initializing AURUM contract...${NC}"
GOLD_CONTRACT_ID=$(cat "$KEYS_DIR/gold_contract_id.txt")

stellar contract invoke --id "$AURUM_ID" --source-account issuer --network testnet \
    -- initialize \
    --admin "$ISSUER_ADDR" \
    --gold_token "$GOLD_CONTRACT_ID" \
    --oracle_address "$ORACLE_ID"

echo -e "  ✅ Aurum initialized with Oracle integration!"

# ============================================================================
# Summary
# ============================================================================

echo -e "\n${GREEN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  ✅ Deployment Complete!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo -e ""
echo -e "  ${CYAN}Oracle Contract:${NC} $ORACLE_ID"
echo -e "  ${CYAN}AURUM Contract:${NC}  $AURUM_ID"
echo -e "  ${CYAN}GOLD Token SAC:${NC}  $GOLD_CONTRACT_ID"
echo -e ""
