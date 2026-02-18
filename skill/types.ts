/**
 * TypeScript type definitions for Stellar Squid skill
 */

import { Keypair, Contract, Transaction } from '@stellar/stellar-sdk';

// ============================================================================
// Agent Status
// ============================================================================

export enum AgentStatus {
  Alive = 'Alive',
  Wounded = 'Wounded',
  Dead = 'Dead',
  Withdrawn = 'Withdrawn',
}

// ============================================================================
// Game State
// ============================================================================

export interface SeasonState {
  seasonId: number;
  currentRound: number;
  totalAgents: number;
  aliveAgents: number;
  prizePool: string; // XLM as string to preserve precision
  startLedger: number;
  endLedger: number | null;
}

export interface RoundConfig {
  durationLedgers: number;
  pulseWindowLedgers: number;
  graceWindowLedgers: number;
  costXLM: number;
}

// ============================================================================
// Agent Data
// ============================================================================

export interface AgentSummary {
  agentId: string;
  contractAddress: string;
  status: AgentStatus;
  deadlineLedger: number;
  graceDeadline: number;
  ledgersRemaining: number;
  heartBalance: string; // XLM as string
  activityScore: number;
  woundCount: number;
}

export interface AgentRecord extends AgentSummary {
  owner: string;
  seasonId: number;
  lastPulseLedger: number;
  streakCount: number;
  totalEarned: string;
  totalSpent: string;
  killCount: number;
}

export interface AgentState {
  keypair: {
    publicKey: string;
    secretKey: string; // Encrypted in storage
  };
  contractId: string | null;
  status: AgentStatus;
  seasonId: number | null;
  deadlineLedger: number | null;
  graceDeadline: number | null;
  heartBalance: string;
  streakCount: number;
  activityScore: number;
  woundCount: number;
  killCount: number;
  totalEarned: string;
  totalSpent: string;
}

// ============================================================================
// Skill Config
// ============================================================================

export interface SkillConfig {
  network: 'testnet' | 'public';
  gameRegistry: string | null;
  wasmHash: string | null;
  relayerUrl: string | null;
  pulseInterval: number;
  scanInterval: number;
  safetyMarginPercent: number;
  entryBond: number;
  minBalanceBuffer: number;
  roundCosts: Record<number, number>;
  roundDurations: Record<number, number>;
}

// ============================================================================
// Transaction Results
// ============================================================================

export interface PulseResult {
  success: boolean;
  ledger: number;
  newDeadline: number;
  cost: string;
  streakMaintained: boolean;
  error?: string;
}

export interface LiquidationResult {
  success: boolean;
  targetId: string;
  amountClaimed: string;
  newBalance: string;
  error?: string;
}

export interface WithdrawResult {
  success: boolean;
  refundedAmount: string;
  penaltyAmount: string;
  error?: string;
}

export interface ScanResult {
  deadAgents: AgentSummary[];
  woundedAgents: AgentSummary[];
  vulnerableAgents: AgentSummary[]; // Low balance, likely to die
}

// ============================================================================
// Survival Loop State
// ============================================================================

export interface SurvivalLoopState {
  isRunning: boolean;
  lastPulseCheck: number | null;
  lastScan: number | null;
  lastPulseLedger: number | null;
  scanCache: AgentSummary[];
  priorityTargets: string[]; // Agents being tracked for liquidation
  errorCount: number;
  consecutiveErrors: number;
}

// ============================================================================
// Strategy Decision Types
// ============================================================================

export enum SurvivalAction {
  Pulse = 'pulse',
  EmergencyPulse = 'emergency_pulse',
  Liquidate = 'liquidate',
  Scan = 'scan',
  Withdraw = 'withdraw',
  Wait = 'wait',
}

export interface StrategyDecision {
  action: SurvivalAction;
  priority: number; // 1 = highest
  reason: string;
  targetId?: string;
  deadline?: number;
}

// ============================================================================
// Contract Interface
// ============================================================================

export interface AgentContractInterface {
  pulse(): Promise<PulseResult>;
  scan(): Promise<AgentSummary[]>;
  liquidate(targetId: string): Promise<LiquidationResult>;
  withdraw(): Promise<WithdrawResult>;
  getStatus(): Promise<AgentRecord>;
}

export interface GameRegistryInterface {
  register(agentContract: string): Promise<boolean>;
  getAllAgents(): Promise<AgentSummary[]>;
  getDeadAgents(): Promise<AgentSummary[]>;
  getVulnerableAgents(): Promise<AgentSummary[]>;
  getAgentDetail(agentId: string): Promise<AgentRecord>;
  getSeasonState(): Promise<SeasonState>;
}

// ============================================================================
// Stellar Network Types
// ============================================================================

export interface StellarNetworkConfig {
  network: 'testnet' | 'public';
  rpcUrl: string;
  horizonUrl: string;
  passphrase: string;
}

export interface WalletState {
  publicKey: string;
  balance: string;
  sequence: number;
}

// ============================================================================
// Event Types
// ============================================================================

export interface AgentEvent {
  type: 'pulse' | 'liquidation' | 'wound' | 'death' | 'withdraw' | 'error';
  timestamp: number;
  ledger: number;
  details: Record<string, unknown>;
}

export interface StreakInfo {
  count: number;
  multiplier: number;
  nextMultiplier: number;
  multiplierThreshold: number;
}
