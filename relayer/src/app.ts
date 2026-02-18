/**
 * Stellar Squid Relayer Service
 * Main Application Entry Point
 * Uses OpenZeppelin Defender Relayer for secure transaction submission
 */
import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import dotenv from 'dotenv';
import { DefenderService } from './services/defender';
import { RelayerService } from './services/relayer';
import { createRoutes } from './routes';
import RateLimiter from './middleware/rateLimit';
import { errorHandler, notFoundHandler } from './middleware/errorHandler';
import logger from './utils/logger';
import { DefenderRelayerConfig } from './types';

// Load environment variables
dotenv.config();

// Validate required environment variables
function validateConfig(): DefenderRelayerConfig {
  const required = ['DEFENDER_API_KEY', 'DEFENDER_API_SECRET', 'DEFENDER_RELAYER_ID'];
  const missing = required.filter((key) => !process.env[key]);

  if (missing.length > 0) {
    throw new Error(`Missing required environment variables: ${missing.join(', ')}`);
  }

  const network = (process.env.STELLAR_NETWORK || 'testnet') as 'testnet' | 'public' | 'futurenet';

  return {
    port: parseInt(process.env.PORT || '3000', 10),
    network,
    horizonUrl: process.env.STELLAR_HORIZON_URL || getDefaultHorizonUrl(network),
    defenderApiKey: process.env.DEFENDER_API_KEY!,
    defenderApiSecret: process.env.DEFENDER_API_SECRET!,
    defenderRelayerId: process.env.DEFENDER_RELAYER_ID!,
    protocolFeeAddress: process.env.PROTOCOL_FEE_ADDRESS || '',
    maxRetries: parseInt(process.env.MAX_RETRIES || '3', 10),
    retryDelayMs: parseInt(process.env.RETRY_DELAY_MS || '1000', 10),
    txTimeoutSeconds: parseInt(process.env.TX_TIMEOUT_SECONDS || '30', 10),
    rateLimitWindowMs: parseInt(process.env.RATE_LIMIT_WINDOW_MS || '60000', 10),
    rateLimitMaxRequests: parseInt(process.env.RATE_LIMIT_MAX_REQUESTS || '100', 10),
    logLevel: process.env.LOG_LEVEL || 'info',
  };
}

function getDefaultHorizonUrl(network: string): string {
  switch (network) {
    case 'public':
      return 'https://horizon.stellar.org';
    case 'testnet':
      return 'https://horizon-testnet.stellar.org';
    case 'futurenet':
      return 'https://horizon-futurenet.stellar.org';
    default:
      return 'https://horizon-testnet.stellar.org';
  }
}

async function startServer() {
  try {
    // Load and validate configuration
    const config = validateConfig();

    logger.info('Starting Stellar Squid Relayer with OpenZeppelin Defender', {
      network: config.network,
      port: config.port,
      horizon: config.horizonUrl,
      relayerId: config.defenderRelayerId,
    });

    // Initialize Defender service
    const defenderService = new DefenderService({
      apiKey: config.defenderApiKey,
      apiSecret: config.defenderApiSecret,
      relayerId: config.defenderRelayerId,
      network: config.network,
      horizonUrl: config.horizonUrl,
    });

    // Initialize Defender connection
    await defenderService.initialize();

    // Initialize relayer service
    const relayerService = new RelayerService(defenderService, config as any);

    // Create Express app
    const app = express();

    // Security middleware
    app.use(helmet());
    app.use(cors());

    // Body parsing
    app.use(express.json({ limit: '10mb' }));
    app.use(express.urlencoded({ extended: true }));

    // Rate limiting
    const rateLimiter = new RateLimiter(config.rateLimitWindowMs, config.rateLimitMaxRequests);
    app.use(rateLimiter.middleware.bind(rateLimiter));

    // API routes
    app.use('/api/v1', createRoutes(relayerService));

    // Health check at root
    app.get('/', async (_req, res) => {
      const health = await relayerService.getHealth();
      res.json({
        service: 'Stellar Squid Relayer',
        provider: 'OpenZeppelin Defender',
        version: process.env.npm_package_version || '1.0.0',
        status: health.status,
        network: config.network,
        relayerAddress: relayerService.getRelayerAddress(),
      });
    });

    // Error handling
    app.use(notFoundHandler);
    app.use(errorHandler);

    // Start server
    const server = app.listen(config.port, () => {
      logger.info(`Relayer server listening on port ${config.port}`, {
        relayerAddress: relayerService.getRelayerAddress(),
        network: config.network,
        provider: 'OpenZeppelin Defender',
      });
    });

    // Graceful shutdown
    process.on('SIGTERM', () => {
      logger.info('SIGTERM received, shutting down gracefully');
      server.close(() => {
        logger.info('Server closed');
        process.exit(0);
      });
    });

    process.on('SIGINT', () => {
      logger.info('SIGINT received, shutting down gracefully');
      server.close(() => {
        logger.info('Server closed');
        process.exit(0);
      });
    });

    return server;
  } catch (error) {
    logger.error('Failed to start server', {
      error: error instanceof Error ? error.message : 'Unknown error',
    });
    process.exit(1);
  }
}

// Start the server
startServer();
