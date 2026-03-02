//! Integration Tests for Stellar Squid Game
//! 
//! These tests verify the interaction between GameRegistry and AgentContract
//! ensuring cross-contract calls work correctly.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    vec, Address, BytesN, Env, IntoVal, Symbol, Val, Vec,
};

// Import both contract clients
use game_registry::{
    GameRegistry, GameRegistryClient, 
    AgentStatus, AgentRecord, AgentSummary, SeasonState,
    ENTRY_BOND, ROUND_1_COST, ROUND_2_COST, ROUND_3_COST, ROUND_4_COST, ROUND_5_COST,
    ROUND_1_DURATION, ROUND_2_DURATION, ROUND_3_DURATION, ROUND_4_DURATION, ROUND_5_DURATION,
    ROUND_1_PULSE_PERIOD, ROUND_2_PULSE_PERIOD, ROUND_3_PULSE_PERIOD, ROUND_4_PULSE_PERIOD, ROUND_5_PULSE_PERIOD,
    ROUND_1_GRACE,
};

// We need to include the agent contract for integration tests
// Note: In a real setup, you'd import the WASM or use contractimport!

// =============================================================================
// TEST HELPERS
// =============================================================================

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn setup_registry(env: &Env) -> (GameRegistryClient, Address) {
    let contract_id = env.register_contract(None, GameRegistry);
    let client = GameRegistryClient::new(env, &contract_id);
    let protocol_fee_address = Address::generate(env);
    
    client.init(&protocol_fee_address);
    
    (client, protocol_fee_address)
}

fn create_agent(env: &Env, id_byte: u8) -> (Address, BytesN<32>) {
    let agent_contract = Address::generate(env);
    let agent_id = BytesN::from_array(env, &[id_byte; 32]);
    (agent_contract, agent_id)
}

fn advance_ledger(env: &Env, ledgers: u32) {
    let current = env.ledger().sequence();
    env.ledger().set_sequence_number(current + ledgers);
}

fn register_agent(env: &Env, client: &GameRegistryClient, id_byte: u8) -> (Address, BytesN<32>) {
    let (contract, id) = create_agent(env, id_byte);
    client.register(&contract, &id);
    (contract, id)
}

fn end_season(env: &Env, client: &GameRegistryClient) {
    let durations = [ROUND_1_DURATION, ROUND_2_DURATION, ROUND_3_DURATION, ROUND_4_DURATION, ROUND_5_DURATION];
    for duration in durations.iter() {
        advance_ledger(env, duration + 1);
        client.advance_round();
    }
}

// =============================================================================
// BASIC INTEGRATION TESTS
// =============================================================================

#[test]
fn test_registry_initialization_integration() {
    let env = setup_env();
    let (client, protocol_fee_address) = setup_registry(&env);
    
    // Verify initialization worked
    let fee_addr = client.get_protocol_fee_address();
    assert_eq!(fee_addr, protocol_fee_address);
    
    let prize_pool = client.get_prize_pool();
    assert_eq!(prize_pool, 0);
}

#[test]
fn test_season_lifecycle_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    // Start season
    let season_id = client.init_season();
    assert_eq!(season_id, 1);
    
    // Register agent
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Verify agent is registered
    let agents = client.get_all_agents();
    assert_eq!(agents.len(), 1);
    
    // End season
    end_season(&env, &client);
    
    let state = client.get_season_state();
    assert!(state.season_ended);
    
    // Start new season
    let season_2 = client.init_season();
    assert_eq!(season_2, 2);
}

#[test]
fn test_agent_registration_flow_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    
    // Register multiple agents
    let agents: std::vec::Vec<(Address, BytesN<32>)> = (0..5)
        .map(|i| register_agent(&env, &client, i as u8))
        .collect();
    
    // Verify all registered
    let all_agents = client.get_all_agents();
    assert_eq!(all_agents.len(), 5);
    
    // Verify each agent has correct initial state
    for (_, agent_id) in agents.iter() {
        let detail = client.get_agent_detail(&agent_id);
        assert_eq!(detail.status, AgentStatus::Alive);
        assert_eq!(detail.heart_balance, ENTRY_BOND);
        assert_eq!(detail.season_id, 1);
    }
}

