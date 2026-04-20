import { resetAgent, getAgent } from './index';
import { StellarSquidAgent } from './agent';

// Mock the StellarSquidAgent class
jest.mock('./agent', () => {
  return {
    StellarSquidAgent: jest.fn().mockImplementation(() => {
      return {
        stopLoop: jest.fn(),
        // Add other methods if needed for getAgent to work without crashing
        install: jest.fn(),
        checkStatus: jest.fn(),
        pulse: jest.fn(),
        scan: jest.fn(),
        liquidate: jest.fn(),
        withdraw: jest.fn(),
        startLoop: jest.fn(),
        debug: jest.fn(),
        loadKeypair: jest.fn(),
        setContractId: jest.fn(),
      };
    }),
  };
});

describe('resetAgent', () => {
  beforeEach(() => {
    // We need to clear the singleton in index.ts before each test
    // But index.ts doesn't expose the agentInstance variable directly
    // Fortunately, resetAgent itself does exactly that!
    resetAgent();
    jest.clearAllMocks();
  });

  it('should call stopLoop if an agent instance exists', () => {
    // First, create an instance by calling getAgent
    const agent = getAgent();

    // Verify it's an instance of our mocked StellarSquidAgent
    expect(StellarSquidAgent).toHaveBeenCalled();

    // Now call resetAgent
    resetAgent();

    // Verify stopLoop was called on that instance
    expect(agent.stopLoop).toHaveBeenCalledTimes(1);
  });

  it('should not throw if no agent instance exists', () => {
    // Calling resetAgent when no instance exists should be fine
    expect(() => resetAgent()).not.toThrow();
  });

  it('should allow creating a new instance after reset', () => {
    const agent1 = getAgent();
    resetAgent();

    const agent2 = getAgent();

    // Since it's a singleton pattern, agent1 and agent2 would be different
    // because we reset it in between
    expect(agent1).not.toBe(agent2);
    expect(StellarSquidAgent).toHaveBeenCalledTimes(2);
  });
});
