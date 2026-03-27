#!/bin/bash
# ============================================================================
# AURUM - Complete Demo Flow
# Unified script for the 3-minute hackathon pitch demo
# ============================================================================

set -e
source "$HOME/.cargo/env" 2>/dev/null || true

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
KEYS_DIR="$PROJECT_DIR/.keys"

# Load addresses
source "$KEYS_DIR/addresses.env"
GOLD_CONTRACT_ID=$(cat "$KEYS_DIR/gold_contract_id.txt")
AURUM_CONTRACT_ID=$(cat "$KEYS_DIR/aurum_contract_id.txt")

clear

echo -e "${BOLD}${GREEN}"
echo -e "╔══════════════════════════════════════════════════════════════╗"
echo -e "║                                                              ║"
echo -e "║          ⭐  AURUM - Gold-Backed Digital Payments  ⭐        ║"
echo -e "║              Real World Asset Tokenization on Stellar        ║"
echo -e "║                                                              ║"
echo -e "╚══════════════════════════════════════════════════════════════╝${NC}"
echo -e ""
echo -e "  ${CYAN}Smart Contract:${NC} $AURUM_CONTRACT_ID"
echo -e "  ${CYAN}GOLD Token:${NC}     $GOLD_CONTRACT_ID"
echo -e "  ${CYAN}Network:${NC}        Stellar Testnet"

sleep 2

# ============================================================================
# Scene 0: Real-Time Oracle Update
# ============================================================================

echo -e "\n${BOLD}${YELLOW}━━━ SCENE 0: 🔮 Real-Time Oracle Update ━━━${NC}"
echo -e "  Fetching live gold price from ${MAGENTA}gold-api.com${NC}..."
echo ""

# Run oracle feeder (one-shot, updates the contract with real price)
bash "$SCRIPT_DIR/oracle_feeder.sh" 2>/dev/null || echo -e "  ${YELLOW}⚠️  Oracle feeder skipped (run manually if needed)${NC}"

sleep 2

# ============================================================================
# Scene 1: Show Oracle Price (now REAL, not hardcoded)
# ============================================================================

echo -e "\n${BOLD}${YELLOW}━━━ SCENE 1: 📈 Oracle Price Check (LIVE DATA) ━━━${NC}"

ORACLE_PRICE=$(stellar contract invoke \
    --id "$AURUM_CONTRACT_ID" \
    --network testnet \
    --source-account user1 \
    -- \
    get_oracle_price 2>/dev/null | tr -d '"')

ORACLE_SOURCE=$(stellar contract invoke \
    --id "$AURUM_CONTRACT_ID" \
    --network testnet \
    --source-account user1 \
    -- \
    get_oracle_source 2>/dev/null | tr -d '"')

# Convert from 7 decimals
FORMATTED_PRICE=$(echo "scale=2; $ORACLE_PRICE / 10000000" | bc 2>/dev/null || echo "$ORACLE_PRICE")

echo -e "  📈 Current GOLD Price: ${MAGENTA}$ORACLE_PRICE${NC} (raw, 7 decimals)"
echo -e "  📈 Human readable:     ${MAGENTA}1 gram gold = $FORMATTED_PRICE ARS${NC}"
echo -e "  📡 Source:             ${CYAN}$ORACLE_SOURCE${NC}"

sleep 2

# ============================================================================
# Scene 2: User Balance BEFORE
# ============================================================================

echo -e "\n${BOLD}${YELLOW}━━━ SCENE 2: User Wallet - Before Payment ━━━${NC}"

USER_BALANCE=$(stellar contract invoke \
    --id "$GOLD_CONTRACT_ID" \
    --network testnet \
    --source-account user1 \
    -- \
    balance \
    --id "$USER1_ADDR" 2>/dev/null | tr -d '"')

MERCHANT_BALANCE=$(stellar contract invoke \
    --id "$GOLD_CONTRACT_ID" \
    --network testnet \
    --source-account merchant \
    -- \
    balance \
    --id "$MERCHANT_ADDR" 2>/dev/null | tr -d '"')

echo -e "  👤 User:     ${CYAN}$USER_BALANCE${NC} GOLD (raw)"
echo -e "  🏪 Merchant: ${CYAN}$MERCHANT_BALANCE${NC} GOLD (raw)"

sleep 2

# ============================================================================
# Scene 3: QR Code Scan Simulation
# ============================================================================

echo -e "\n${BOLD}${YELLOW}━━━ SCENE 3: 📱 QR Code Scanned! ━━━${NC}"

