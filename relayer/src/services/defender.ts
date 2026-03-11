/**
 * OpenZeppelin Defender Service - Blockchain Interactions
 * Handles all Stellar network operations via OpenZeppelin Defender Relayer
 */
import { Defender } from '@openzeppelin/defender-sdk';
import {
  Horizon,
  TransactionBuilder,
  Transaction,
  Networks,
} from '@stellar/stellar-sdk';
import crypto from 'crypto';
import logger from '../utils/logger';
import {
  RelayRequest,
  RelayResponse,
  StatusResponse,
  FeeEstimateRequest,
  FeeEstimateResponse,
  TransactionStatus,
  RelayError,
  QueuedTransaction,
} from '../types';

export interface DefenderConfig {
  apiKey: string;
  apiSecret: string;
  relayerId: string;
  network: 'testnet' | 'public' | 'futurenet';
  horizonUrl: string;
}

export class DefenderService {
  private client: Defender;
  private relayerId: string;
  private horizon: Horizon.Server;
  private network: string;
  private networkPassphrase: string;
  private pendingTransactions: Map<string, QueuedTransaction> = new Map();
  private relayerAddress: string = '';

  constructor(config: DefenderConfig) {
    // Initialize Defender client
    this.client = new Defender({
      apiKey: config.apiKey,
      apiSecret: config.apiSecret,
    });

    this.relayerId = config.relayerId;
    this.horizon = new Horizon.Server(config.horizonUrl);
    this.network = config.network;
    this.networkPassphrase = this.getNetworkPassphrase(config.network);

    logger.info('DefenderService initialized', {
      network: config.network,
      relayerId: config.relayerId,
    });
  }

  /**
   * Initialize the service and fetch relayer details
   */
  async initialize(): Promise<void> {
    try {
      const relayer = await this.client.relaySigner.getRelayer(this.relayerId);
      this.relayerAddress = relayer.address;
      logger.info('Defender relayer loaded', {
        relayerId: this.relayerId,
        address: this.relayerAddress,
        network: relayer.network,
      });
    } catch (error) {
      logger.error('Failed to initialize Defender relayer', {
        error: error instanceof Error ? error.message : 'Unknown error',
        relayerId: this.relayerId,
      });
      throw error;
    }
  }

  private getNetworkPassphrase(network: string): string {
    switch (network) {
      case 'public':
        return Networks.PUBLIC;
      case 'testnet':
        return Networks.TESTNET;
      case 'futurenet':
        return Networks.FUTURENET;
      default:
        return Networks.TESTNET;
    }
  }

  /**
   * Submit a signed transaction via Defender Relayer
   */
  async submitTransaction(
    request: RelayRequest,
    maxRetries: number = 3
  ): Promise<RelayResponse> {
    const startTime = Date.now();
    const txId = this.generateTxId();

    try {
      // Decode and validate the signed transaction
      const transaction = TransactionBuilder.fromXDR(
        request.signedXdr,
        this.networkPassphrase
      ) as Transaction;

      logger.info('Processing relay request via Defender', {
        txId,
        source: transaction.source,
        seqNum: transaction.sequence,
        operationCount: transaction.operations.length,
        metadata: request.metadata,
      });

      // Queue the transaction
      // Performance optimization: Precompute the transaction hash here to avoid
      // expensive XDR parsing during repeated status checks in getTransactionStatus
      const queuedTx: QueuedTransaction = {
        id: txId,
        hash: transaction.hash().toString('hex'),
        signedXdr: request.signedXdr,
        status: TransactionStatus.PENDING,
        retries: 0,
        submittedAt: new Date(),
        metadata: request.metadata,
      };

      this.pendingTransactions.set(txId, queuedTx);

      // Submit via Defender with retry logic
      const result = await this.submitWithDefender(
        request.signedXdr,
        queuedTx,
        maxRetries
      );

      // Update queue status
      queuedTx.status = result.status;
      this.pendingTransactions.set(txId, queuedTx);

      const processingTime = Date.now() - startTime;

      return {
        success: result.status === TransactionStatus.CONFIRMED,
        transactionHash: result.hash,
        ledgerSequence: result.ledger,
        status: result.status,
        error: result.error,
        meta: {
          submittedAt: new Date().toISOString(),
          retryCount: queuedTx.retries,
          processingTimeMs: processingTime,
        },
      };
    } catch (error) {
      const processingTime = Date.now() - startTime;
      logger.error('Transaction submission failed via Defender', {
        txId,
        error: error instanceof Error ? error.message : 'Unknown error',
      });

      return {
        success: false,
        status: TransactionStatus.FAILED,
        error: this.formatError(error),
        meta: {
          submittedAt: new Date().toISOString(),
          retryCount: 0,
          processingTimeMs: processingTime,
        },
      };
    }
  }

