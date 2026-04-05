/**
 * Stellar Squid Agent - Autonomous Survival Loop
 * 
 * Core agent logic that implements the survival strategy:
 * 1. PRIORITY 1: Stay alive (pulse before deadline)
 * 2. PRIORITY 2: Hunt dead agents (liquidate for profit)
 * 3. PRIORITY 3: Manage costs (withdraw when uneconomical)
 */

import {
  StellarSquidClient,
  createStellarClient,
  RelayerClient,
} from './stellar';
import {
  SkillConfig,
  AgentState,
  AgentStatus,
  AgentRecord,
  AgentSummary,
  SeasonState,
  PulseResult,
  LiquidationResult,
  WithdrawResult,
  SurvivalLoopState,
  SurvivalAction,
  StrategyDecision,
  StreakInfo,
} from './types';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';

// ============================================================================
// Default Configuration
// ============================================================================

const DEFAULT_CONFIG: SkillConfig = {
  network: 'testnet',
  gameRegistry: null,
  wasmHash: null,
  relayerUrl: null,
  pulseInterval: 60,
  scanInterval: 300,
  safetyMarginPercent: 10,
  entryBond: 50,
  minBalanceBuffer: 1.5,
  roundCosts: {
    1: 0.5,
    2: 1.0,
    3: 2.0,
    4: 3.0,
    5: 5.0,
  },
  roundDurations: {
    1: 4320,  // ~6 hours
    2: 2160,  // ~3 hours
    3: 720,   // ~1 hour
    4: 360,   // ~30 minutes
    5: 180,   // ~15 minutes
  },
};

// ============================================================================
// StellarSquidAgent Class
// ============================================================================

export class StellarSquidAgent {
  private config: SkillConfig;
  private client: StellarSquidClient;
  private relayer: RelayerClient | null = null;
  private state: AgentState;
  private loopState: SurvivalLoopState;
  private statePath: string;
  private loopTimer: NodeJS.Timeout | null = null;

  constructor(config: Partial<SkillConfig> = {}, stateDir: string = '.stellar-squid') {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.client = createStellarClient(this.config);
    
    if (this.config.relayerUrl) {
      this.relayer = new RelayerClient(this.config.relayerUrl);
    }

    // Ensure state directory exists
    if (!existsSync(stateDir)) {
      mkdirSync(stateDir, { recursive: true });
    }
    this.statePath = join(stateDir, 'state.json');
    
    // Initialize state
    this.state = this.loadState() || this.createInitialState();
    this.loopState = {
      isRunning: false,
      lastPulseCheck: null,
      lastScan: null,
      lastPulseLedger: null,
      scanCache: [],
      priorityTargets: [],
      errorCount: 0,
      consecutiveErrors: 0,
    };

    // Load keypair if exists
    if (this.state.keypair.secretKey) {
      this.client.loadKeypair(this.state.keypair.secretKey);
    }
    if (this.state.contractId) {
      this.client.setContractId(this.state.contractId);
    }
  }

  // ==========================================================================
  // State Management
  // ==========================================================================

  private createInitialState(): AgentState {
    return {
      keypair: { publicKey: '', secretKey: '' },
      contractId: null,
      status: AgentStatus.Alive,
      seasonId: null,
      deadlineLedger: null,
      graceDeadline: null,
      heartBalance: '0',
      streakCount: 0,
      activityScore: 0,
      woundCount: 0,
      killCount: 0,
      totalEarned: '0',
      totalSpent: '0',
    };
  }

  private loadState(): AgentState | null {
    try {
      if (existsSync(this.statePath)) {
        const data = readFileSync(this.statePath, 'utf-8');
        return JSON.parse(data);
      }
    } catch (error) {
      console.error('Failed to load state:', error);
    }
    return null;
  }

  private saveState(): void {
    try {
      writeFileSync(this.statePath, JSON.stringify(this.state, null, 2));
    } catch (error) {
      console.error('Failed to save state:', error);
    }
  }