# Different payment amounts for variety
PAYMENTS=(
    "15000000000:1,500 ARS:☕ Café y medialunas"
    "50000000000:5,000 ARS:🛒 Compra en almacén"
    "32000000000:3,200 ARS:🚕 Viaje en taxi"
)

for PAYMENT in "${PAYMENTS[@]}"; do
    IFS=':' read -r AMOUNT DISPLAY DESC <<< "$PAYMENT"

    echo -e ""
    echo -e "  ${BOLD}$DESC${NC}"
    echo -e "  QR: {\"dest\": \"${MERCHANT_ADDR:0:10}...\", \"monto_fiat\": $DISPLAY}"

    # Preview
    GOLD_PREVIEW=$(stellar contract invoke \
        --id "$AURUM_CONTRACT_ID" \
        --network testnet \
        --source-account user1 \
        -- \
        get_payment_preview \
        --amount_fiat "$AMOUNT" 2>/dev/null | tr -d '"')

    echo -e "  💡 GOLD needed: ${MAGENTA}$GOLD_PREVIEW${NC} (~$(echo "scale=7; $GOLD_PREVIEW / 10000000" | bc) grams)"

    # Execute payment
    GOLD_USED=$(stellar contract invoke \
        --id "$AURUM_CONTRACT_ID" \
        --network testnet \
        --source-account user1 \
        -- \
        pay_with_rwa \
        --sender "$USER1_ADDR" \
        --destination "$MERCHANT_ADDR" \
        --amount_fiat "$AMOUNT" 2>/dev/null | tr -d '"')

    echo -e "  ✅ Paid! GOLD transferred: ${GREEN}$GOLD_USED${NC}"

    sleep 1
done

# ============================================================================
# Scene 4: Final Balances
# ============================================================================

echo -e "\n${BOLD}${YELLOW}━━━ SCENE 4: 📊 Final Balances ━━━${NC}"

USER_BALANCE_AFTER=$(stellar contract invoke \
    --id "$GOLD_CONTRACT_ID" \
    --network testnet \
    --source-account user1 \
    -- \
    balance \
    --id "$USER1_ADDR" 2>/dev/null | tr -d '"')

MERCHANT_BALANCE_AFTER=$(stellar contract invoke \
    --id "$GOLD_CONTRACT_ID" \
    --network testnet \
    --source-account merchant \
    -- \
    balance \
    --id "$MERCHANT_ADDR" 2>/dev/null | tr -d '"')

echo -e ""
echo -e "  ┌────────────────────────────────────────────────┐"
echo -e "  │          💰 GOLD Balance Comparison            │"
echo -e "  ├────────────┬───────────────┬───────────────────┤"
echo -e "  │ Account    │ Before        │ After             │"
echo -e "  ├────────────┼───────────────┼───────────────────┤"
echo -e "  │ 👤 User    │ $USER_BALANCE │ $USER_BALANCE_AFTER │"
echo -e "  │ 🏪 Merchant│ $MERCHANT_BALANCE │ $MERCHANT_BALANCE_AFTER │"
echo -e "  └────────────┴───────────────┴───────────────────┘"

# ============================================================================
# Scene 5: Verification Links
# ============================================================================

echo -e "\n${BOLD}${YELLOW}━━━ SCENE 5: 🔗 On-Chain Verification ━━━${NC}"
echo -e ""
echo -e "  ${CYAN}AURUM Contract:${NC}"
echo -e "  https://stellar.expert/explorer/testnet/contract/$AURUM_CONTRACT_ID"
echo -e ""
echo -e "  ${CYAN}GOLD Asset:${NC}"
echo -e "  https://stellar.expert/explorer/testnet/asset/GOLD-$ISSUER_ADDR"
echo -e ""
echo -e "  ${CYAN}User Account:${NC}"
echo -e "  https://stellar.expert/explorer/testnet/account/$USER1_ADDR"
echo -e ""

echo -e "${BOLD}${GREEN}"
echo -e "╔══════════════════════════════════════════════════════════════╗"
echo -e "║                                                              ║"
echo -e "║   ✅ Demo complete! Gold-backed micropayments on Stellar.    ║"
echo -e "║   Low-cost transactions via Soroban make this viable for     ║"
echo -e "║   everyday purchases - something impossible on congested     ║"
echo -e "║   networks.                                                  ║"
echo -e "║                                                              ║"
echo -e "╚══════════════════════════════════════════════════════════════╝${NC}"
