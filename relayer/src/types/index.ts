/**
 * Relayer Service Types
 * Stellar Squid - Transaction Relay Service
 */

export interface RelayRequest {
  /** Base64 encoded signed transaction XDR */
  signedXdr: string;
  /** Optional transaction metadata */
  metadata?: TransactionMetadata;
}

export interface TransactionMetadata {
  /** Agent identifier */
  agentId?: string;
  /** Operation type (pulse, scan, liquidate, etc.) */
  operationType?: string;
  /** Estimated ledger sequence */
  estimatedLedger?: number;
}

export interface RelayResponse {
  success: boolean;
  transactionHash?: string;
  ledgerSequence?: number;
  status: TransactionStatus;
  error?: RelayError;
  meta: RelayMeta;
}

export interface RelayMeta {
  submittedAt: string;
  retryCount: number;
  processingTimeMs: number;
}

export interface StatusResponse {
  transactionHash: string;
  status: TransactionStatus;
  ledgerSequence?: number;
  createdAt?: string;
  memo?: string;
  operationCount?: number;
  feeCharged?: string;
  resultXdr?: string;
  resultMetaXdr?: string;
  error?: string;
}

export interface FeeEstimateRequest {
  /** Transaction XDR (unsigned) */
  xdr: string;
}

export interface FeeEstimateResponse {
  baseFee: string;
  minResourceFee: string;
  suggestedFee: string;
  networkPassphrase: string;
  latestLedger: number;
}

export interface HealthResponse {
  status: 'healthy' | 'degraded' | 'unhealthy';
  version: string;
  timestamp: string;
  network: string;
  horizonConnected: boolean;
  rpcConnected: boolean;
  relayerBalance: string;
  queuedTransactions: number;
  system: SystemHealth;
}

export interface SystemHealth {
  uptime: number;
  memory: MemoryStats;
  pendingTxCount: number;
}

export interface MemoryStats {
  used: number;
  total: number;
  percentage: number;
}

export enum TransactionStatus {
  PENDING = 'pending',
  SUBMITTED = 'submitted',
  CONFIRMED = 'confirmed',
  FAILED = 'failed',
  EXPIRED = 'expired',
  NOT_FOUND = 'not_found',
  RETRYING = 'retrying'
}

export interface RelayError {
  code: string;
  message: string;
  details?: unknown;
}

export interface QueuedTransaction {
  id: string;
  signedXdr: string;
  status: TransactionStatus;
  retries: number;
  submittedAt: Date;
  lastError?: string;
  metadata?: TransactionMetadata;
}

export interface RelayerConfig {
  port: number;
  network: 'testnet' | 'public' | 'futurenet';
  horizonUrl: string;
  rpcUrl: string;
  relayerSecretKey: string;
  protocolFeeAddress: string;
  maxRetries: number;
  retryDelayMs: number;
  txTimeoutSeconds: number;
  rateLimitWindowMs: number;
  rateLimitMaxRequests: number;
  logLevel: string;
}