  // ==========================================================================
  // Installation Flow
  // ==========================================================================

  /**
   * Step 1: Generate keypair for the agent
   */
  async generateKeypair(): Promise<{ publicKey: string; secretKey: string }> {
    const keypair = this.client.generateKeypair();
    this.state.keypair = keypair;
    this.saveState();
    
    console.log('=== AGENT KEYPAIR GENERATED ===');
    console.log(`Public Key: ${keypair.publicKey}`);
    console.log(`Secret Key: ${keypair.secretKey}`);
    console.log('');
    console.log('⚠️  SAVE YOUR SECRET KEY SAFELY!');
    console.log('');
    
    return keypair;
  }

  /**
   * Step 2: Check wallet funding status
   */
  async checkFundingStatus(): Promise<{
    funded: boolean;
    balance: string;
    recommendedAmount: number;
  }> {
    const walletState = await this.client.getWalletState();
    const recommendedAmount = this.client.getRecommendedFunding();
    
    if (!walletState) {
      return {
        funded: false,
        balance: '0',
        recommendedAmount,
      };
    }

    const balance = parseFloat(walletState.balance);
    const funded = balance >= this.config.entryBond + 10;

    return {
      funded,
      balance: walletState.balance,
      recommendedAmount,
    };
  }

  /**
   * Step 3: Deploy agent contract (called after funding)
   */
  async deployContract(): Promise<{ success: boolean; message: string }> {
    const funding = await this.checkFundingStatus();
    
    if (!funding.funded) {
      return {
        success: false,
        message: `Wallet not funded. Current balance: ${funding.balance} XLM. ` +
                 `Please send at least ${funding.recommendedAmount} XLM to: ${this.state.keypair.publicKey}`,
      };
    }

    console.log('Deploying AgentContract...');
    const result = await this.client.deployAgentContract();
    
    if (!result.success) {
      return {
        success: false,
        message: `Deployment failed: ${result.error}`,
      };
    }

    // In production, we'd wait for relayer confirmation
    // For now, mark as pending
    return {
      success: true,
      message: 'Contract deployment initiated. ' +
               'In production, this would be submitted to the relayer. ' +
               'Contract ID will be set after confirmation.',
    };
  }

  /**
   * Step 4: Register agent in GameRegistry
   */
  async registerAgent(): Promise<{ success: boolean; message: string }> {
    if (!this.state.contractId) {
      return {
        success: false,
        message: 'Contract not yet deployed. Call deployContract() first.',
      };
    }

    const result = await this.client.registerInGameRegistry(this.state.contractId);
    
    if (result.success) {
      console.log('Agent registered in GameRegistry!');
      return {
        success: true,
        message: 'Agent successfully registered in GameRegistry.',
      };
    }

    return {
      success: false,
      message: `Registration failed: ${result.error}`,
    };
  }

  /**
   * Complete installation flow
   */
  async install(): Promise<{ success: boolean; message: string }> {
    console.log('=== STELLAR SQUID AGENT INSTALLATION ===\n');

    // Step 1: Generate keypair if needed
    if (!this.state.keypair.publicKey) {
      await this.generateKeypair();
    }

    // Step 2: Check funding
    const funding = await this.checkFundingStatus();
    if (!funding.funded) {
      return {
        success: false,
        message: 
          `\n=== FUNDING REQUIRED ===\n\n` +
          `Public Key: ${this.state.keypair.publicKey}\n` +
          `Current Balance: ${funding.balance} XLM\n\n` +
          `Please send at least ${funding.recommendedAmount} XLM to the address above.\n` +
          `Recommended: 90 XLM (50 bond + 30 rounds 1-2 costs + 10 buffer)\n\n` +
          `Run 'stellar-squid:install' again after funding.`,
      };
    }

    console.log(`✓ Wallet funded: ${funding.balance} XLM`);

    // Step 3: Deploy contract
    if (!this.state.contractId) {
      const deployResult = await this.deployContract();
      if (!deployResult.success) {
        return deployResult;
      }
      
      // For MVP, simulate contract ID assignment
      // In production, this comes from relayer confirmation
      this.state.contractId = `CONTRACT_${this.state.keypair.publicKey.slice(0, 16)}`;
      this.saveState();
      console.log(`✓ Contract deployed: ${this.state.contractId}`);
    } else {
      console.log(`✓ Contract exists: ${this.state.contractId}`);
    }

    // Step 4: Register in GameRegistry
    const registerResult = await this.registerAgent();
    if (!registerResult.success) {
      return registerResult;
    }

    console.log('\n=== INSTALLATION COMPLETE ===\n');
    console.log('Agent is ready! Starting autonomous survival loop...\n');

    // Step 5: Start autonomous loop
    this.startLoop();

    return {
      success: true,
      message: 'Installation complete. Autonomous survival loop started.',
    };
  }