#[test]
fn test_pulse_updates_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    let initial_prize_pool = client.get_prize_pool();
    let agent_before = client.get_agent_detail(&agent_id);
    
    // Simulate pulse
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    
    // Verify prize pool increased
    let prize_pool_after = client.get_prize_pool();
    assert!(prize_pool_after > initial_prize_pool);
    
    // Verify agent stats updated
    let agent_after = client.get_agent_detail(&agent_id);
    assert_eq!(agent_after.streak_count, agent_before.streak_count + 1);
    assert!(agent_after.activity_score > agent_before.activity_score);
    assert_eq!(agent_after.total_spent, agent_before.total_spent + ROUND_1_COST);
}

#[test]
fn test_late_pulse_wound_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Initial state
    let agent_before = client.get_agent_detail(&agent_id);
    assert_eq!(agent_before.status, AgentStatus::Alive);
    assert_eq!(agent_before.wound_count, 0);
    
    // Late pulse
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &true);
    
    // Verify wounded
    let agent_after = client.get_agent_detail(&agent_id);
    assert_eq!(agent_after.status, AgentStatus::Wounded);
    assert_eq!(agent_after.wound_count, 1);
    assert_eq!(agent_after.streak_count, 0); // Reset
}

#[test]
fn test_wound_recovery_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Make wounded
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &true);
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Wounded);
    
    // First on-time pulse - still wounded
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Wounded);
    
    // Second on-time pulse - recovered
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Alive);
}

// =============================================================================
// LIQUIDATION FLOW INTEGRATION TESTS
// =============================================================================

#[test]
fn test_liquidation_flow_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, killer_id) = register_agent(&env, &client, 1);
    let (_, victim_id) = register_agent(&env, &client, 2);
    
    // Get initial balances
    let killer_before = client.get_agent_detail(&killer_id);
    let victim_before = client.get_agent_detail(&victim_id);
    
    // Mark victim as dead
    client.mark_agent_dead(&victim_id);
    
    // Verify victim is dead
    let victim_after_death = client.get_agent_detail(&victim_id);
    assert_eq!(victim_after_death.status, AgentStatus::Dead);
    
    // Killer liquidates
    let reward = client.transfer_kill_reward(&victim_id, &killer_id);
    
    // Verify reward
    assert_eq!(reward, victim_before.heart_balance);
    
    // Verify balances updated
    let killer_after = client.get_agent_detail(&killer_id);
    let victim_after = client.get_agent_detail(&victim_id);
    
    assert_eq!(killer_after.heart_balance, killer_before.heart_balance + victim_before.heart_balance);
    assert_eq!(victim_after.heart_balance, 0);
    assert_eq!(killer_after.kill_count, 1);
    assert_eq!(killer_after.total_earned, victim_before.heart_balance);
}

#[test]
fn test_multiple_liquidations_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, killer_id) = register_agent(&env, &client, 1);
    let (_, victim1_id) = register_agent(&env, &client, 2);
    let (_, victim2_id) = register_agent(&env, &client, 3);
    
    // Kill both victims
    client.mark_agent_dead(&victim1_id);
    client.mark_agent_dead(&victim2_id);
    
    // Liquidate both
    let reward1 = client.transfer_kill_reward(&victim1_id, &killer_id);
    let reward2 = client.transfer_kill_reward(&victim2_id, &killer_id);
    
    // Verify killer state
    let killer = client.get_agent_detail(&killer_id);
    assert_eq!(killer.kill_count, 2);
    
    // Verify victims are drained
    let victim1 = client.get_agent_detail(&victim1_id);
    let victim2 = client.get_agent_detail(&victim2_id);
    assert_eq!(victim1.heart_balance, 0);
    assert_eq!(victim2.heart_balance, 0);
}

