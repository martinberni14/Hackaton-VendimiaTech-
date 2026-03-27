# AURUM — Integración con Stellar Stack

Documentación técnica de cómo AURUM integra y/o utilizaría cada componente del ecosistema Stellar.

## Resumen de Integración

| Componente | Estado | Descripción |
|-----------|--------|-------------|
| **Soroban (CAP-46)** | ✅ Implementado | Smart contract en Rust, compilado a WASM, desplegado en Testnet |
| **SAC (CAP-46-06)** | ✅ Implementado | Token GOLD como Stellar Asset Contract para Soroban |
| **SEP-40 (Oracle)** | ✅ Implementado | Oracle feeder off-chain con precio real de oro (gold-api.com) |
| **Stellar SDK/CLI** | ✅ Usado | `stellar-cli` + `soroban-sdk` para build/deploy/invoke |
| **SEP-10 (Auth)** | 📋 Diseñado | Autenticación de wallet via challenge-response |
| **SEP-24 (Deposit/Withdraw)** | 📋 Diseñado | On/off-ramp con ancla para oro físico |
| **SEP-31 (Cross-border)** | 📋 Diseñado | Pagos transfronterizos con GOLD tokenizado |
| **Freighter Wallet** | 📋 Diseñado | Firma de transacciones desde navegador web |

---

## ✅ Componentes Implementados

### 1. Soroban Smart Contracts (CAP-46)

**¿Qué es?** CAP-46 introdujo la capa de contratos inteligentes Turing-completos sobre Stellar, llamada Soroban.

**Cómo lo usamos:** El contrato `AurumContract` está escrito en Rust (`no_std`), compilado a WebAssembly (WASM), y desplegado en Stellar Testnet. Implementa:
- Conversión fiat→oro en tiempo real
- Transferencias atómicas de tokens RWA
- Gestión de oráculo con eventos on-chain
- TTL management para optimización de costos

```rust
// Ejemplo: función estrella del contrato
pub fn pay_with_rwa(env: Env, sender: Address, destination: Address, amount_fiat: i128) -> i128 {
    sender.require_auth();
    let oracle_price = env.storage().instance().get(&DataKey::OraclePrice).unwrap();
    let gold_needed = amount_fiat.checked_mul(TOKEN_DECIMALS).unwrap()
        .checked_div(oracle_price).unwrap();
    token_client.transfer(&sender, &destination, &gold_needed);
    gold_needed
}
```

### 2. Stellar Asset Contract — SAC (CAP-46-06)

**¿Qué es?** CAP-46-06 define cómo los activos clásicos de Stellar (emitidos via operaciones `Payment`) se envuelven automáticamente como contratos Soroban, permitiendo que los smart contracts interactúen con ellos.

**Cómo lo usamos:**
1. Emitimos el token `GOLD` como activo clásico de Stellar (`GOLD:ISSUER_ADDRESS`)
2. Lo "wrapeamos" como SAC con `stellar contract asset deploy`
3. El contrato AURUM interactúa con el GOLD via `token::TokenClient`, llamando `transfer()`, `balance()`, etc.

```bash
# Wrapping GOLD como SAC
stellar contract asset deploy \
    --asset "GOLD:$ISSUER_ADDR" \
    --source-account issuer \
    --network testnet
```

### 3. Oracle Pattern (SEP-40)

**¿Qué es?** SEP-40 define el estándar para oráculos en Soroban. Establece cómo un contrato puede proveer datos externos (precios, tasas) de forma estandarizada.

**Cómo lo usamos:** Implementamos el patrón de oracle feeder — el mismo que usa Chainlink, Band Protocol, y Lightecho Oracle en producción:

```
┌──────────────────┐     ┌─────────────┐     ┌──────────────────┐
│  gold-api.com    │────▸│ Oracle      │────▸│ AURUM Contract   │
│  (precio XAU/USD)│     │ Feeder.sh   │     │ set_oracle_price │
├──────────────────┤     │             │     │                  │
│  dolarapi.com    │────▸│ Calcula     │     │ Guarda precio +  │
│  (USD/ARS)       │     │ XAU → ARS   │     │ timestamp +      │
└──────────────────┘     └─────────────┘     │ source on-chain  │
                                              └──────────────────┘
```

**Oráculo Lightecho (futuro):** Para producción, AURUM podría integrar directamente el contrato Lightecho Oracle desplegado en mainnet (`CDOR3QD27WAAF4TK4MO33TGQXR6RPNANNVLOY277W2XVV6ZVJ6X6X42T`) via cross-contract calls, eliminando la necesidad de un feeder propio.

### 4. Stellar CLI & SDK

**¿Qué es?** Las herramientas oficiales para interactuar con la red Stellar y Soroban.

**Cómo lo usamos:**
- `stellar-cli`: Build, deploy, invoke, y gestión de cuentas
- `soroban-sdk 25.3.0`: SDK de Rust para desarrollo del contrato
- Scripts bash que orquestan el flujo completo (creación de activos, deploy, test, demo)

---

## 📋 Componentes Diseñados (Roadmap de Producción)

### 5. SEP-10: Stellar Web Authentication

**¿Qué es?** Un estándar para autenticar usuarios verificando que poseen una cuenta Stellar específica, sin revelar su clave secreta.

**Cómo lo usaríamos en AURUM:**