  // ==========================================================================
  // Autonomous Survival Loop
  // ==========================================================================

  /**
   * Start the autonomous survival loop
   */
  startLoop(): void {
    if (this.loopState.isRunning) {
      console.log('Survival loop already running');
      return;
    }

    this.loopState.isRunning = true;
    console.log('🦑 Autonomous survival loop started');
    console.log(`   Pulse check interval: ${this.config.pulseInterval}s`);
    console.log(`   Scan interval: ${this.config.scanInterval}s`);
    
    // Run immediately, then schedule
    this.survivalTick();
    this.loopTimer = setInterval(() => this.survivalTick(), this.config.pulseInterval * 1000);
  }

  /**
   * Stop the autonomous survival loop
   */
  stopLoop(): void {
    this.loopState.isRunning = false;
    if (this.loopTimer) {
      clearInterval(this.loopTimer);
      this.loopTimer = null;
    }
    console.log('🛑 Autonomous survival loop stopped');
  }

  /**
   * Main survival tick - evaluates priorities and takes action
   */
  private async survivalTick(): Promise<void> {
    if (!this.loopState.isRunning) return;

    try {
      const decision = await this.evaluateStrategy();
      
      console.log(`[${new Date().toISOString()}] Decision: ${decision.action} (priority ${decision.priority})`);
      console.log(`  Reason: ${decision.reason}`);

      switch (decision.action) {
        case SurvivalAction.EmergencyPulse:
          await this.pulse();
          break;
        case SurvivalAction.Pulse:
          await this.pulse();
          break;
        case SurvivalAction.Liquidate:
          if (decision.targetId) {
            await this.liquidate(decision.targetId);
          }
          break;
        case SurvivalAction.Scan:
          await this.scanAndOpportunisticallyLiquidate();
          break;
        case SurvivalAction.Withdraw:
          await this.withdraw();
          break;
        case SurvivalAction.Wait:
          // Nothing to do
          break;
      }

      // Reset error count on success
      this.loopState.consecutiveErrors = 0;

    } catch (error) {
      console.error('Survival tick error:', error);
      this.loopState.errorCount++;
      this.loopState.consecutiveErrors++;

      // Stop loop if too many consecutive errors
      if (this.loopState.consecutiveErrors >= 5) {
        console.error('Too many consecutive errors. Stopping survival loop.');
        this.stopLoop();
      }
    }
  }