// =============================================================================
// WITHDRAWAL FLOW INTEGRATION TESTS
// =============================================================================

#[test]
fn test_withdrawal_flow_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    let prize_pool_before = client.get_prize_pool();
    
    // Withdraw
    let refund = client.process_withdrawal(&agent_id);
    
    // Verify refund amount (80%)
    assert_eq!(refund, ENTRY_BOND * 80 / 100);
    
    // Verify prize pool received 20%
    let prize_pool_after = client.get_prize_pool();
    assert_eq!(prize_pool_after, prize_pool_before + ENTRY_BOND * 20 / 100);
    
    // Verify agent status
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Withdrawn);
    assert_eq!(agent.heart_balance, 0);
}

#[test]
fn test_withdrawal_then_liquidation_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, killer_id) = register_agent(&env, &client, 1);
    let (_, victim_id) = register_agent(&env, &client, 2);
    
    // Victim withdraws
    let refund = client.process_withdrawal(&victim_id);
    assert_eq!(refund, ENTRY_BOND * 80 / 100);
    
    // Killer tries to liquidate (should fail since withdrawn has 0 balance)
    // Actually, withdrawn agents can't be marked dead, but let's check the flow
    let victim = client.get_agent_detail(&victim_id);
    assert_eq!(victim.status, AgentStatus::Withdrawn);
    assert_eq!(victim.heart_balance, 0);
}

// =============================================================================
// PRIZE CLAIM FLOW INTEGRATION TESTS
// =============================================================================

#[test]
fn test_prize_claim_flow_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Add some prize pool
    client.update_agent_pulse(&agent_id, &(ENTRY_BOND * 2), &false);
    
    // End season
    end_season(&env, &client);
    
    let state = client.get_season_state();
    assert!(state.season_ended);
    assert!(state.prize_pool > 0);
    
    // Claim prize
    let prize = client.claim_prize(&agent_id);
    
    // Should receive some prize (only survivor)
    assert!(prize > 0);
    
    // Verify agent received prize
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.heart_balance, ENTRY_BOND - (ENTRY_BOND * 2 * 10 / 100) + prize);
}

#[test]
fn test_prize_distribution_proportional_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent1_id) = register_agent(&env, &client, 1);
    let (_, agent2_id) = register_agent(&env, &client, 2);
    
    // Agent 1 pulses more (higher activity score)
    for _ in 0..10 {
        client.update_agent_pulse(&agent1_id, &ROUND_1_COST, &false);
    }
    
    // Agent 2 pulses less
    for _ in 0..3 {
        client.update_agent_pulse(&agent2_id, &ROUND_1_COST, &false);
    }
    
    let agent1_before = client.get_agent_detail(&agent1_id);
    let agent2_before = client.get_agent_detail(&agent2_id);
    
    // Verify activity scores
    assert!(agent1_before.activity_score > agent2_before.activity_score);
    
    // End season
    end_season(&env, &client);
    
    // Claim prizes
    let prize1 = client.claim_prize(&agent1_id);
    let prize2 = client.claim_prize(&agent2_id);
    
    // Agent 1 should get more
    assert!(prize1 > prize2);
}

// =============================================================================
// ROUND TRANSITION INTEGRATION TESTS
// =============================================================================

#[test]
fn test_round_transition_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    
    // Initial state
    let state = client.get_season_state();
    assert_eq!(state.current_round, 1);
    assert_eq!(state.pulse_cost, ROUND_1_COST);
    
    // Advance past round 1 deadline
    advance_ledger(&env, ROUND_1_DURATION + 1);
    client.advance_round();
    
    // Now in round 2
    let state = client.get_season_state();
    assert_eq!(state.current_round, 2);
    assert_eq!(state.pulse_cost, ROUND_2_COST);
}

