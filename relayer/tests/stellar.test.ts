/**
 * Tests for StellarService
 */
import { StellarService } from '../src/services/stellar';
import { TransactionStatus } from '../src/types';

// Mock the Stellar SDK
jest.mock('@stellar/stellar-sdk', () => ({
  Horizon: {
    Server: jest.fn().mockImplementation(() => ({
      submitTransaction: jest.fn(),
      ledgers: jest.fn().mockReturnValue({
        order: jest.fn().mockReturnValue({
          limit: jest.fn().mockResolvedValue({
            records: [{ sequence: 12345 }],
          }),
        }),
      }),
      feeStats: jest.fn().mockResolvedValue({
        last_ledger_base_fee: '100',
      }),
      transactions: jest.fn().mockReturnValue({
        transaction: jest.fn().mockReturnValue({
          call: jest.fn().mockResolvedValue({
            hash: 'abc123',
            ledger: 12345,
            created_at: '2024-01-01T00:00:00Z',
            memo: 'test',
            operation_count: 1,
            fee_charged: '100',
            result_xdr: 'test',
            result_meta_xdr: 'test',
          }),
        }),
      }),
      loadAccount: jest.fn().mockResolvedValue({
        balances: [
          { asset_type: 'native', balance: '100.5' },
        ],
      }),
    })),
  },
  rpc: {
    Server: jest.fn().mockImplementation(() => ({
      getLatestLedger: jest.fn().mockResolvedValue({ sequence: 12345 }),
    })),
  },
  TransactionBuilder: {
    fromXDR: jest.fn().mockImplementation(() => ({
      source: 'GXXX',
      sequence: '123',
      operations: [{ type: 'payment' }],
      fee: '100',
      hash: jest.fn().mockReturnValue(Buffer.from('abc123')),
    })),
  },
  Networks: {
    PUBLIC: 'Public Global Stellar Network ; September 2015',
    TESTNET: 'Test SDF Network ; September 2015',
    FUTURENET: 'Test SDF Future Network ; October 2022',
  },
  Keypair: {
    fromSecret: jest.fn().mockReturnValue({
      publicKey: jest.fn().mockReturnValue('GRELAYERADDRESS'),
    }),
  },
  Transaction: class Transaction {},
}));

describe('StellarService', () => {
  let service: StellarService;
  const mockConfig = {
    horizonUrl: 'https://horizon-testnet.stellar.org',
    rpcUrl: 'https://soroban-testnet.stellar.org',
    relayerSecretKey: 'SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
    network: 'testnet' as const,
  };

  beforeEach(() => {
    jest.clearAllMocks();
    service = new StellarService(mockConfig);
  });

  describe('constructor', () => {
    it('should initialize with the provided config', () => {
      expect(service).toBeDefined();
      expect(service.getRelayerPublicKey()).toBe('GRELAYERADDRESS');
      expect(service.getNetwork()).toBe('testnet');
    });
  });

  describe('submitTransaction', () => {
    it('should successfully submit a valid transaction', async () => {
      const request = {
        signedXdr: 'validbase64xdr',
        metadata: { agentId: 'agent1', operationType: 'pulse' },
      };

      const result = await service.submitTransaction(request);

      expect(result.success).toBe(true);
      expect(result.status).toBe(TransactionStatus.CONFIRMED);
      expect(result.transactionHash).toBeDefined();
    });

    it('should handle missing XDR', async () => {
      const request = {
        signedXdr: '',
      };

      const result = await service.submitTransaction(request);

      expect(result.success).toBe(false);
      expect(result.status).toBe(TransactionStatus.FAILED);
      expect(result.error).toBeDefined();
    });

    it('should handle invalid XDR', async () => {
      const { TransactionBuilder } = require('@stellar/stellar-sdk');
      TransactionBuilder.fromXDR.mockImplementationOnce(() => {
        throw new Error('Invalid XDR');
      });

      const request = {
        signedXdr: 'invalidxdr',
      };

      const result = await service.submitTransaction(request);

      expect(result.success).toBe(false);
      expect(result.status).toBe(TransactionStatus.FAILED);
    });
  });

  describe('getTransactionStatus', () => {
    it('should return confirmed status for existing transaction', async () => {
      const status = await service.getTransactionStatus('abc123');

      expect(status.status).toBe(TransactionStatus.CONFIRMED);
      expect(status.transactionHash).toBe('abc123');
      expect(status.ledgerSequence).toBe(12345);
    });

    it('should return not found for unknown transaction', async () => {
      const { Horizon } = require('@stellar/stellar-sdk');
      Horizon.Server.mockImplementationOnce(() => ({
        transactions: jest.fn().mockReturnValue({
          transaction: jest.fn().mockReturnValue({
            call: jest.fn().mockRejectedValue(new Error('Not found')),
          }),
        }),
      }));

      const newService = new StellarService(mockConfig);
      const status = await newService.getTransactionStatus('unknown');

      expect(status.status).toBe(TransactionStatus.NOT_FOUND);
    });
  });

  describe('estimateFees', () => {
    it('should return fee estimates', async () => {
      const request = { xdr: 'validxdr' };
      const estimate = await service.estimateFees(request);

      expect(estimate.baseFee).toBeDefined();
      expect(estimate.minResourceFee).toBe('100');
      expect(estimate.suggestedFee).toBeDefined();
      expect(estimate.networkPassphrase).toBeDefined();
      expect(estimate.latestLedger).toBe(12345);
    });
  });

  describe('getRelayerBalance', () => {
    it('should return relayer balance', async () => {
      const balance = await service.getRelayerBalance();
      expect(balance).toBe('100.5');
    });
  });

  describe('isHorizonConnected', () => {
    it('should return true when connected', async () => {
      const connected = await service.isHorizonConnected();
      expect(connected).toBe(true);
    });
  });

  describe('isRpcConnected', () => {
    it('should return true when connected', async () => {
      const connected = await service.isRpcConnected();
      expect(connected).toBe(true);
    });
  });
});
