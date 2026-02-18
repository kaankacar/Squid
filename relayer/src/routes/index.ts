/**
 * API Routes
 * Stellar Squid Relayer Service
 */
import { Router } from 'express';
import { RelayerService } from '../services/relayer';
import { asyncHandler } from '../middleware/errorHandler';
import { validate, relayValidationRules, estimateValidationRules } from '../middleware/validation';

export const createRoutes = (relayerService: RelayerService): Router => {
  const router = Router();

  /**
   * POST /relay
   * Submit a signed transaction to be relayed to the Stellar network
   */
  router.post(
    '/relay',
    validate(relayValidationRules),
    asyncHandler(async (req, res) => {
      const result = await relayerService.relay({
        signedXdr: req.body.signedXdr,
        metadata: req.body.metadata,
      });

      const statusCode = result.success ? 200 : result.error?.code?.includes('RATE') ? 429 : 400;

      res.status(statusCode).json(result);
    })
  );

  /**
   * GET /status/:txHash
   * Check the status of a transaction
   */
  router.get(
    '/status/:txHash',
    asyncHandler(async (req, res) => {
      const { txHash } = req.params;

      if (!txHash || typeof txHash !== 'string' || txHash.length < 32) {
        res.status(400).json({
          success: false,
          error: {
            code: 'INVALID_HASH',
            message: 'Invalid transaction hash',
          },
        });
        return;
      }

      const status = await relayerService.getStatus(txHash);

      res.json({
        success: true,
        data: status,
      });
    })
  );

  /**
   * POST /estimate
   * Get fee estimate for a transaction
   */
  router.post(
    '/estimate',
    validate(estimateValidationRules),
    asyncHandler(async (req, res) => {
      const estimate = await relayerService.estimateFees({
        xdr: req.body.xdr,
      });

      res.json({
        success: true,
        data: estimate,
      });
    })
  );

  /**
   * GET /health
   * Health check endpoint
   */
  router.get(
    '/health',
    asyncHandler(async (_req, res) => {
      const health = await relayerService.getHealth();

      const statusCode = health.status === 'healthy' ? 200 : health.status === 'degraded' ? 200 : 503;

      res.status(statusCode).json({
        success: true,
        data: health,
      });
    })
  );

  /**
   * GET /info
   * Get relayer information
   */
  router.get(
    '/info',
    asyncHandler(async (_req, res) => {
      res.json({
        success: true,
        data: {
          name: 'Stellar Squid Relayer',
          version: process.env.npm_package_version || '1.0.0',
          relayerAddress: relayerService.getRelayerAddress(),
          network: process.env.STELLAR_NETWORK || 'testnet',
          supportedOperations: ['pulse', 'scan', 'liquidate', 'withdraw', 'claim_prize'],
        },
      });
    })
  );

  return router;
};