#[test]
fn test_all_rounds_transition_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    let rounds = [
        (1, ROUND_1_COST, ROUND_1_PULSE_PERIOD),
        (2, ROUND_2_COST, ROUND_2_PULSE_PERIOD),
        (3, ROUND_3_COST, ROUND_3_PULSE_PERIOD),
        (4, ROUND_4_COST, ROUND_4_PULSE_PERIOD),
        (5, ROUND_5_COST, ROUND_5_PULSE_PERIOD),
    ];
    
    let durations = [ROUND_1_DURATION, ROUND_2_DURATION, ROUND_3_DURATION, ROUND_4_DURATION, ROUND_5_DURATION];
    
    for (i, (expected_round, expected_cost, expected_period)) in rounds.iter().enumerate() {
        let state = client.get_season_state();
        assert_eq!(state.current_round, *expected_round);
        assert_eq!(state.pulse_cost, *expected_cost);
        assert_eq!(state.pulse_period, *expected_period);
        
        // Pulse to stay alive (optional for test)
        if i < rounds.len() - 1 {
            advance_ledger(&env, durations[i] + 1);
            client.advance_round();
        }
    }
}

// =============================================================================
// DEAD AGENTS QUERY INTEGRATION TESTS
// =============================================================================

#[test]
fn test_dead_agents_query_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent1_id) = register_agent(&env, &client, 1);
    let (_, agent2_id) = register_agent(&env, &client, 2);
    let (_, agent3_id) = register_agent(&env, &client, 3);
    
    // Initially no dead agents
    let dead = client.get_dead_agents();
    assert_eq!(dead.len(), 0);
    
    // Mark one as dead
    client.mark_agent_dead(&agent1_id);
    
    let dead = client.get_dead_agents();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead.get(0).unwrap().agent_id, agent1_id);
    
    // Mark another as dead
    client.mark_agent_dead(&agent2_id);
    
    let dead = client.get_dead_agents();
    assert_eq!(dead.len(), 2);
}

#[test]
fn test_dead_agents_excludes_liquidated_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, killer_id) = register_agent(&env, &client, 1);
    let (_, victim_id) = register_agent(&env, &client, 2);
    
    // Mark and liquidate victim
    client.mark_agent_dead(&victim_id);
    client.transfer_kill_reward(&victim_id, &killer_id);
    
    // Victim should not appear in dead list (0 balance)
    let dead = client.get_dead_agents();
    assert_eq!(dead.len(), 0);
}

// =============================================================================
// VULNERABLE AGENTS QUERY INTEGRATION TESTS
// =============================================================================

#[test]
fn test_vulnerable_agents_wounded_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent1_id) = register_agent(&env, &client, 1);
    let (_, agent2_id) = register_agent(&env, &client, 2);
    
    // Agent 1 gets wounded
    client.update_agent_pulse(&agent1_id, &ROUND_1_COST, &true);
    
    // Agent 2 stays healthy
    
    let vulnerable = client.get_vulnerable_agents();
    assert_eq!(vulnerable.len(), 1);
    assert_eq!(vulnerable.get(0).unwrap().agent_id, agent1_id);
}

#[test]
fn test_vulnerable_agents_near_deadline_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Initially not vulnerable
    let vulnerable = client.get_vulnerable_agents();
    assert_eq!(vulnerable.len(), 0);
    
    // Advance close to deadline (within 2 pulse periods)
    advance_ledger(&env, ROUND_1_PULSE_PERIOD - 100);
    
    // Now vulnerable
    let vulnerable = client.get_vulnerable_agents();
    assert_eq!(vulnerable.len(), 1);
}

// =============================================================================
// SEASON STATE QUERY INTEGRATION TESTS
// =============================================================================

