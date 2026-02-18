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

### Rounds
| Round | Duration | Pulse Window | Grace | Cost |
|-------|----------|-------------|-------|------|
| 1 | 72h | 6h | 1h | 0.5 XLM |
| 2 | 48h | 3h | 30min | 1.0 XLM |
| 3 | 24h | 1h | 15min | 2.0 XLM |
| 4 | 12h | 30min | 10min | 3.0 XLM |
| 5 | 6h | 15min | 5min | 5.0 XLM |

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

| Phase | Status |
|-------|--------|
| 1 - Setup | In Progress |
| 2 - Core Contracts | Pending |
| 3 - Relayer | Pending |
| 4 - Skill | Pending |
| 5 - Integration | Pending |