```
┌──────────┐    1. Request challenge     ┌──────────────┐
│  Wallet  │ ──────────────────────────▸ │  AURUM       │
│  (User)  │                              │  Backend     │
│          │ ◂────────────────────────── │              │
│          │    2. Challenge TX           │              │
│          │                              │              │
│          │    3. Signed challenge       │              │
│          │ ──────────────────────────▸ │              │
│          │                              │              │
│          │ ◂────────────────────────── │              │
│          │    4. JWT Token              │              │
└──────────┘                              └──────────────┘
```

El backend AURUM generaría un "challenge" (una transacción especial de Stellar). El usuario la firma con su wallet (Freighter) demostrando que posee la cuenta. El backend verifica la firma y le da un JWT para sesiones autenticadas. Esto permite:
- Verificar identidad sin custodiar claves privadas
- Proteger endpoints como `set_oracle_price` (solo admin)
- Habilitar un dashboard personalizado por usuario

### 6. SEP-24: Hosted Deposit and Withdrawal

**¿Qué es?** Define cómo un "ancla" (anchor) — una entidad que mantiene activos del mundo real — facilita el depósito y retiro de esos activos en/desde la blockchain.

**Cómo lo usaríamos en AURUM:**

Un **ancla de oro** sería la entidad que mantiene oro físico en bóvedas certificadas. El flujo sería:

| Acción | Flujo |
|--------|-------|
| **Depósito (Compra de GOLD)** | Usuario deposita ARS via transferencia bancaria → Ancla recibe ARS → Compra oro físico → Emite tokens GOLD al usuario |
| **Retiro (Venta de GOLD)** | Usuario envía tokens GOLD al ancla → Ancla vende oro físico → Transfiere ARS a cuenta bancaria del usuario |

```
┌──────────────┐     SEP-24      ┌──────────────────┐     Custodia     ┌──────────┐
│  App AURUM   │ ──────────────▸ │  Ancla de Oro    │ ───────────────▸ │ Bóveda   │
│  (wallet)    │  Deposit ARS   │  (Ej: Agrotoken, │  Compra/Vende   │ de Oro   │
│              │ ◂────────────── │   Ripio, etc.)   │  oro físico     │ Físico   │
│              │  Recibe GOLD   │                  │                 │          │
└──────────────┘                 └──────────────────┘                 └──────────┘
```

> **Nota para el jurado:** En Argentina, empresas como Agrotoken ya tokenizan commodities agrícolas en Stellar. AURUM aplicaría el mismo modelo pero para oro, con la ventaja de que el oro tiene un mercado global más líquido.

### 7. SEP-31: Cross-Border Payments

**¿Qué es?** Define pagos transfronterizos entre anclas, permitiendo enviar valor entre diferentes jurisdicciones.

**Cómo lo usaríamos:**
Un usuario en Argentina podría pagar a un comercio en Brasil usando GOLD como moneda puente:
1. Usuario envía GOLD → Ancla Argentina
2. Ancla Argentina envía instrucciones SEP-31 → Ancla Brasil
3. Ancla Brasil entrega BRL al comercio brasileño

Esto elimina intermediarios bancarios y reduce comisiones de ~5% (SWIFT) a ~0.01% (Stellar).

### 8. Freighter Wallet Integration

**¿Qué es?** Freighter es la wallet oficial de Stellar que funciona como extensión de navegador (similar a MetaMask para Ethereum).

**Cómo lo integraríamos:**

```javascript
// Ejemplo de integración con Freighter
import freighterApi from "@stellar/freighter-api";

// 1. Verificar que Freighter está instalado
const isConnected = await freighterApi.isConnected();

// 2. Obtener la dirección del usuario
const publicKey = await freighterApi.getPublicKey();

// 3. Firmar la transacción de pago
const signedTx = await freighterApi.signTransaction(
    paymentTxXDR,
    { networkPassphrase: "Test SDF Network ; September 2015" }
);

// 4. Enviar la transacción firmada a la red
const result = await server.submitTransaction(signedTx);
```

En la versión con frontend web, el usuario abriría la app AURUM, escanearía un QR del comercio, vería cuánto GOLD necesita, y Freighter le pediría confirmación para firmar la transacción — exactamente como funciona MetaMask en Ethereum, pero con fees 1000x más baratos.

---

## Ventajas del Stack Stellar para RWA

| Característica | Stellar | Ethereum | Solana |
|---------------|---------|----------|--------|
| Costo por tx | ~$0.00001 | ~$2-50 | ~$0.025 |
| Finalidad | 5 segundos | 12+ segundos | 0.4 segundos |
| Compliance built-in | ✅ Clawback, Trustlines | ❌ Requiere ERC-3643 | ❌ |
| Soporte RWA nativo | ✅ Anclas, SEP-24 | ❌ Custom bridges | ❌ |
| Smart contracts | ✅ Soroban (WASM) | ✅ EVM (Solidity) | ✅ BPF (Rust) |

---

## Referencias

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [SEP-10 Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md)
- [SEP-24 Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0024.md)
- [SEP-40 Oracle Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0040.md)
- [CAP-46-06 Token Contract](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-06.md)
- [Lightecho Oracle (Testnet)](https://stellar.expert/explorer/testnet/contract/CA335SIV2XT6OC3SOUTZBHTX5IXMFO3WYBD3NNVBP37JXX4FXFNF5CI6)
- [Freighter Wallet](https://www.freighter.app/)
- [Agrotoken — RWA en Stellar](https://www.agrotoken.com/)