  /**
   * Submit transaction via Defender Relayer with retry logic
   */
  private async submitWithDefender(
    signedXdr: string,
    queuedTx: QueuedTransaction,
    maxRetries: number
  ): Promise<{
    hash?: string;
    ledger?: number;
    status: TransactionStatus;
    error?: RelayError;
  }> {
    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        if (attempt > 0) {
          logger.info(`Defender retry attempt ${attempt}/${maxRetries}`, {
            txId: queuedTx.id,
          });
          queuedTx.status = TransactionStatus.RETRYING;
          queuedTx.retries = attempt;

          // Wait before retry
          await this.delay(1000 * attempt);
        }

        // Submit via Defender Relayer
        // Note: Defender's signAndSendTransaction is used for EVM chains
        // For Stellar, we use the relayer's built-in transaction submission
        const response = await this.client.relaySigner.sendTransaction({
          relayerId: this.relayerId,
          // For Stellar, we pass the signed XDR directly
          data: signedXdr,
          speed: 'fast',
        });

        logger.info('Transaction submitted successfully via Defender', {
          txId: queuedTx.id,
          hash: response.hash,
          attempt,
        });

        // Query Horizon for ledger sequence
        let ledgerSequence: number | undefined;
        try {
          const tx = await this.horizon.transactions().transaction(response.hash).call();
          ledgerSequence = Number(tx.ledger);
        } catch {
          // Ignore, ledger sequence is optional
        }

        return {
          hash: response.hash,
          ledger: ledgerSequence,
          status: TransactionStatus.CONFIRMED,
        };
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));
        queuedTx.lastError = lastError.message;

        // Check if error is retryable
        if (!this.isRetryableError(lastError)) {
          logger.warn('Non-retryable error from Defender, aborting', {
            txId: queuedTx.id,
            error: lastError.message,
          });
          break;
        }

        logger.warn(`Defender submission attempt ${attempt + 1} failed`, {
          txId: queuedTx.id,
          error: lastError.message,
        });
      }
    }

    return {
      status: TransactionStatus.FAILED,
      error: this.formatError(lastError || new Error('Max retries exceeded')),
    };
  }

  /**
   * Check if error is retryable
   */
  private isRetryableError(error: Error): boolean {
    const retryableCodes = [
      'tx_bad_seq',
      'tx_insufficient_fee',
      'tx_timeout',
      'tx_too_late',
      'timeout',
      'rate_limit_exceeded',
      'connection_error',
      'ECONNRESET',
      'ETIMEDOUT',
      'RELAYER_ERROR',
    ];

    const errorMessage = error.message.toLowerCase();
    return retryableCodes.some((code) => errorMessage.includes(code));
  }

  /**
   * Get transaction status
   */
  async getTransactionStatus(txHash: string): Promise<StatusResponse> {
    try {
      // First check if it's in our pending queue
      for (const [id, tx] of this.pendingTransactions) {
        if (id === txHash || tx.hash === txHash) {
          return {
            transactionHash: txHash,
            status: tx.status,
            createdAt: tx.submittedAt.toISOString(),
            error: tx.lastError,
          };
        }
      }

      // Query Horizon for the transaction
      try {
        const response = await this.horizon.transactions().transaction(txHash).call();

        return {
          transactionHash: txHash,
          status: TransactionStatus.CONFIRMED,
          ledgerSequence: Number(response.ledger),
          createdAt: response.created_at,
          memo: response.memo,
          operationCount: response.operation_count,
          feeCharged: String(response.fee_charged),
          resultXdr: response.result_xdr,
          resultMetaXdr: response.result_meta_xdr,
        };
      } catch (horizonError) {
        // Transaction not found on network
        return {
          transactionHash: txHash,
          status: TransactionStatus.NOT_FOUND,
        };
      }
    } catch (error) {
      logger.error('Error getting transaction status', {
        txHash,
        error: error instanceof Error ? error.message : 'Unknown error',
      });

      return {
        transactionHash: txHash,
        status: TransactionStatus.FAILED,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Estimate fees for a transaction
   */
  async estimateFees(request: FeeEstimateRequest): Promise<FeeEstimateResponse> {
    try {
      // Get the latest ledger info
      const ledgerInfo = await this.horizon.ledgers().order('desc').limit(1).call();
      const latestLedger = ledgerInfo.records[0];

      // Parse the transaction to get base fee
      const transaction = TransactionBuilder.fromXDR(
        request.xdr,
        this.networkPassphrase
      ) as Transaction;

      // Get fee stats from network
      const feeStats = await this.horizon.feeStats();

      // Calculate suggested fee based on network conditions
      const baseFee = parseInt(transaction.fee) / transaction.operations.length;
      const minResourceFee = '100'; // Minimum resource fee for Soroban
      const suggestedFee = feeStats.last_ledger_base_fee || baseFee.toString();

      return {
        baseFee: baseFee.toString(),
        minResourceFee,
        suggestedFee,
        networkPassphrase: this.networkPassphrase,
        latestLedger: latestLedger.sequence,
      };
    } catch (error) {
      logger.error('Error estimating fees', {
        error: error instanceof Error ? error.message : 'Unknown error',
      });
      throw error;
    }
  }

  /**
   * Get relayer wallet balance
   */
  async getRelayerBalance(): Promise<string> {
    try {
      if (!this.relayerAddress) {
        await this.initialize();
      }
      const account = await this.horizon.loadAccount(this.relayerAddress);
      const balance = account.balances.find(
        (b: { asset_type: string; balance: string }) => b.asset_type === 'native'
      );
      return balance ? balance.balance : '0';
    } catch (error) {
      logger.error('Error fetching relayer balance', {
        error: error instanceof Error ? error.message : 'Unknown error',
      });
      return '0';
    }
  }

  /**
   * Check if Horizon is connected
   */
  async isHorizonConnected(): Promise<boolean> {
    try {
      await this.horizon.ledgers().limit(1).call();
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Check if Defender Relayer is connected
   */
  async isDefenderConnected(): Promise<boolean> {
    try {
      const relayer = await this.client.relaySigner.getRelayer(this.relayerId);
      return relayer.active !== false;
    } catch {
      return false;
    }
  }

  /**
   * Get pending transaction count
   */
  getPendingCount(): number {
    return this.pendingTransactions.size;
  }

  /**
   * Get relayer public key
   */
  getRelayerPublicKey(): string {
    return this.relayerAddress;
  }

  /**
   * Get network info
   */
  getNetwork(): string {
    return this.network;
  }

  /**
   * Generate unique transaction ID
   */
  private generateTxId(): string {
    return `tx_${Date.now()}_${crypto.randomBytes(4).toString('hex')}`;
  }

  /**
   * Format error for response
   */
  private formatError(error: unknown): RelayError {
    if (error instanceof Error) {
      return {
        code: this.extractErrorCode(error.message),
        message: error.message,
      };
    }
    return {
      code: 'UNKNOWN_ERROR',
      message: 'An unknown error occurred',
    };
  }

  /**
   * Extract error code from error message
   */
  private extractErrorCode(message: string): string {
    const knownCodes = [
      'tx_bad_seq',
      'tx_bad_auth',
      'tx_insufficient_fee',
      'tx_insufficient_balance',
      'tx_too_late',
      'tx_too_early',
      'tx_bad_auth_extra',
      'tx_bad_operation',
      'tx_internal_error',
      'tx_failed',
      'RELAYER_ERROR',
      'DEFENDER_ERROR',
    ];

    const lowerMessage = message.toLowerCase();
    for (const code of knownCodes) {
      if (lowerMessage.includes(code)) {
        return code.toUpperCase();
      }
    }
    return 'SUBMISSION_ERROR';
  }

  /**
   * Delay helper
   */
  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}
