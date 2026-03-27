# AURUM - RWA Gold Tokenization on Stellar/Soroban

**MVP de tokenización de activos reales (oro) para pagos fraccionados en la red Stellar.**

Un contrato inteligente en Soroban que permite pagar bienes y servicios con fracciones de oro digital, convirtiendo automáticamente montos en fiat (ARS) a la cantidad exacta de GOLD necesaria usando un **oráculo de precios en tiempo real** alimentado desde `gold-api.com`.

## 🏗️ Arquitectura

```
┌─────────────────────────────────────────────────────────────────────┐
│                        AURUM v2 - Arquitectura                      │
│                                                                     │
│  ┌──────────────┐    ┌───────────────┐    ┌──────────────────────┐ │
│  │ gold-api.com │    │ dolarapi.com  │    │                      │ │
│  │ (XAU/USD)    │    │ (USD/ARS)     │    │   Stellar Testnet    │ │
│  └──────┬───────┘    └──────┬────────┘    │                      │ │
│         │                   │              │  ┌────────────────┐  │ │
│         └───────┬───────────┘              │  │ AURUM Contract │  │ │
│                 ▼                          │  │  pay_with_rwa  │  │ │
│  ┌──────────────────────┐                 │  │  oracle_price  │  │ │
│  │  🔮 Oracle Feeder    │────────────────▸│  │  set_oracle    │  │ │
│  │  (oracle_feeder.sh)  │  set_oracle_    │  └────────┬───────┘  │ │
│  │  Calcula XAU → ARS   │  price()        │           │          │ │
│  └──────────────────────┘                 │  ┌────────▼───────┐  │ │
│                                            │  │ GOLD Token SAC │  │ │
│  ┌──────────┐                             │  │  transfer()    │  │ │
│  │ 📱 QR    │───▸ pay_with_rwa() ────────▸│  └────────────────┘  │ │
│  │ (monto)  │                             │                      │ │
│  └──────────┘                             └──────────────────────┘ │
│                                                                     │
│  Flujo:                                                             │
│  1. Oracle Feeder trae precio real del oro desde internet           │
│  2. Actualiza el contrato con el precio en ARS (7 decimales)       │
│  3. Usuario escanea QR (monto en ARS)                              │
│  4. Contrato calcula GOLD exacto necesario                         │
│  5. Transfiere GOLD del usuario al comerciante                     │
│  6. Emite evento on-chain con detalles del pago                    │
└─────────────────────────────────────────────────────────────────────┘
```

## 🛠️ Tech Stack

| Componente | Tecnología | Estado |
|-----------|------------|--------|
| Smart Contract | Rust (no-std) + soroban-sdk 25.3.0 | ✅ Implementado |
| Blockchain | Stellar / Soroban (Testnet) | ✅ Implementado |
| Token | GOLD (Stellar Asset Contract - SAC) | ✅ Implementado |
| Oráculo | gold-api.com + dolarapi.com → Oracle Feeder | ✅ Implementado |
| CLI | stellar-cli 25.2.0 | ✅ Usado |
| Wallet (Freighter) | @stellar/freighter-api | 📋 Diseñado |
| Ancla (SEP-24) | Deposit/Withdraw oro físico | 📋 Diseñado |

> Para detalles completos de integración con el Stellar Stack (SEP-10, SEP-24, SEP-31, CAP-46, Freighter), ver **[docs/STELLAR_STACK.md](docs/STELLAR_STACK.md)**

## 📁 Estructura del Proyecto