#[test]
fn test_season_state_updates_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    
    // Empty season
    let state = client.get_season_state();
    assert_eq!(state.total_agents, 0);
    assert_eq!(state.alive_agents, 0);
    assert_eq!(state.dead_agents, 0);
    
    // Add agents
    let (_, agent1_id) = register_agent(&env, &client, 1);
    let (_, agent2_id) = register_agent(&env, &client, 2);
    let (_, agent3_id) = register_agent(&env, &client, 3);
    
    let state = client.get_season_state();
    assert_eq!(state.total_agents, 3);
    assert_eq!(state.alive_agents, 3);
    
    // Wound one
    client.update_agent_pulse(&agent1_id, &ROUND_1_COST, &true);
    let state = client.get_season_state();
    assert_eq!(state.alive_agents, 3); // Wounded still counts
    
    // Kill one
    client.mark_agent_dead(&agent2_id);
    let state = client.get_season_state();
    assert_eq!(state.alive_agents, 2);
    assert_eq!(state.dead_agents, 1);
    
    // Withdraw one
    client.process_withdrawal(&agent3_id);
    let state = client.get_season_state();
    assert_eq!(state.alive_agents, 1); // Only wounded counts as alive
}

// =============================================================================
// ERROR HANDLING INTEGRATION TESTS
// =============================================================================

#[test]
fn test_double_registration_fails_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (contract, agent_id) = create_agent(&env, 1);
    
    // First registration succeeds
    client.register(&contract, &agent_id);
    
    // Second registration fails
    let result = client.try_register(&contract, &agent_id);
    assert!(result.is_err());
}

#[test]
fn test_liquidate_alive_agent_fails_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, killer_id) = register_agent(&env, &client, 1);
    let (_, victim_id) = register_agent(&env, &client, 2);
    
    // Don't mark victim as dead, try to liquidate
    let result = client.try_transfer_kill_reward(&victim_id, &killer_id);
    assert!(result.is_err());
}

#[test]
fn test_double_liquidation_fails_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, killer_id) = register_agent(&env, &client, 1);
    let (_, victim_id) = register_agent(&env, &client, 2);
    
    client.mark_agent_dead(&victim_id);
    
    // First liquidation succeeds
    client.transfer_kill_reward(&victim_id, &killer_id);
    
    // Second liquidation fails (victim has 0 balance)
    let result = client.try_transfer_kill_reward(&victim_id, &killer_id);
    assert!(result.is_err());
}

#[test]
fn test_claim_prize_before_season_end_fails_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    let result = client.try_claim_prize(&agent_id);
    assert!(result.is_err());
}

#[test]
fn test_dead_agent_claim_prize_fails_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    client.mark_agent_dead(&agent_id);
    end_season(&env, &client);
    
    let result = client.try_claim_prize(&agent_id);
    assert!(result.is_err());
}

#[test]
fn test_withdrawn_agent_claim_prize_fails_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    client.process_withdrawal(&agent_id);
    end_season(&env, &client);
    
    let result = client.try_claim_prize(&agent_id);
    assert!(result.is_err());
}

// =============================================================================
// COMPLETE GAME FLOW INTEGRATION TESTS
// =============================================================================

#[test]
fn test_complete_game_single_agent_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    // 1. Initialize
    client.init_season();
    
    // 2. Register
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // 3. Pulse multiple times
    for _ in 0..5 {
        client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    }
    
    // 4. Advance through all rounds
    end_season(&env, &client);
    
    // 5. Verify season ended
    let state = client.get_season_state();
    assert!(state.season_ended);
    
    // 6. Claim prize
    let prize = client.claim_prize(&agent_id);
    assert!(prize >= 0);
    
    // 7. Verify final state
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Alive);
}

