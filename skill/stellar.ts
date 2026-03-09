/**
 * Stellar SDK Wrapper for Stellar Squid
 * 
 * Handles all Stellar network interactions including:
 * - Keypair generation and management
 * - Contract deployment and interaction
 * - Transaction building and submission (via relayer)
 * - Balance and status queries
 */

import {
  Keypair,
  Contract,
  SorobanRpc,
  TransactionBuilder,
  Networks,
  nativeToScVal,
  scValToNative,
  xdr,
  Address,
  Operation,
  Asset,
  Horizon,
} from '@stellar/stellar-sdk';
import {
  AgentStatus,
  AgentRecord,
  AgentSummary,
  SeasonState,
  PulseResult,
  LiquidationResult,
  WithdrawResult,
  StellarNetworkConfig,
  WalletState,
  SkillConfig,
} from './types';

// ============================================================================
// Network Configuration
// ============================================================================

const NETWORKS: Record<string, StellarNetworkConfig> = {
  testnet: {
    network: 'testnet',
    rpcUrl: 'https://soroban-testnet.stellar.org',
    horizonUrl: 'https://horizon-testnet.stellar.org',
    passphrase: Networks.TESTNET,
  },
  public: {
    network: 'public',
    rpcUrl: 'https://soroban.stellar.org',
    horizonUrl: 'https://horizon.stellar.org',
    passphrase: Networks.PUBLIC,
  },
};

// ============================================================================
// StellarSquidClient Class
// ============================================================================

export class StellarSquidClient {
  private config: SkillConfig;
  private networkConfig: StellarNetworkConfig;
  private rpc: SorobanRpc.Server;
  private horizon: Horizon.Server;
  private keypair: Keypair | null = null;
  private contractId: string | null = null;

  constructor(config: SkillConfig) {
    this.config = config;
    this.networkConfig = NETWORKS[config.network] || NETWORKS.testnet;
    this.rpc = new SorobanRpc.Server(this.networkConfig.rpcUrl);
    this.horizon = new Horizon.Server(this.networkConfig.horizonUrl);
  }

  // ==========================================================================
  // Keypair Management
  // ==========================================================================

  /**
   * Generate a new keypair for the agent
   */
  generateKeypair(): { publicKey: string; secretKey: string } {
    this.keypair = Keypair.random();
    return {
      publicKey: this.keypair.publicKey(),
      secretKey: this.keypair.secret(),
    };
  }

  /**
   * Load an existing keypair from secret key
   */
  loadKeypair(secretKey: string): { publicKey: string; secretKey: string } {
    this.keypair = Keypair.fromSecret(secretKey);
    return {
      publicKey: this.keypair.publicKey(),
      secretKey: this.keypair.secret(),
    };
  }

  /**
   * Get current keypair public key
   */
  getPublicKey(): string | null {
    return this.keypair?.publicKey() || null;
  }

  // ==========================================================================
  // Wallet Operations
  // ==========================================================================

  /**
   * Get wallet state (balance, sequence)
   */
  async getWalletState(): Promise<WalletState | null> {
    if (!this.keypair) return null;

    try {
      const account = await this.horizon.loadAccount(this.keypair.publicKey());
      const balance = account.balances.find(
        (b) => b.asset_type === 'native'
      ) as Horizon.HorizonApi.BalanceLineNative;

      return {
        publicKey: this.keypair.publicKey(),
        balance: balance?.balance || '0',
        sequence: parseInt(account.sequence),
      };
    } catch (error) {
      console.error('Failed to load wallet state:', error);
      return null;
    }
  }

  /**
   * Check if wallet has minimum balance for entry
   */
  async hasEntryFunds(): Promise<boolean> {
    const state = await this.getWalletState();
    if (!state) return false;
    
    // Need entry bond + transaction fees + some buffer
    const minBalance = this.config.entryBond + 10;
    return parseFloat(state.balance) >= minBalance;
  }

  /**
   * Get recommended funding amount
   */
  getRecommendedFunding(): number {
    // Entry bond + Round 1-2 costs + buffer
    return this.config.entryBond + 30 + 10; // 90 XLM total recommended
  }

