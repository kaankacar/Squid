/**
 * Validation Middleware
 */
import { Request, Response, NextFunction } from 'express';

export interface ValidationRule {
  field: string;
  required?: boolean;
  type?: 'string' | 'number' | 'object' | 'array';
  minLength?: number;
  maxLength?: number;
  pattern?: RegExp;
  custom?: (value: unknown) => boolean | string;
}

export const validate = (rules: ValidationRule[]) => {
  return (req: Request, res: Response, next: NextFunction): void => {
    const errors: { field: string; message: string }[] = [];

    for (const rule of rules) {
      const value = req.body[rule.field];

      // Check required
      if (rule.required && (value === undefined || value === null || value === '')) {
        errors.push({
          field: rule.field,
          message: `${rule.field} is required`,
        });
        continue;
      }

      // Skip further validation if not required and value is missing
      if (!rule.required && (value === undefined || value === null)) {
        continue;
      }

      // Check type
      if (rule.type && typeof value !== rule.type) {
        errors.push({
          field: rule.field,
          message: `${rule.field} must be of type ${rule.type}`,
        });
      }

      // Check string length
      if (typeof value === 'string') {
        if (rule.minLength !== undefined && value.length < rule.minLength) {
          errors.push({
            field: rule.field,
            message: `${rule.field} must be at least ${rule.minLength} characters`,
          });
        }
        if (rule.maxLength !== undefined && value.length > rule.maxLength) {
          errors.push({
            field: rule.field,
            message: `${rule.field} must be at most ${rule.maxLength} characters`,
          });
        }
        if (rule.pattern && !rule.pattern.test(value)) {
          errors.push({
            field: rule.field,
            message: `${rule.field} format is invalid`,
          });
        }
      }

      // Custom validation
      if (rule.custom) {
        const result = rule.custom(value);
        if (result !== true) {
          errors.push({
            field: rule.field,
            message: typeof result === 'string' ? result : `${rule.field} is invalid`,
          });
        }
      }
    }

    if (errors.length > 0) {
      res.status(400).json({
        success: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Request validation failed',
          details: errors,
        },
      });
      return;
    }

    next();
  };
};

// Validation rules for relay request
export const relayValidationRules: ValidationRule[] = [
  {
    field: 'signedXdr',
    required: true,
    type: 'string',
    minLength: 10,
    custom: (value) => {
      // Check if valid base64
      try {
        Buffer.from(value as string, 'base64').toString('base64') === value;
        return true;
      } catch {
        return 'signedXdr must be valid base64';
      }
    },
  },
];

// Validation rules for fee estimate
export const estimateValidationRules: ValidationRule[] = [
  {
    field: 'xdr',
    required: true,
    type: 'string',
    minLength: 10,
    custom: (value) => {
      try {
        Buffer.from(value as string, 'base64').toString('base64') === value;
        return true;
      } catch {
        return 'xdr must be valid base64';
      }
    },
  },
];