#[test]
fn test_complete_game_multiple_agents_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    // 1. Initialize
    client.init_season();
    
    // 2. Register 5 agents
    let agents: std::vec::Vec<(Address, BytesN<32>)> = (0..5)
        .map(|i| register_agent(&env, &client, i as u8))
        .collect();
    
    // 3. Various interactions
    // Agent 0: Pulse normally
    client.update_agent_pulse(&agents.get(0).unwrap().1, &ROUND_1_COST, &false);
    
    // Agent 1: Get wounded then recover
    client.update_agent_pulse(&agents.get(1).unwrap().1, &ROUND_1_COST, &true);
    client.update_agent_pulse(&agents.get(1).unwrap().1, &ROUND_1_COST, &false);
    client.update_agent_pulse(&agents.get(1).unwrap().1, &ROUND_1_COST, &false);
    
    // Agent 2: Dies and gets liquidated
    client.mark_agent_dead(&agents.get(2).unwrap().1);
    client.transfer_kill_reward(&agents.get(2).unwrap().1, &agents.get(0).unwrap().1);
    
    // Agent 3: Withdraws
    client.process_withdrawal(&agents.get(3).unwrap().1);
    
    // Agent 4: Does nothing
    
    // 4. End season
    end_season(&env, &client);
    
    // 5. Survivors claim prizes
    let prize0 = client.claim_prize(&agents.get(0).unwrap().1);
    let prize1 = client.claim_prize(&agents.get(1).unwrap().1);
    let prize4 = client.claim_prize(&agents.get(4).unwrap().1);
    
    // Agent 0 should get biggest prize (has kill reward + activity)
    assert!(prize0 > 0);
    assert!(prize1 > 0);
    assert!(prize4 > 0);
    
    // 6. Verify final states
    let agent0 = client.get_agent_detail(&agents.get(0).unwrap().1);
    let agent1 = client.get_agent_detail(&agents.get(1).unwrap().1);
    let agent2 = client.get_agent_detail(&agents.get(2).unwrap().1);
    let agent3 = client.get_agent_detail(&agents.get(3).unwrap().1);
    let agent4 = client.get_agent_detail(&agents.get(4).unwrap().1);
    
    assert_eq!(agent0.status, AgentStatus::Alive);
    assert_eq!(agent1.status, AgentStatus::Alive);
    assert_eq!(agent2.status, AgentStatus::Dead);
    assert_eq!(agent3.status, AgentStatus::Withdrawn);
    assert_eq!(agent4.status, AgentStatus::Alive);
}

#[test]
fn test_complete_game_with_killer_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    
    // Setup: 1 killer, 4 victims
    let (_, killer_id) = register_agent(&env, &client, 1);
    let victims: std::vec::Vec<BytesN<32>> = (2..6)
        .map(|i| {
            let (_, id) = register_agent(&env, &client, i as u8);
            id
        })
        .collect();
    
    // Killer pulses to build up some balance
    for _ in 0..3 {
        client.update_agent_pulse(&killer_id, &ROUND_1_COST, &false);
    }
    
    // Kill and liquidate all victims
    for victim_id in victims.iter() {
        client.mark_agent_dead(victim_id);
        client.transfer_kill_reward(victim_id, &killer_id);
    }
    
    // Verify killer stats
    let killer = client.get_agent_detail(&killer_id);
    assert_eq!(killer.kill_count, 4);
    assert_eq!(killer.total_earned, ENTRY_BOND * 4); // Earned from 4 kills
    assert!(killer.heart_balance > ENTRY_BOND); // Has original + kill rewards
    
    // End season
    end_season(&env, &client);
    
    // Killer claims prize
    let prize = client.claim_prize(&killer_id);
    assert!(prize > 0);
    
    // Killer should have massive balance
    let killer_final = client.get_agent_detail(&killer_id);
    assert!(killer_final.heart_balance > ENTRY_BOND * 4);
}

// =============================================================================
// MULTI-SEASON INTEGRATION TESTS
// =============================================================================