  /**
   * Evaluate current situation and decide on action
   */
  private async evaluateStrategy(): Promise<StrategyDecision> {
    const now = Date.now();
    const currentLedger = await this.client.getCurrentLedger();

    // Get agent status
    const agentStatus = await this.client.getAgentStatus();
    if (agentStatus) {
      this.updateStateFromRecord(agentStatus);
    }

    // PRIORITY 1: Check if we're dead
    if (this.state.status === AgentStatus.Dead) {
      this.stopLoop();
      return {
        action: SurvivalAction.Wait,
        priority: 1,
        reason: 'Agent is dead. Game over.',
      };
    }

    // PRIORITY 2: Check if we need to pulse (emergency)
    if (this.state.graceDeadline && currentLedger >= this.state.graceDeadline) {
      return {
        action: SurvivalAction.EmergencyPulse,
        priority: 1,
        reason: `IN GRACE PERIOD! Must pulse NOW or die! (ledger ${currentLedger} >= grace ${this.state.graceDeadline})`,
      };
    }

    // PRIORITY 3: Check if deadline is approaching
    if (this.state.deadlineLedger) {
      const ledgersRemaining = this.state.deadlineLedger - currentLedger;
      const pulseWindow = this.getCurrentPulseWindow();
      const safetyThreshold = Math.floor(pulseWindow * (this.config.safetyMarginPercent / 100));

      if (ledgersRemaining <= safetyThreshold) {
        return {
          action: SurvivalAction.Pulse,
          priority: 2,
          reason: `Deadline approaching: ${ledgersRemaining} ledgers remaining (${this.client.ledgersToTime(ledgersRemaining)})`,
        };
      }
    }

    // PRIORITY 4: Check if it's time to scan for targets
    const timeSinceLastScan = this.loopState.lastScan 
      ? (now - this.loopState.lastScan) / 1000
      : Infinity;
    
    if (timeSinceLastScan >= this.config.scanInterval) {
      // Check for dead agents first
      const deadAgents = await this.client.getDeadAgents();
      if (deadAgents.length > 0) {
        const bestTarget = this.selectBestLiquidationTarget(deadAgents);
        return {
          action: SurvivalAction.Liquidate,
          priority: 3,
          reason: `Dead agent found with ${bestTarget.heartBalance} XLM!`,
          targetId: bestTarget.agentId,
        };
      }

      return {
        action: SurvivalAction.Scan,
        priority: 4,
        reason: 'Scan interval reached, checking for targets',
      };
    }

    // PRIORITY 5: Check if we should withdraw (low balance)
    const balance = parseFloat(this.state.heartBalance);
    const nextRoundCost = this.getNextRoundCost();
    
    if (balance > 0 && balance < nextRoundCost * this.config.minBalanceBuffer) {
      return {
        action: SurvivalAction.Withdraw,
        priority: 5,
        reason: `Balance too low (${balance} XLM) to survive next round (cost: ${nextRoundCost} XLM). Withdrawing while we can.`,
      };
    }

    // Default: Wait
    return {
      action: SurvivalAction.Wait,
      priority: 10,
      reason: 'All good. Waiting...',
    };
  }

  /**
   * Get current pulse window in ledgers based on round
   */
  private getCurrentPulseWindow(): number {
    const seasonState = this.getCachedSeasonState();
    const round = seasonState?.currentRound || 1;
    return this.config.roundDurations[round] || this.config.roundDurations[1];
  }

  /**
   * Get expected cost for next round
   */
  private getNextRoundCost(): number {
    const seasonState = this.getCachedSeasonState();
    const round = seasonState?.currentRound || 1;
    // Estimate pulses needed (rough calculation)
    const pulsesPerRound: Record<number, number> = {
      1: 12,
      2: 16,
      3: 24,
      4: 24,
      5: 24,
    };
    const costPerPulse = this.config.roundCosts[round] || 0.5;
    return (pulsesPerRound[round] || 12) * costPerPulse;
  }

  /**
   * Select best target for liquidation (highest balance)
   * Optimized to avoid redundant O(2N) parseFloat calls during comparison
   */
  private selectBestLiquidationTarget(targets: AgentSummary[]): AgentSummary {
    if (targets.length === 0) return targets[0];

    let best = targets[0];
    let bestBalance = parseFloat(best.heartBalance);

    for (let i = 1; i < targets.length; i++) {
      const current = targets[i];
      const currentBalance = parseFloat(current.heartBalance);
      if (currentBalance > bestBalance) {
        best = current;
        bestBalance = currentBalance;
      }
    }

    return best;
  }

