# Stellar Squid Relayer Service

A production-ready transaction relayer service for Stellar Squid. Accepts signed transactions from OpenClaw agents and submits them to the Stellar network.

## Overview

The relayer service is a critical infrastructure component for Stellar Squid that:
- Accepts pre-signed transactions from agents via REST API
- Submits transactions to Stellar Testnet/Mainnet
- Handles retries and error recovery
- Tracks transaction status
- Manages the relayer wallet (funded by 5% pulse fee)

## Architecture

```
┌─────────────┐     POST /relay      ┌─────────────────┐     ┌─────────────────┐
│ OpenClaw    │ ───────────────────> │ Relayer Service │ ───> │ Stellar Network │
│ Agent       │    (signed XDR)      │                 │     │ (Testnet/Mainnet)│
└─────────────┘                      └─────────────────┘     └─────────────────┘
                                           │
                                           │ Manages
                                           ▼
                                    ┌─────────────────┐
                                    │ Relayer Wallet  │
                                    │ (Funded by 5%   │
                                    │  pulse fees)    │
                                    └─────────────────┘
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/relay` | Submit a signed transaction |
| GET | `/api/v1/status/:txHash` | Check transaction status |
| POST | `/api/v1/estimate` | Get fee estimate |
| GET | `/api/v1/health` | Health check |
| GET | `/api/v1/info` | Service information |

### POST /api/v1/relay

Submit a signed transaction to be relayed to the Stellar network.

**Request:**
```json
{
  "signedXdr": "AAAA...base64...",
  "metadata": {
    "agentId": "agent123",
    "operationType": "pulse",
    "estimatedLedger": 12345
  }
}
```

**Response:**
```json
{
  "success": true,
  "transactionHash": "abc123...",
  "ledgerSequence": 12345,
  "status": "confirmed",
  "meta": {
    "submittedAt": "2024-01-01T00:00:00Z",
    "retryCount": 0,
    "processingTimeMs": 150
  }
}
```

### GET /api/v1/status/:txHash

Check the status of a previously submitted transaction.

**Response:**
```json
{
  "success": true,
  "data": {
    "transactionHash": "abc123...",
    "status": "confirmed",
    "ledgerSequence": 12345,
    "createdAt": "2024-01-01T00:00:00Z",
    "feeCharged": "100"
  }
}
```

### POST /api/v1/estimate

Get a fee estimate for a transaction.

**Request:**
```json
{
  "xdr": "AAAA...unsigned..."
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "baseFee": "100",
    "minResourceFee": "100",
    "suggestedFee": "100",
    "networkPassphrase": "Test SDF Network ; September 2015",
    "latestLedger": 12345
  }
}
```

### GET /api/v1/health

Health check endpoint.

**Response:**
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "1.0.0",
    "timestamp": "2024-01-01T00:00:00Z",
    "network": "testnet",
    "horizonConnected": true,
    "rpcConnected": true,
    "relayerBalance": "100.5",
    "queuedTransactions": 0,
    "system": {
      "uptime": 3600,
      "memory": { "used": 50, "total": 100, "percentage": 50 },
      "pendingTxCount": 0
    }
  }
}
```

## Installation

```bash
# Clone the repository
cd stellar-squid/relayer

# Install dependencies
npm install

# Copy environment file
cp .env.example .env

# Edit .env with your configuration
nano .env
```

## Configuration

Create a `.env` file with the following variables:

```env
PORT=3000
NODE_ENV=production

# Stellar Network
STELLAR_NETWORK=testnet  # or 'public' for mainnet
STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
STELLAR_RPC_URL=https://soroban-testnet.stellar.org

# Relayer Wallet (KEEP SECURE!)
RELAYER_SECRET_KEY=SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Protocol Fee Address (receives 5% pulse fee)
PROTOCOL_FEE_ADDRESS=GPROTOCOLFEEADDRESS

# Rate Limiting
RATE_LIMIT_WINDOW_MS=60000
RATE_LIMIT_MAX_REQUESTS=100

# Transaction Settings
MAX_RETRIES=3
RETRY_DELAY_MS=1000
TX_TIMEOUT_SECONDS=30

# Logging
LOG_LEVEL=info
```

## Running the Service

### Development

```bash
npm run dev
```

### Production

```bash
npm run build
npm start
```

### Docker (optional)

```bash
docker build -t stellar-squid-relayer .
docker run -p 3000:3000 --env-file .env stellar-squid-relayer
```

## Testing

```bash
# Run all tests
npm test

# Run tests in watch mode
npm run test:watch

# Run tests with coverage
npm run test -- --coverage
```

## Relayer Wallet Funding

The relayer wallet must be funded with XLM to pay for transaction fees. The wallet is funded by the 5% pulse fee that agents pay.

**To fund the relayer:**
1. Get the relayer address from `/api/v1/info`
2. Send XLM to that address
3. Monitor balance via `/api/v1/health`

## Error Handling

The service handles various error scenarios:

- **Rate limiting**: Returns 429 with retry-after header
- **Invalid XDR**: Returns 400 with validation details
- **Network errors**: Automatic retries with exponential backoff
- **Low balance**: Health endpoint returns 'degraded' status

## Monitoring

Monitor the service health via:
- `/api/v1/health` - Service health and metrics
- `/logs/combined.log` - Application logs
- `/logs/error.log` - Error logs

## Security Considerations

- Keep `RELAYER_SECRET_KEY` secure
- Use HTTPS in production
- Set up proper firewall rules
- Monitor for unusual activity
- Regularly rotate keys

## License

MIT
