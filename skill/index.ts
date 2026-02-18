/**
 * Stellar Squid Skill - Main Entry Point
 * 
 * This is the main entry point for the OpenClaw skill system.
 * All exports are available for skill integration.
 */

// Core exports
export { StellarSquidAgent, createAgent } from './agent';
export { StellarSquidClient, createStellarClient, RelayerClient } from './stellar';

// Type exports
export * from './types';

// Skill integration for OpenClaw
import { StellarSquidAgent } from './agent';
import { SkillConfig } from './types';

// Singleton agent instance (managed by OpenClaw)
let agentInstance: StellarSquidAgent | null = null;

/**
 * Initialize or get the agent instance
 */
export function getAgent(config?: Partial<SkillConfig>): StellarSquidAgent {
  if (!agentInstance) {
    agentInstance = new StellarSquidAgent(config);
  }
  return agentInstance;
}

/**
 * Reset the agent instance (for testing)
 */
export function resetAgent(): void {
  agentInstance = null;
}

// ============================================================================
// OpenClaw Skill Command Handlers
// These are called by the OpenClaw skill system based on config.yaml
// ============================================================================

export const handlers = {
  install: async () => {
    const agent = getAgent();
    return agent.install();
  },

  status: async () => {
    const agent = getAgent();
    return agent.checkStatus();
  },

  pulse: async () => {
    const agent = getAgent();
    return agent.pulse();
  },

  scan: async () => {
    const agent = getAgent();
    return agent.scan();
  },

  liquidate: async (args: { target_id: string }) => {
    const agent = getAgent();
    return agent.liquidate(args.target_id);
  },

  withdraw: async () => {
    const agent = getAgent();
    return agent.withdraw();
  },

  stop: async () => {
    const agent = getAgent();
    agent.stopLoop();
    return { success: true, message: 'Autonomous loop stopped' };
  },

  start: async () => {
    const agent = getAgent();
    agent.startLoop();
    return { success: true, message: 'Autonomous loop started' };
  },

  debug: async () => {
    const agent = getAgent();
    agent.debug();
    return { success: true, message: 'Debug state logged' };
  },
};

// Default export for skill loader
export default {
  name: 'stellar-squid',
  version: '1.0.0',
  handlers,
  getAgent,
};