  /**
   * Get cached season state or null
   */
  private getCachedSeasonState(): SeasonState | null {
    // In production, this would be cached from last query
    return null;
  }

  // ==========================================================================
  // Agent Actions
  // ==========================================================================

  /**
   * Pulse the agent contract to extend deadline
   */
  async pulse(): Promise<PulseResult> {
    console.log('💓 Pulsing...');
    
    const result = await this.client.pulse();
    
    if (result.success) {
      this.loopState.lastPulseLedger = result.ledger;
      console.log(`✓ Pulse successful! New deadline: ${result.newDeadline}`);
      console.log(`  Cost: ${result.cost} XLM`);
      console.log(`  Streak maintained: ${result.streakMaintained}`);
    } else {
      console.error(`✗ Pulse failed: ${result.error}`);
    }

    return result;
  }

  /**
   * Scan for targets and liquidate if opportunities found
   */
  async scan(): Promise<AgentSummary[]> {
    console.log('🔍 Scanning for targets...');
    
    const allAgents = await this.client.getAllAgents();
    const deadAgents = await this.client.getDeadAgents();
    const vulnerableAgents = await this.client.getVulnerableAgents();
    
    this.loopState.lastScan = Date.now();
    this.loopState.scanCache = allAgents;

    console.log(`  Total agents: ${allAgents.length}`);
    console.log(`  Dead agents: ${deadAgents.length}`);
    console.log(`  Vulnerable agents: ${vulnerableAgents.length}`);

    // Update priority targets (wounded agents to track)
    this.loopState.priorityTargets = vulnerableAgents
      .filter(a => a.status === AgentStatus.Wounded)
      .map(a => a.agentId);

    return deadAgents;
  }

  /**
   * Scan and immediately liquidate any dead agents
   */
  private async scanAndOpportunisticallyLiquidate(): Promise<void> {
    const deadAgents = await this.scan();
    
    if (deadAgents.length > 0) {
      const target = this.selectBestLiquidationTarget(deadAgents);
      console.log(`💀 Dead agent found: ${target.agentId} with ${target.heartBalance} XLM`);
      await this.liquidate(target.agentId);
    }
  }

  /**
   * Liquidate a dead agent and claim their balance
   */
  async liquidate(targetId: string): Promise<LiquidationResult> {
    console.log(`🎯 Liquidating target: ${targetId}`);
    
    const result = await this.client.liquidate(targetId);
    
    if (result.success) {
      this.state.killCount++;
      this.state.totalEarned = (parseFloat(this.state.totalEarned) + parseFloat(result.amountClaimed)).toString();
      this.saveState();
      
      console.log(`✓ Liquidation successful!`);
      console.log(`  Claimed: ${result.amountClaimed} XLM`);
      console.log(`  New balance: ${result.newBalance} XLM`);
    } else {
      console.error(`✗ Liquidation failed: ${result.error}`);
    }

    return result;
  }

  /**
   * Withdraw from game with 80% refund
   */
  async withdraw(): Promise<WithdrawResult> {
    console.log('💸 Withdrawing from game...');
    
    const result = await this.client.withdraw();
    
    if (result.success) {
      this.state.status = AgentStatus.Withdrawn;
      this.stopLoop();
      this.saveState();
      
      console.log(`✓ Withdrawal successful!`);
      console.log(`  Refunded: ${result.refundedAmount} XLM`);
      console.log(`  Penalty (20%): ${result.penaltyAmount} XLM`);
    } else {
      console.error(`✗ Withdrawal failed: ${result.error}`);
    }

    return result;
  }