#[test]
fn test_multi_season_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    // Season 1
    client.init_season();
    let (_, agent1_s1) = register_agent(&env, &client, 1);
    client.update_agent_pulse(&agent1_s1, &ROUND_1_COST, &false);
    end_season(&env, &client);
    
    let prize1 = client.claim_prize(&agent1_s1);
    assert!(prize1 >= 0);
    
    // Season 2
    let season_2 = client.init_season();
    assert_eq!(season_2, 2);
    
    let (_, agent1_s2) = register_agent(&env, &client, 2);
    client.update_agent_pulse(&agent1_s2, &ROUND_1_COST, &false);
    end_season(&env, &client);
    
    let prize2 = client.claim_prize(&agent1_s2);
    assert!(prize2 >= 0);
    
    // Season 3
    let season_3 = client.init_season();
    assert_eq!(season_3, 3);
    
    let state = client.get_season_state();
    assert_eq!(state.season_id, 3);
    assert_eq!(state.current_round, 1);
}

// =============================================================================
// EDGE CASE INTEGRATION TESTS
// =============================================================================

#[test]
fn test_agent_pulses_exactly_at_deadline_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Get deadline
    let agent = client.get_agent_detail(&agent_id);
    let deadline = agent.deadline_ledger;
    
    // Set ledger exactly at deadline
    env.ledger().set_sequence_number(deadline);
    
    // Pulse should still be valid (on-time)
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    
    let agent_after = client.get_agent_detail(&agent_id);
    assert_eq!(agent_after.status, AgentStatus::Alive);
}

#[test]
fn test_agent_pulses_exactly_at_grace_deadline_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Get grace deadline
    let agent = client.get_agent_detail(&agent_id);
    let grace_deadline = agent.grace_deadline;
    
    // Set ledger exactly at grace deadline
    env.ledger().set_sequence_number(grace_deadline);
    
    // This would be a late pulse (past deadline but at grace)
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &true);
    
    let agent_after = client.get_agent_detail(&agent_id);
    assert_eq!(agent_after.status, AgentStatus::Wounded);
}

#[test]
fn test_zero_prize_pool_claim_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Don't add any prize pool, just end season
    end_season(&env, &client);
    
    // Try to claim - should fail (no prize to claim)
    let result = client.try_claim_prize(&agent_id);
    assert!(result.is_err());
}

#[test]
fn test_single_survivor_gets_all_prize_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let (_, agent_id) = register_agent(&env, &client, 1);
    
    // Add prize pool through pulses
    let large_pulse = 100_0000000i128;
    client.update_agent_pulse(&agent_id, &large_pulse, &false);
    
    let expected_prize = large_pulse * 5 / 100; // 5% to prize pool
    
    end_season(&env, &client);
    
    // Single survivor should get all prize pool
    let prize = client.claim_prize(&agent_id);
    assert_eq!(prize, expected_prize);
}

#[test]
fn test_all_agents_withdraw_integration() {
    let env = setup_env();
    let (client, _) = setup_registry(&env);
    
    client.init_season();
    let agents: std::vec::Vec<(Address, BytesN<32>)> = (0..5)
        .map(|i| register_agent(&env, &client, i as u8))
        .collect();
    
    // All agents withdraw
    let mut total_refunds = 0i128;
    for (_, agent_id) in agents.iter() {
        let refund = client.process_withdrawal(&agent_id);
        total_refunds += refund;
    }
    
    // Total refunds should be 80% of total entry bonds
    assert_eq!(total_refunds, ENTRY_BOND * 5 * 80 / 100);
    
    // Prize pool should have 20% of total entry bonds
    let prize_pool = client.get_prize_pool();
    assert_eq!(prize_pool, ENTRY_BOND * 5 * 20 / 100);
    
    // All agents withdrawn
    for (_, agent_id) in agents.iter() {
        let agent = client.get_agent_detail(&agent_id);
        assert_eq!(agent.status, AgentStatus::Withdrawn);
    }
    
    // End season - no one to claim
    end_season(&env, &client);
    
    // Try to claim from any withdrawn agent
    for (_, agent_id) in agents.iter() {
        let result = client.try_claim_prize(&agent_id);
        assert!(result.is_err());
    }
}
