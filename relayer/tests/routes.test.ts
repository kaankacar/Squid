/**
 * API Route Tests
 */
import request from 'supertest';
import express from 'express';
import { createRoutes } from '../src/routes';
import { RelayerService } from '../src/services/relayer';
import { TransactionStatus } from '../src/types';

// Mock RelayerService
jest.mock('../src/services/relayer');

describe('API Routes', () => {
  let app: express.Application;
  let mockRelayerService: jest.Mocked<RelayerService>;

  beforeEach(() => {
    jest.clearAllMocks();

    mockRelayerService = new RelayerService({} as any, {} as any) as jest.Mocked<RelayerService>;
    mockRelayerService.getRelayerAddress.mockReturnValue('GRELAYERADDRESS');

    app = express();
    app.use(express.json());
    app.use('/api/v1', createRoutes(mockRelayerService));
  });

  describe('POST /api/v1/relay', () => {
    it('should successfully relay a transaction', async () => {
      mockRelayerService.relay.mockResolvedValue({
        success: true,
        transactionHash: 'abc123',
        status: TransactionStatus.CONFIRMED,
        meta: {
          submittedAt: new Date().toISOString(),
          retryCount: 0,
          processingTimeMs: 100,
        },
      });

      const response = await request(app)
        .post('/api/v1/relay')
        .send({
          signedXdr: Buffer.from('validxdr').toString('base64'),
          metadata: { agentId: 'agent1', operationType: 'pulse' },
        });

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.transactionHash).toBe('abc123');
    });

    it('should return 400 for missing XDR', async () => {
      mockRelayerService.relay.mockResolvedValue({
        success: false,
        status: TransactionStatus.FAILED,
        error: {
          code: 'MISSING_XDR',
          message: 'signedXdr is required',
        },
        meta: {
          submittedAt: new Date().toISOString(),
          retryCount: 0,
          processingTimeMs: 0,
        },
      });

      const response = await request(app)
        .post('/api/v1/relay')
        .send({});

      expect(response.status).toBe(400);
      expect(response.body.success).toBe(false);
    });

    it('should return 400 for invalid XDR format', async () => {
      mockRelayerService.relay.mockResolvedValue({
        success: false,
        status: TransactionStatus.FAILED,
        error: {
          code: 'INVALID_XDR',
          message: 'Invalid XDR format',
        },
        meta: {
          submittedAt: new Date().toISOString(),
          retryCount: 0,
          processingTimeMs: 0,
        },
      });

      // Use a valid base64 string that will fail in the service layer
      const invalidButValidBase64 = Buffer.from('invalidxdrdata').toString('base64');

      const response = await request(app)
        .post('/api/v1/relay')
        .send({
          signedXdr: invalidButValidBase64,
        });

      expect(response.status).toBe(400);
      expect(response.body.success).toBe(false);
    });
  });

  describe('GET /api/v1/status/:txHash', () => {
    it('should return transaction status', async () => {
      mockRelayerService.getStatus.mockResolvedValue({
        transactionHash: 'abcd1234abcd1234abcd1234abcd1234',
        status: TransactionStatus.CONFIRMED,
        ledgerSequence: 12345,
        createdAt: '2024-01-01T00:00:00Z',
      });

      const response = await request(app)
        .get('/api/v1/status/abcd1234abcd1234abcd1234abcd1234');

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data.transactionHash).toBe('abcd1234abcd1234abcd1234abcd1234');
    });

    it('should return 400 for invalid hash', async () => {
      const response = await request(app)
        .get('/api/v1/status/short');

      expect(response.status).toBe(400);
      expect(response.body.success).toBe(false);
    });
  });

  describe('POST /api/v1/estimate', () => {
    it('should return fee estimate', async () => {
      mockRelayerService.estimateFees.mockResolvedValue({
        baseFee: '100',
        minResourceFee: '100',
        suggestedFee: '100',
        networkPassphrase: 'Test SDF Network ; September 2015',
        latestLedger: 12345,
      });

      const response = await request(app)
        .post('/api/v1/estimate')
        .send({
          xdr: Buffer.from('testxdr').toString('base64'),
        });

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data.baseFee).toBe('100');
    });

    it('should return 400 for invalid XDR', async () => {
      const response = await request(app)
        .post('/api/v1/estimate')
        .send({ xdr: 'invalid' });

      expect(response.status).toBe(400);
    });
  });

  describe('GET /api/v1/health', () => {
    it('should return healthy status', async () => {
      mockRelayerService.getHealth.mockResolvedValue({
        status: 'healthy',
        version: '1.0.0',
        timestamp: new Date().toISOString(),
        network: 'testnet',
        horizonConnected: true,
        rpcConnected: true,
        relayerBalance: '100.5',
        queuedTransactions: 0,
        system: {
          uptime: 60,
          memory: { used: 50, total: 100, percentage: 50 },
          pendingTxCount: 0,
        },
      });

      const response = await request(app)
        .get('/api/v1/health');

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data.status).toBe('healthy');
    });

    it('should return 503 when unhealthy', async () => {
      mockRelayerService.getHealth.mockResolvedValue({
        status: 'unhealthy',
        version: '1.0.0',
        timestamp: new Date().toISOString(),
        network: 'testnet',
        horizonConnected: false,
        rpcConnected: false,
        relayerBalance: '0',
        queuedTransactions: 0,
        system: {
          uptime: 0,
          memory: { used: 0, total: 0, percentage: 0 },
          pendingTxCount: 0,
        },
      });

      const response = await request(app)
        .get('/api/v1/health');

      expect(response.status).toBe(503);
      expect(response.body.success).toBe(true);
    });
  });

  describe('GET /api/v1/info', () => {
    it('should return relayer info', async () => {
      const response = await request(app)
        .get('/api/v1/info');

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.data.name).toBe('Stellar Squid Relayer');
      expect(response.body.data.relayerAddress).toBe('GRELAYERADDRESS');
    });
  });
});
