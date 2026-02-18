/**
 * Rate Limiting Middleware
 * Simple in-memory rate limiting per IP
 */
import { Request, Response, NextFunction } from 'express';
import logger from '../utils/logger';

interface RateLimitEntry {
  count: number;
  resetTime: number;
}

class RateLimiter {
  private requests: Map<string, RateLimitEntry> = new Map();
  private windowMs: number;
  private maxRequests: number;

  constructor(windowMs: number = 60000, maxRequests: number = 100) {
    this.windowMs = windowMs;
    this.maxRequests = maxRequests;

    // Clean up expired entries every minute
    setInterval(() => this.cleanup(), 60000);
  }

  middleware(req: Request, res: Response, next: NextFunction): void {
    const clientId = this.getClientId(req);
    const now = Date.now();

    let entry = this.requests.get(clientId);

    if (!entry || now > entry.resetTime) {
      // New window
      entry = {
        count: 1,
        resetTime: now + this.windowMs,
      };
      this.requests.set(clientId, entry);
      next();
      return;
    }

    // Increment count
    entry.count++;

    if (entry.count > this.maxRequests) {
      logger.warn('Rate limit exceeded', {
        clientId,
        count: entry.count,
      });

      res.status(429).json({
        success: false,
        error: {
          code: 'RATE_LIMIT_EXCEEDED',
          message: 'Too many requests, please try again later',
          retryAfter: Math.ceil((entry.resetTime - now) / 1000),
        },
      });
      return;
    }

    // Set rate limit headers
    res.setHeader('X-RateLimit-Limit', this.maxRequests.toString());
    res.setHeader('X-RateLimit-Remaining', Math.max(0, this.maxRequests - entry.count).toString());
    res.setHeader('X-RateLimit-Reset', entry.resetTime.toString());

    next();
  }

  private getClientId(req: Request): string {
    // Use forwarded IP if behind proxy, otherwise use direct IP
    const forwarded = req.headers['x-forwarded-for'];
    const ip = forwarded
      ? (typeof forwarded === 'string' ? forwarded.split(',')[0].trim() : forwarded[0])
      : req.ip || req.socket.remoteAddress || 'unknown';
    return ip;
  }

  private cleanup(): void {
    const now = Date.now();
    for (const [key, entry] of this.requests) {
      if (now > entry.resetTime) {
        this.requests.delete(key);
      }
    }
  }
}

export default RateLimiter;