```
Hackaton-VendimiaTech-/
├── README.md
├── docs/
│   └── STELLAR_STACK.md          # Integración con Stellar Stack
├── contracts/
│   └── aurum/
│       ├── Cargo.toml             # Dependencias Rust/Soroban
│       └── src/
│           └── lib.rs             # Smart contract AURUM
├── scripts/
│   ├── setup_env.sh               # Instala Rust + stellar-cli
│   ├── create_assets.sh           # Crea cuentas y token GOLD
│   ├── build_and_deploy.sh        # Compila y deploya el contrato
│   ├── oracle_feeder.sh           # 🔮 Trae precio REAL del oro
│   ├── test_payment.sh            # Test de pago individual
│   └── demo_flow.sh               # Demo completo (para el pitch)
└── .keys/                         # (auto-generado) Claves y IDs
    ├── addresses.env
    ├── gold_contract_id.txt
    └── aurum_contract_id.txt
```

## 🚀 Quick Start

### 1. Setup del entorno
```bash
./scripts/setup_env.sh
```
Instala Rust, wasm32 target, stellar-cli y configura testnet.

### 2. Crear activos y cuentas
```bash
./scripts/create_assets.sh
```
Genera 5 cuentas (issuer, distributor, user1, user2, merchant), emite GOLD y lo distribuye.

### 3. Compilar y deployar contrato
```bash
./scripts/build_and_deploy.sh
```
Compila el WASM, optimiza, deploya a testnet e inicializa con oráculo.

### 4. 🔮 Actualizar oráculo con precio real
```bash
./scripts/oracle_feeder.sh           # Una sola actualización
./scripts/oracle_feeder.sh --dry-run # Ver precio sin actualizar on-chain
./scripts/oracle_feeder.sh --loop 60 # Actualizar cada 60 segundos
```
Trae el precio real del oro desde `gold-api.com`, convierte a ARS con `dolarapi.com`, y actualiza el contrato en testnet.

### 5. Testing
```bash
./scripts/test_payment.sh
```
Ejecuta un pago de prueba y muestra balances antes/después.

### 6. Demo (Pitch)
```bash
./scripts/demo_flow.sh
```
Flujo completo: actualiza oráculo con precio real → ejecuta 3 pagos → muestra balances.

## 📜 Funciones del Smart Contract

| Función | Descripción |
|---------|-------------|
| `initialize(admin, gold_token, oracle_price)` | Configura admin, token y precio |
| `set_oracle_price(admin, new_price, source)` | Actualiza tasa con fuente y timestamp |
| `get_oracle_price()` | Consulta tasa actual |
| `get_oracle_last_update()` | Timestamp de última actualización |
| `get_oracle_source()` | Fuente del precio (ej: "gold-api.com") |
| `get_payment_preview(amount_fiat)` | Vista previa: cuánto GOLD necesita |
| `pay_with_rwa(sender, dest, amount_fiat)` | **⭐ Ejecuta pago con conversión automática** |
| `get_admin()` | Consulta admin |
| `get_gold_token()` | Consulta dirección del token |

## 💡 Ejemplo de Pago

```
Precio REAL (gold-api.com): XAU/USD $4,433.39 por onza troy
Tipo de cambio (dolarapi.com): USD/ARS $1,425
Precio calculado: 1 gramo GOLD = $203,105.25 ARS

Compra: ☕ Café + medialunas = 1,500 ARS

Cálculo del contrato:
  gold_needed = 1,500 / 203,105.25 = 0.0073863 GOLD

Resultado:
  ✅ Se transfieren exactamente 73863 unidades (7 decimales)
  ✅ del usuario al comerciante
  ✅ Evento on-chain con precio, fuente, y timestamp
```

## 🔒 Seguridad

- ✅ `require_auth()` para autorización de pagos y admin
- ✅ `i128` con checked math (overflow/underflow protection)
- ✅ Prevención de re-inicialización
- ✅ Validaciones de montos positivos
- ✅ Eventos enriquecidos para auditoría (precio + fuente + timestamp)
- ✅ TTL management para optimización de storage
- ✅ Oracle feeder con validación de respuesta de API

## 🔗 Verificación On-Chain

Tras ejecutar el demo, verificá las transacciones en:
- **Stellar Expert Testnet**: https://stellar.expert/explorer/testnet

## 📝 Licencia

MIT - Hackathon VendimiaTech 2026