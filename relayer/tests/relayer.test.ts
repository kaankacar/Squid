/**
 * Tests for RelayerService
 */
import { RelayerService } from '../src/services/relayer';
import { StellarService } from '../src/services/stellar';
import { TransactionStatus, RelayerConfig } from '../src/types';

// Mock StellarService
jest.mock('../src/services/stellar');

describe('RelayerService', () => {
  let relayerService: RelayerService;
  let mockStellarService: jest.Mocked<StellarService>;
  const mockConfig: RelayerConfig = {
    port: 3000,
    network: 'testnet',
    horizonUrl: 'https://horizon-testnet.stellar.org',
    rpcUrl: 'https://soroban-testnet.stellar.org',
    relayerSecretKey: 'SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
    protocolFeeAddress: 'GPROTOCOLFEEADDRESS',
    maxRetries: 3,
    retryDelayMs: 1000,
    txTimeoutSeconds: 30,
    rateLimitWindowMs: 60000,
    rateLimitMaxRequests: 100,
    logLevel: 'info',
  };

  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();

    mockStellarService = new StellarService(mockConfig) as jest.Mocked<StellarService>;
    mockStellarService.getRelayerPublicKey.mockReturnValue('GRELAYERADDRESS');
    mockStellarService.getNetwork.mockReturnValue('testnet');
    mockStellarService.getRelayerBalance.mockResolvedValue('100.5');
    mockStellarService.isHorizonConnected.mockResolvedValue(true);
    mockStellarService.isRpcConnected.mockResolvedValue(true);
    mockStellarService.getPendingCount.mockReturnValue(0);

    relayerService = new RelayerService(mockStellarService, mockConfig);
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  describe('constructor', () => {
    it('should initialize with stellar service and config', () => {
      expect(relayerService).toBeDefined();
      expect(relayerService.getRelayerAddress()).toBe('GRELAYERADDRESS');
    });
  });

  describe('relay', () => {
    it('should relay a valid transaction', async () => {
      mockStellarService.submitTransaction.mockResolvedValue({
        success: true,
        transactionHash: 'abc123',
        status: TransactionStatus.CONFIRMED,
        meta: {
          submittedAt: new Date().toISOString(),
          retryCount: 0,
          processingTimeMs: 100,
        },
      });

      const result = await relayerService.relay({
        signedXdr: 'validxdr',
        metadata: { agentId: 'agent1', operationType: 'pulse' },
      });

      expect(result.success).toBe(true);
      expect(result.transactionHash).toBe('abc123');
      expect(result.status).toBe(TransactionStatus.CONFIRMED);
    });

    it('should reject missing XDR', async () => {
      const result = await relayerService.relay({
        signedXdr: '',
      });

      expect(result.success).toBe(false);
      expect(result.error?.code).toBe('MISSING_XDR');
    });

    it('should handle submission failure', async () => {
      mockStellarService.submitTransaction.mockResolvedValue({
        success: false,
        status: TransactionStatus.FAILED,
        error: {
          code: 'SUBMISSION_ERROR',
          message: 'Network error',
        },
        meta: {
          submittedAt: new Date().toISOString(),
          retryCount: 0,
          processingTimeMs: 100,
        },
      });

      const result = await relayerService.relay({
        signedXdr: 'validxdr',
      });

      expect(result.success).toBe(false);
      expect(result.status).toBe(TransactionStatus.FAILED);
    });
  });

  describe('getStatus', () => {
    it('should return transaction status', async () => {
      mockStellarService.getTransactionStatus.mockResolvedValue({
        transactionHash: 'abc123',
        status: TransactionStatus.CONFIRMED,
        ledgerSequence: 12345,
      });

      const status = await relayerService.getStatus('abc123');

      expect(status.transactionHash).toBe('abc123');
      expect(status.status).toBe(TransactionStatus.CONFIRMED);
    });
  });

  describe('estimateFees', () => {
    it('should return fee estimates', async () => {
      mockStellarService.estimateFees.mockResolvedValue({
        baseFee: '100',
        minResourceFee: '100',
        suggestedFee: '100',
        networkPassphrase: 'Test SDF Network ; September 2015',
        latestLedger: 12345,
      });

      const estimate = await relayerService.estimateFees({ xdr: 'testxdr' });

      expect(estimate.baseFee).toBe('100');
      expect(estimate.latestLedger).toBe(12345);
    });
  });

  describe('getHealth', () => {
    it('should return healthy status when all systems operational', async () => {
      const health = await relayerService.getHealth();

      expect(health.status).toBe('healthy');
      expect(health.horizonConnected).toBe(true);
      expect(health.rpcConnected).toBe(true);
      expect(health.relayerBalance).toBe('100.5');
      expect(health.system.uptime).toBeGreaterThanOrEqual(0);
      expect(health.system.memory).toBeDefined();
    });

    it('should return unhealthy when services disconnected', async () => {
      mockStellarService.isHorizonConnected.mockResolvedValue(false);

      const health = await relayerService.getHealth();

      expect(health.status).toBe('unhealthy');
    });

    it('should return degraded when balance is low', async () => {
      mockStellarService.getRelayerBalance.mockResolvedValue('5');

      const health = await relayerService.getHealth();

      expect(health.status).toBe('degraded');
    });
  });

  describe('getRelayerAddress', () => {
    it('should return the relayer address', () => {
      const address = relayerService.getRelayerAddress();
      expect(address).toBe('GRELAYERADDRESS');
    });
  });

  describe('getProcessedCount', () => {
    it('should return the count of processed transactions', () => {
      const count = relayerService.getProcessedCount();
      expect(typeof count).toBe('number');
      expect(count).toBeGreaterThanOrEqual(0);
    });
  });
});