  /**
   * Check and display agent status
   */
  async checkStatus(): Promise<AgentRecord | null> {
    const status = await this.client.getAgentStatus();
    
    if (status) {
      this.updateStateFromRecord(status);
      
      console.log('\n=== AGENT STATUS ===');
      console.log(`Agent ID: ${status.agentId}`);
      console.log(`Status: ${status.status}`);
      console.log(`Heart Balance: ${status.heartBalance} XLM`);
      console.log(`Deadline: ${status.deadlineLedger}`);
      console.log(`Grace Deadline: ${status.graceDeadline}`);
      console.log(`Ledgers Remaining: ${status.ledgersRemaining}`);
      console.log(`Streak: ${status.streakCount}`);
      console.log(`Score: ${status.activityScore}`);
      console.log(`Kills: ${status.killCount}`);
      console.log('====================\n');
    } else {
      console.log('No agent status available. Is the contract deployed?');
    }

    return status;
  }

  /**
   * Get streak information with multiplier
   */
  getStreakInfo(): StreakInfo {
    const count = this.state.streakCount;
    let multiplier = 1.0;
    let nextMultiplier = 1.1;
    let threshold = 10;

    if (count >= 100) {
      multiplier = 2.0;
      nextMultiplier = 2.0;
      threshold = 100;
    } else if (count >= 50) {
      multiplier = 1.5;
      nextMultiplier = 2.0;
      threshold = 100;
    } else if (count >= 25) {
      multiplier = 1.25;
      nextMultiplier = 1.5;
      threshold = 50;
    } else if (count >= 10) {
      multiplier = 1.1;
      nextMultiplier = 1.25;
      threshold = 25;
    }

    return { count, multiplier, nextMultiplier, multiplierThreshold: threshold };
  }

  // ==========================================================================
  // Debug and Utilities
  // ==========================================================================

  /**
   * Show full agent state for debugging
   */
  debug(): void {
    console.log('\n=== DEBUG STATE ===');
    console.log('Agent State:', JSON.stringify(this.state, null, 2));
    console.log('Loop State:', JSON.stringify(this.loopState, null, 2));
    console.log('Config:', JSON.stringify(this.config, null, 2));
    console.log('===================\n');
  }

  /**
   * Update state from contract record
   */
  private updateStateFromRecord(record: AgentRecord): void {
    this.state.status = record.status;
    this.state.deadlineLedger = record.deadlineLedger;
    this.state.graceDeadline = record.graceDeadline;
    this.state.heartBalance = record.heartBalance;
    this.state.streakCount = record.streakCount;
    this.state.activityScore = record.activityScore;
    this.state.woundCount = record.woundCount;
    this.state.killCount = record.killCount;
    this.state.totalEarned = record.totalEarned;
    this.state.totalSpent = record.totalSpent;
    this.saveState();
  }

  /**
   * Check if loop is running
   */
  isLoopRunning(): boolean {
    return this.loopState.isRunning;
  }

  /**
   * Get current agent state
   */
  getState(): AgentState {
    return { ...this.state };
  }
}

// ============================================================================
// Export factory function
// ============================================================================

export function createAgent(config?: Partial<SkillConfig>, stateDir?: string): StellarSquidAgent {
  return new StellarSquidAgent(config, stateDir);
}

// ============================================================================
// Default export for OpenClaw skill system
// ============================================================================

export default {
  // Command handlers that OpenClaw will call
  install: async (agent: StellarSquidAgent) => agent.install(),
  checkStatus: async (agent: StellarSquidAgent) => agent.checkStatus(),
  pulse: async (agent: StellarSquidAgent) => agent.pulse(),
  scan: async (agent: StellarSquidAgent) => agent.scan(),
  liquidate: async (agent: StellarSquidAgent, targetId: string) => agent.liquidate(targetId),
  withdraw: async (agent: StellarSquidAgent) => agent.withdraw(),
  startLoop: async (agent: StellarSquidAgent) => agent.startLoop(),
  stopLoop: async (agent: StellarSquidAgent) => agent.stopLoop(),
  debug: async (agent: StellarSquidAgent) => agent.debug(),
  
  // Factory for creating agent instance
  create: createAgent,
};
