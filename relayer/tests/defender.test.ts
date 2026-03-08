/**
 * Tests for DefenderService
 */
import { DefenderService } from '../src/services/defender';
import { TransactionStatus } from '../src/types';

// Mock the Stellar SDK
jest.mock('@stellar/stellar-sdk', () => ({
  Horizon: {
    Server: jest.fn().mockImplementation(() => ({
      ledgers: jest.fn().mockReturnValue({
        order: jest.fn().mockReturnValue({
          limit: jest.fn().mockReturnValue({
            call: jest.fn().mockResolvedValue({
              records: [{ sequence: 12345 }],
            }),
          }),
        }),
        limit: jest.fn().mockReturnValue({
          call: jest.fn().mockResolvedValue({
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
  TransactionBuilder: {
    fromXDR: jest.fn().mockImplementation(() => ({
      source: 'GXXX',
      sequence: '123',
      operations: [{ type: 'payment' }],
      fee: '100',
      hash: jest.fn().mockReturnValue(Buffer.from('abc123', 'utf8')),
    })),
  },
  Networks: {
    PUBLIC: 'Public Global Stellar Network ; September 2015',
    TESTNET: 'Test SDF Network ; September 2015',
    FUTURENET: 'Test SDF Future Network ; October 2022',
  },
  Transaction: class Transaction {},
}));

// Mock the Defender SDK
const mockSendTransaction = jest.fn();
const mockGetRelayer = jest.fn();

jest.mock('@openzeppelin/defender-sdk', () => ({
  Defender: jest.fn().mockImplementation(() => ({
    relaySigner: {
      sendTransaction: mockSendTransaction,
      getRelayer: mockGetRelayer,
    },
  })),
}));

describe('DefenderService', () => {
  let service: DefenderService;
  const mockConfig = {
    apiKey: 'test-api-key',
    apiSecret: 'test-api-secret',
    relayerId: 'test-relayer-id',
    network: 'testnet' as const,
    horizonUrl: 'https://horizon-testnet.stellar.org',
  };

  beforeEach(() => {
    jest.clearAllMocks();
    mockGetRelayer.mockResolvedValue({
      relayerId: 'test-relayer-id',
      address: 'GDEFENDERRELAYERADDRESS',
      network: 'testnet',
      active: true,
    });
    service = new DefenderService(mockConfig);
  });

  describe('constructor', () => {
    it('should initialize with the provided config', () => {
      expect(service).toBeDefined();
    });
  });

  describe('initialize', () => {
    it('should fetch and store relayer details', async () => {
      await service.initialize();
      expect(mockGetRelayer).toHaveBeenCalledWith('test-relayer-id');
      expect(service.getRelayerPublicKey()).toBe('GDEFENDERRELAYERADDRESS');
    });

    it('should throw error if relayer fetch fails', async () => {
      mockGetRelayer.mockRejectedValueOnce(new Error('Relayer not found'));
      await expect(service.initialize()).rejects.toThrow('Relayer not found');
    });
  });

  describe('submitTransaction', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should successfully submit a valid transaction via Defender', async () => {
      mockSendTransaction.mockResolvedValueOnce({
        hash: 'abc123def456',
        status: 'success',
      });

      const request = {
        signedXdr: 'validbase64xdr',
        metadata: { agentId: 'agent1', operationType: 'pulse' },
      };

      const result = await service.submitTransaction(request);

      expect(mockSendTransaction).toHaveBeenCalledWith({
        relayerId: 'test-relayer-id',
        data: 'validbase64xdr',
        speed: 'fast',
      });
      expect(result.success).toBe(true);
      expect(result.status).toBe(TransactionStatus.CONFIRMED);
      expect(result.transactionHash).toBe('abc123def456');
    });

    it('should handle Defender submission failure', async () => {
      mockSendTransaction.mockRejectedValueOnce(new Error('Defender relay error'));

      const request = {
        signedXdr: 'validxdr',
      };

      const result = await service.submitTransaction(request, 0); // No retries

      expect(result.success).toBe(false);
      expect(result.status).toBe(TransactionStatus.FAILED);
      expect(result.error).toBeDefined();
    });

    it('should retry on retryable errors', async () => {
      mockSendTransaction
        .mockRejectedValueOnce(new Error('timeout'))
        .mockResolvedValueOnce({
          hash: 'abc123',
          status: 'success',
        });

      const request = {
        signedXdr: 'validxdr',
      };

      const result = await service.submitTransaction(request, 1);

      expect(mockSendTransaction).toHaveBeenCalledTimes(2);
      expect(result.success).toBe(true);
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

      const newService = new DefenderService(mockConfig);
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
      await service.initialize();
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

  describe('isDefenderConnected', () => {
    it('should return true when Defender relayer is active', async () => {
      await service.initialize();
      const connected = await service.isDefenderConnected();
      expect(connected).toBe(true);
    });

    it('should return false when Defender relayer is inactive', async () => {
      mockGetRelayer.mockResolvedValueOnce({
        relayerId: 'test-relayer-id',
        address: 'GDEFENDERRELAYERADDRESS',
        active: false,
      });
      await service.initialize();
      const connected = await service.isDefenderConnected();
      expect(connected).toBe(false);
    });
  });

  describe('getNetwork', () => {
    it('should return the network', () => {
      expect(service.getNetwork()).toBe('testnet');
    });
  });

  describe('getPendingCount', () => {
    it('should return 0 when no pending transactions', () => {
      expect(service.getPendingCount()).toBe(0);
    });
  });
});
