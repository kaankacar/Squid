/**
 * Relayer Service - Main Business Logic
 * Handles transaction queuing, monitoring, and relayer wallet management
 * Uses OpenZeppelin Defender Relayer for blockchain interactions
 */
import { DefenderService } from './defender';
import logger from '../utils/logger';
import {
  RelayRequest,
  RelayResponse,
  StatusResponse,
  FeeEstimateRequest,
  FeeEstimateResponse,
  HealthResponse,
  TransactionStatus,
  QueuedTransaction,
  RelayerConfig,
} from '../types';
import cron from 'node-cron';

export class RelayerService {
  private defender: DefenderService;
  private config: RelayerConfig;
  private startTime: number = Date.now();
  private submissionQueue: QueuedTransaction[] = [];
  private processedTxs: Map<string, { status: TransactionStatus; timestamp: Date }> = new Map();

  constructor(defenderService: DefenderService, config: RelayerConfig) {
    this.defender = defenderService;
    this.config = config;

    // Start monitoring cron job
    this.startMonitoring();

    logger.info('RelayerService initialized with Defender', {
      relayerAddress: this.defender.getRelayerPublicKey(),
      network: config.network,
    });
  }

  /**
   * Initialize the service and Defender connection
   */
  async initialize(): Promise<void> {
    await this.defender.initialize();
    logger.info('RelayerService fully initialized', {
      relayerAddress: this.defender.getRelayerPublicKey(),
    });
  }

  /**
   * Submit a signed transaction to the network via Defender
   */
  async relay(request: RelayRequest): Promise<RelayResponse> {
    logger.info('Relay request received', {
      operationType: request.metadata?.operationType,
      agentId: request.metadata?.agentId,
    });

    // Validate request
    if (!request.signedXdr) {
      return {
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
      };
    }

    // Submit via Defender service
    const result = await this.defender.submitTransaction(
      request,
      this.config.maxRetries
    );

    // Track processed transaction
    if (result.transactionHash) {
      this.processedTxs.set(result.transactionHash, {
        status: result.status,
        timestamp: new Date(),
      });

      // Clean up old entries (keep last 1000)
      if (this.processedTxs.size > 1000) {
        const oldestKey = this.processedTxs.keys().next().value as string;
        this.processedTxs.delete(oldestKey);
      }
    }

    return result;
  }

  /**
   * Get transaction status
   */
  async getStatus(txHash: string): Promise<StatusResponse> {
    // Check cache first
    const cached = this.processedTxs.get(txHash);
    if (cached) {
      return {
        transactionHash: txHash,
        status: cached.status,
        createdAt: cached.timestamp.toISOString(),
      };
    }

    // Query from network
    return this.defender.getTransactionStatus(txHash);
  }

  /**
   * Estimate fees for a transaction
   */
  async estimateFees(request: FeeEstimateRequest): Promise<FeeEstimateResponse> {
    return this.defender.estimateFees(request);
  }

  /**
   * Get health status
   */
  async getHealth(): Promise<HealthResponse> {
    const [horizonConnected, defenderConnected, balance] = await Promise.all([
      this.defender.isHorizonConnected(),
      this.defender.isDefenderConnected(),
      this.defender.getRelayerBalance(),
    ]);

    // Determine overall status
    let status: 'healthy' | 'degraded' | 'unhealthy' = 'healthy';
    if (!horizonConnected || !defenderConnected) {
      status = 'unhealthy';
    } else if (parseFloat(balance) < 10) {
      status = 'degraded'; // Low balance warning
    }

    // Get memory stats
    const memUsage = process.memoryUsage();
    const memStats = {
      used: Math.round(memUsage.heapUsed / 1024 / 1024),
      total: Math.round(memUsage.heapTotal / 1024 / 1024),
      percentage: Math.round((memUsage.heapUsed / memUsage.heapTotal) * 100),
    };

    return {
      status,
      version: process.env.npm_package_version || '1.0.0',
      timestamp: new Date().toISOString(),
      network: this.defender.getNetwork(),
      horizonConnected,
      rpcConnected: defenderConnected, // Defender acts as RPC
      relayerBalance: balance,
      queuedTransactions: this.defender.getPendingCount(),
      system: {
        uptime: Math.floor((Date.now() - this.startTime) / 1000),
        memory: memStats,
        pendingTxCount: this.submissionQueue.length,
      },
    };
  }

  /**
   * Start monitoring cron job
   * Periodically cleans up old transactions and checks relayer health
   */
  private startMonitoring(): void {
    // Run every 5 minutes
    cron.schedule('*/5 * * * *', async () => {
      try {
        logger.debug('Running monitoring cron job');

        // Clean up old processed transactions
        const now = Date.now();
        const maxAge = 24 * 60 * 60 * 1000; // 24 hours

        for (const [hash, data] of this.processedTxs) {
          if (now - data.timestamp.getTime() > maxAge) {
            this.processedTxs.delete(hash);
          }
        }

        // Check relayer balance and alert if low
        const balance = await this.defender.getRelayerBalance();
        const balanceXlm = parseFloat(balance);

        if (balanceXlm < 5) {
          logger.error('CRITICAL: Relayer balance is critically low', {
            balance: balanceXlm,
            address: this.defender.getRelayerPublicKey(),
          });
        } else if (balanceXlm < 20) {
          logger.warn('Relayer balance is low', {
            balance: balanceXlm,
            address: this.defender.getRelayerPublicKey(),
          });
        }

        logger.debug('Monitoring check completed', {
          processedTxCount: this.processedTxs.size,
          balance: balanceXlm,
        });
      } catch (error) {
        logger.error('Error in monitoring cron job', {
          error: error instanceof Error ? error.message : 'Unknown error',
        });
      }
    });

    logger.info('Monitoring cron job started');
  }

  /**
   * Get relayer wallet address
   */
  getRelayerAddress(): string {
    return this.defender.getRelayerPublicKey();
  }

  /**
   * Get processed transaction count
   */
  getProcessedCount(): number {
    return this.processedTxs.size;
  }
}
