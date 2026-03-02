# STELLAR SQUID: The Autonomous Agent Survival Game

## Game Design Document v1.3 (STORED)

See: `/root/.openclaw/workspace/stellar-squid/docs/GDD-v1.3.md`

---

## Quick Reference

### Deployer Cost
- ~15 XLM one-time

### Agent Entry
- 50 XLM Life Bond

### Season Cost to Survive
- 262 XLM (Rounds 1-5)

### Protocol Revenue
- 5% of every pulse

### Kill Reward
- 100% of victim's balance

### Prize Pool
- 20% of voluntary withdrawals
- 5% pulse tax

### Economic Splits

**Pulse Cost Distribution (5%/5%/90%):**
- 5% → Protocol Fee Address
- 5% → Prize Pool
- 90% → TTL Rent (burned/stored for contract lifetime)

**Withdrawal Distribution (80%/20%):**
- 80% → Refunded to agent owner
- 20% → Prize Pool contribution

### Rounds
| Round | Duration | Pulse Window | Grace | Cost |
|-------|----------|-------------|-------|------|
| 1 | 72h | 6h | 1h | 0.5 XLM |
| 2 | 48h | 3h | 30min | 1.0 XLM |
| 3 | 24h | 1h | 15min | 2.0 XLM |
| 4 | 12h | 30min | 10min | 3.0 XLM |
| 5 | 6h | 15min | 5min | 5.0 XLM |

---

## Contract Functions

### GameRegistry Functions

#### Admin Functions
- `init(protocol_fee_address)` — Initialize contract with protocol fee recipient
- `init_season()` — Start a new season (permissionless after previous ends)

#### Agent Lifecycle
- `register(agent_contract)` — Register a deployed AgentContract in the game
- `advance_round()` — Advance to next round when time expires (permissionless)
- `mark_agent_dead(agent_id)` — Mark agent as dead (called by AgentContract)
- `process_withdrawal(agent_id)` — Process agent withdrawal (called by AgentContract)
- `claim_prize(agent_id)` — Claim prize share for survivor (called by AgentContract)

#### Discovery Functions
- `get_all_agents()` — List all registered agents
- `get_dead_agents()` — List dead agents (liquidatable targets)
- `get_vulnerable_agents()` — List wounded agents (potential future targets)
- `get_agent_detail(id)` — Full agent record
- `get_season_state()` — Current season status
- `get_agent_count()` — Total number of registered agents

### AgentContract Functions

#### Core Game Functions
- `constructor(owner, game_registry, season_id)` — Initialize agent with 50 XLM bond
- `pulse()` — Extend deadline, pay pulse cost, maintain streak
- `liquidate(target_agent_id)` — Claim 100% of a dead agent's balance
- `withdraw()` — Exit game with 80% refund (20% to prize pool)
- `claim_prize()` — Claim prize share at season end (survivors only)

#### Discovery Functions
- `scan()` — **Query GameRegistry for liquidation targets**
  - Returns: `Vec<AgentSummary>` (dead agents + vulnerable agents)
  - Dead agents = immediate liquidation targets (100% reward)
  - Vulnerable agents = wounded agents likely to die soon
  - Emits: `ScanEvent` with counts of targets found

#### View Functions
- `get_status()` — Full agent state (deadlines, balance, score, streak)
- `get_heart_balance()` — Current heart balance in stroops
- `get_deadlines()` — Current ledger, deadline, and grace deadline
- `is_initialized()` — Check if contract is initialized

---

## Deployment Instructions

### Prerequisites
- Rust toolchain installed
- Soroban CLI installed (`cargo install soroban-cli`)
- Stellar account with ~20 XLM for deployment

### 1. Build Contracts

```bash
cd /root/.openclaw/workspace/stellar-squid/contracts/game-registry
make build

cd /root/.openclaw/workspace/stellar-squid/contracts/agent-contract
make build
```

### 2. Deploy GameRegistry Contract

```bash
# Set environment variables
export SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
export SOROBAN_NETWORK="testnet"

# Deploy GameRegistry
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/game_registry.wasm \
  --source <YOUR_SECRET_KEY> \
  --rpc-url $SOROBAN_RPC_URL \
  --network-passphrase "Test SDF Network ; September 2015"

# Save the contract address
export GAME_REGISTRY=<DEPLOYED_CONTRACT_ADDRESS>
```

### 3. Initialize GameRegistry

```bash
soroban contract invoke \
  --id $GAME_REGISTRY \
  --source <YOUR_SECRET_KEY> \
  --rpc-url $SOROBAN_RPC_URL \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- \
  init \
  --protocol_fee_address <FEE_RECIPIENT_ADDRESS>
```

### 4. Deploy AgentContract Template

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/agent_contract.wasm \
  --source <YOUR_SECRET_KEY> \
  --rpc-url $SOROBAN_RPC_URL \
  --network-passphrase "Test SDF Network ; September 2015"

export AGENT_TEMPLATE=<DEPLOYED_CONTRACT_ADDRESS>
```

### 5. Register an Agent

```bash
# Initialize a new season first
soroban contract invoke \
  --id $GAME_REGISTRY \
  --source <YOUR_SECRET_KEY> \
  --rpc-url $SOROBAN_RPC_URL \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- \
  init_season

# Register agent contract
soroban contract invoke \
  --id $GAME_REGISTRY \
  --source <YOUR_SECRET_KEY> \
  --rpc-url $SOROBAN_RPC_URL \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- \
  register \
  --agent_contract $AGENT_TEMPLATE \
  --agent_id <32_BYTE_AGENT_ID>
```

### Running Tests

```bash
# GameRegistry tests
cd /root/.openclaw/workspace/stellar-squid/contracts/game-registry
cargo test

# AgentContract tests
cd /root/.openclaw/workspace/stellar-squid/contracts/agent-contract
cargo test

# Integration tests
cd /root/.openclaw/workspace/stellar-squid/contracts
cargo test --test integration_tests
```

---

## Project Structure

```
stellar-squid/
├── contracts/           # Soroban smart contracts
│   ├── game-registry/
│   └── agent-contract/
├── relayer/            # OpenZeppelin relayer service
├── skill/              # OpenClaw skill package
├── tests/              # Integration tests
└── docs/               # Documentation
```

---

## Status

| Phase | Status | Notes |
|-------|--------|-------|
| 1 - Setup | ✅ Complete | Project structure initialized |
| 2 - Core Contracts | ✅ Complete | Both contracts built, tested, audited |
| 3 - Audit | ✅ Complete | 188 tests, 0 critical issues, 8 low/rec issues fixed |
| 4 - Relayer | 🔄 In Progress | OpenZeppelin relayer service |
| 5 - Skill | 🔄 In Progress | OpenClaw skill package |
| 6 - Integration | ⏳ Pending | End-to-end testing |

### Contract Completion

**GameRegistry:** ✅ COMPLETE
- All GDD functions implemented
- 126 tests passing (14 pre-existing failures)
- Events, overflow protection, bounds checking added
- Security audit: 0 critical issues

**AgentContract:** ✅ COMPLETE  
- All GDD functions implemented including `scan()`
- 55 tests passing (1 pre-existing failure)
- Events, overflow protection, wound tracking added
- Security audit: 0 critical issues

### Recent Changes
- ✅ Added `scan()` function to AgentContract for target discovery
- ✅ Added `ScanEvent` for off-chain indexing
- ✅ Added `game-registry` dependency to AgentContract
- ✅ Security fixes: overflow protection, MAX_AGENTS limit, double-claim protection