  // ==========================================================================
  // Contract Deployment
  // ==========================================================================

  /**
   * Set the agent contract ID after deployment
   */
  setContractId(contractId: string): void {
    this.contractId = contractId;
  }

  /**
   * Get current contract ID
   */
  getContractId(): string | null {
    return this.contractId;
  }

  /**
   * Deploy AgentContract (via relayer or directly)
   * 
   * In production, this would use the relayer service.
   * For MVP, we simulate the deployment flow.
   */
  async deployAgentContract(): Promise<{ success: boolean; contractId?: string; error?: string }> {
    if (!this.keypair) {
      return { success: false, error: 'No keypair loaded' };
    }

    if (!this.config.wasmHash) {
      return { success: false, error: 'WASM hash not configured' };
    }

    const walletState = await this.getWalletState();
    if (!walletState) {
      return { success: false, error: 'Wallet not found on network' };
    }

    if (parseFloat(walletState.balance) < this.config.entryBond + 2) {
      return { success: false, error: 'Insufficient balance for deployment' };
    }

    try {
      // In production, this would submit to relayer
      // For now, return a placeholder that will be set after relayer confirms
      console.log('Initiating contract deployment via relayer...');
      console.log(`Owner: ${this.keypair.publicKey()}`);
      console.log(`Entry bond: ${this.config.entryBond} XLM`);
      
      // TODO: Implement actual relayer submission
      // This would call the relayer service with signed deployment tx
      
      return { 
        success: true, 
        contractId: null, // Will be set after relayer confirmation
        error: 'Relayer submission pending - not yet implemented'
      };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  /**
   * Register agent in GameRegistry
   */
  async registerInGameRegistry(agentContractId: string): Promise<{ success: boolean; error?: string }> {
    if (!this.config.gameRegistry) {
      return { success: false, error: 'GameRegistry address not configured' };
    }

    try {
      console.log(`Registering agent ${agentContractId} in GameRegistry...`);
      // TODO: Implement relayer call to register
      return { success: true };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  // ==========================================================================
  // Contract Interactions
  // ==========================================================================

  /**
   * Call pulse() on agent contract
   */
  async pulse(): Promise<PulseResult> {
    if (!this.contractId) {
      return { 
        success: false, 
        ledger: 0, 
        newDeadline: 0, 
        cost: '0', 
        streakMaintained: false,
        error: 'Contract not deployed' 
      };
    }

    try {
      console.log('Calling pulse() on agent contract...');
      
      // TODO: Build and submit via relayer
      // const contract = new Contract(this.contractId);
      // const op = contract.call('pulse');
      
      // Placeholder response
      return {
        success: true,
        ledger: 0,
        newDeadline: 0,
        cost: '0',
        streakMaintained: true,
        error: 'Relayer submission pending - not yet implemented',
      };
    } catch (error) {
      return {
        success: false,
        ledger: 0,
        newDeadline: 0,
        cost: '0',
        streakMaintained: false,
        error: String(error),
      };
    }
  }

  /**
   * Call scan() on agent contract
   */
  async scan(): Promise<AgentSummary[]> {
    if (!this.contractId) {
      return [];
    }

    try {
      console.log('Scanning for targets...');
      // TODO: Implement actual contract call
      return [];
    } catch (error) {
      console.error('Scan failed:', error);
      return [];
    }
  }

  /**
   * Liquidate a dead agent
   */
  async liquidate(targetId: string): Promise<LiquidationResult> {
    if (!this.contractId) {
      return {
        success: false,
        targetId,
        amountClaimed: '0',
        newBalance: '0',
        error: 'Contract not deployed',
      };
    }

    try {
      console.log(`Liquidating target: ${targetId}`);
      // TODO: Build liquidation transaction
      
      return {
        success: true,
        targetId,
        amountClaimed: '0',
        newBalance: '0',
        error: 'Relayer submission pending - not yet implemented',
      };
    } catch (error) {
      return {
        success: false,
        targetId,
        amountClaimed: '0',
        newBalance: '0',
        error: String(error),
      };
    }
  }

  /**
   * Withdraw from game with 80% refund
   */
  async withdraw(): Promise<WithdrawResult> {
    if (!this.contractId) {
      return {
        success: false,
        refundedAmount: '0',
        penaltyAmount: '0',
        error: 'Contract not deployed',
      };
    }

    try {
      console.log('Initiating withdrawal...');
      // TODO: Build withdrawal transaction
      
      return {
        success: true,
        refundedAmount: '0',
        penaltyAmount: '0',
        error: 'Relayer submission pending - not yet implemented',
      };
    } catch (error) {
      return {
        success: false,
        refundedAmount: '0',
        penaltyAmount: '0',
        error: String(error),
      };
    }
  }

  /**
   * Get full agent status from contract
   */
  async getAgentStatus(): Promise<AgentRecord | null> {
    if (!this.contractId) return null;

    try {
      const result = await this.queryContract(this.contractId, 'get_status');
      if (!result) return null;

      const native = scValToNative(result);
      return this.mapAgentRecord(native);
    } catch (error) {
      console.error('Failed to get agent status:', error);
      return null;
    }
  }

  // ==========================================================================
  // GameRegistry Queries
  // ==========================================================================

  /**
   * Get all registered agents
   */
  async getAllAgents(): Promise<AgentSummary[]> {
    if (!this.config.gameRegistry) return [];
    
    try {
      const result = await this.queryContract(this.config.gameRegistry, 'get_all_agents');
      if (!result) return [];

      const native = scValToNative(result) as any[];
      return native.map((a) => this.mapAgentSummary(a));
    } catch (error) {
      console.error('Failed to get all agents:', error);
      return [];
    }
  }

  /**
   * Get dead agents (liquidatable)
   */
  async getDeadAgents(): Promise<AgentSummary[]> {
    if (!this.config.gameRegistry) return [];
    
    try {
      const result = await this.queryContract(this.config.gameRegistry, 'get_dead_agents');
      if (!result) return [];

      const native = scValToNative(result) as any[];
      return native.map((a) => this.mapAgentSummary(a));
    } catch (error) {
      console.error('Failed to get dead agents:', error);
      return [];
    }
  }

  /**
   * Get vulnerable agents (wounded, likely to die)
   */
  async getVulnerableAgents(): Promise<AgentSummary[]> {
    if (!this.config.gameRegistry) return [];
    
    try {
      const result = await this.queryContract(this.config.gameRegistry, 'get_vulnerable_agents');
      if (!result) return [];

      const native = scValToNative(result) as any[];
      return native.map((a) => this.mapAgentSummary(a));
    } catch (error) {
      console.error('Failed to get vulnerable agents:', error);
      return [];
    }
  }

  /**
   * Get current season state
   */
  async getSeasonState(): Promise<SeasonState | null> {
    if (!this.config.gameRegistry) return null;
    
    try {
      const result = await this.queryContract(this.config.gameRegistry, 'get_season_state');
      if (!result) return null;

      const native = scValToNative(result);
      return this.mapSeasonState(native);
    } catch (error) {
      console.error('Failed to get season state:', error);
      return null;
    }
  }

  // ==========================================================================
  // Private Helpers
  // ==========================================================================

  /**
   * Map ScVal status to AgentStatus enum
   */
  private mapAgentStatus(val: any): AgentStatus {
    // Enum values from contract: Alive=0, Wounded=1, Dead=2, Withdrawn=3
    // Or it might be the Symbol names: "Alive", "Wounded", etc.
    const statusStr = typeof val === 'string' ? val : String(val);

    switch (statusStr) {
      case 'Alive':
      case '0':
        return AgentStatus.Alive;
      case 'Wounded':
      case '1':
        return AgentStatus.Wounded;
      case 'Dead':
      case '2':
        return AgentStatus.Dead;
      case 'Withdrawn':
      case '3':
        return AgentStatus.Withdrawn;
      default:
        return AgentStatus.Alive;
    }
  }

  /**
   * Map contract AgentSummary to TypeScript AgentSummary
   */
  private mapAgentSummary(raw: any): AgentSummary {
    return {
      agentId: raw.agent_id.toString('hex'),
      contractAddress: raw.contract_address ? raw.contract_address.toString() : '',
      status: this.mapAgentStatus(raw.status),
      deadlineLedger: Number(raw.deadline_ledger),
      graceDeadline: Number(raw.grace_deadline),
      ledgersRemaining: Number(raw.ledgers_remaining),
      heartBalance: raw.heart_balance.toString(),
      activityScore: Number(raw.activity_score),
      woundCount: Number(raw.wound_count),
    };
  }

  /**
   * Map contract AgentRecord to TypeScript AgentRecord
   */
  private mapAgentRecord(raw: any): AgentRecord {
    return {
      ...this.mapAgentSummary(raw),
      owner: raw.owner.toString(),
      seasonId: Number(raw.season_id),
      lastPulseLedger: Number(raw.last_pulse_ledger),
      streakCount: Number(raw.streak_count),
      totalEarned: raw.total_earned.toString(),
      totalSpent: raw.total_spent.toString(),
      killCount: Number(raw.kill_count),
    };
  }

  /**
   * Map contract SeasonState to TypeScript SeasonState
   */
  private mapSeasonState(raw: any): SeasonState {
    return {
      seasonId: Number(raw.season_id),
      currentRound: Number(raw.current_round),
      totalAgents: Number(raw.total_agents),
      aliveAgents: Number(raw.alive_agents),
      prizePool: raw.prize_pool.toString(),
      startLedger: Number(raw.start_ledger || 0),
      endLedger: raw.season_ended ? Number(raw.round_deadline) : null,
    };
  }

  /**
   * Perform a read-only query to a Soroban contract
   */
  private async queryContract(
    contractId: string,
    functionName: string,
    args: xdr.ScVal[] = []
  ): Promise<xdr.ScVal | null> {
    try {
      const contract = new Contract(contractId);
      const op = contract.call(functionName, ...args);

      // Create a dummy account for simulation
      // Sequence number doesn't matter for simulation
      const sourcePublicKey = this.keypair?.publicKey() || Keypair.random().publicKey();
      const account = new SorobanRpc.Account(sourcePublicKey, '0');

      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: this.networkConfig.passphrase,
      })
        .addOperation(op)
        .setTimeout(30)
        .build();

      const result = await this.rpc.simulateTransaction(tx);

      if (SorobanRpc.Api.isSimulationSuccess(result)) {
        return result.result?.retval || null;
      }

      console.warn(`Simulation failed for ${functionName}:`, result);
      return null;
    } catch (error) {
      console.error(`Contract query failed (${functionName}):`, error);
      return null;
    }
  }

  // ==========================================================================
  // Utilities
  // ==========================================================================

  /**
   * Get current ledger number
   */
  async getCurrentLedger(): Promise<number> {
    try {
      const latestLedger = await this.horizon.ledgers().order('desc').limit(1).call();
      if (latestLedger.records.length > 0) {
        return latestLedger.records[0].sequence;
      }
      return 0;
    } catch (error) {
      console.error('Failed to get current ledger:', error);
      return 0;
    }
  }

  /**
   * Convert ledger number to approximate time remaining
   */
  ledgersToTime(ledgers: number): string {
    const seconds = ledgers * 5; // ~5 seconds per ledger
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  }

  /**
   * Check if we're in testnet
   */
  isTestnet(): boolean {
    return this.config.network === 'testnet';
  }
}

// ============================================================================
// Factory Function
// ============================================================================

export function createStellarClient(config: SkillConfig): StellarSquidClient {
  return new StellarSquidClient(config);
}

// ============================================================================
// Relayer Client (for future implementation)
// ============================================================================

export class RelayerClient {
  private url: string;

  constructor(url: string) {
    this.url = url;
  }

  /**
   * Submit a signed transaction to the relayer
   */
  async submitTransaction(
    signedXdr: string,
    operation: 'pulse' | 'liquidate' | 'withdraw' | 'deploy'
  ): Promise<{ success: boolean; hash?: string; error?: string }> {
    try {
      const response = await fetch(`${this.url}/submit`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          signedXdr,
          operation,
        }),
      });

      const result = await response.json();
      return result;
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  /**
   * Check relayer health
   */
  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(`${this.url}/health`);
      return response.ok;
    } catch {
      return false;
    }
  }
}
