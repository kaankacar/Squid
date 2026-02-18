#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    vec, BytesN, Env, IntoVal,
};

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn setup_contract(env: &Env) -> (GameRegistryClient, Address) {
    let contract_id = env.register_contract(None, GameRegistry);
    let client = GameRegistryClient::new(env, &contract_id);
    let protocol_fee_address = Address::generate(env);
    
    client.init(&protocol_fee_address);
    
    (client, protocol_fee_address)
}

fn create_agent(env: &Env) -> (Address, BytesN<32>) {
    let agent_contract = Address::generate(env);
    let agent_id = BytesN::from_array(env, &[1u8; 32]);
    (agent_contract, agent_id)
}

fn advance_ledger(env: &Env, ledgers: u32) {
    let current = env.ledger().sequence();
    env.ledger().set_sequence(current + ledgers);
}

// Import the contract client
use super::GameRegistryClient;

#[test]
fn test_init() {
    let env = setup_env();
    let (client, protocol_fee_address) = setup_contract(&env);
    
    let stored_address = client.get_protocol_fee_address();
    assert_eq!(stored_address, protocol_fee_address);
    
    let prize_pool = client.get_prize_pool();
    assert_eq!(prize_pool, 0);
}

#[test]
fn test_init_season() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    // Initialize first season
    let season_id = client.init_season();
    assert_eq!(season_id, 1);
    
    let state = client.get_season_state();
    assert_eq!(state.season_id, 1);
    assert_eq!(state.current_round, 1);
    assert!(!state.season_ended);
    
    // Try to init again while season is active - should fail
    let result = client.try_init_season();
    assert!(result.is_err());
}

#[test]
fn test_register_agent() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    // Need to init season first
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    
    // Register agent
    client.register(&agent_contract, &agent_id);
    
    // Check agent is registered
    let agents = client.get_all_agents();
    assert_eq!(agents.len(), 1);
    
    let agent_summary = agents.get(0).unwrap();
    assert_eq!(agent_summary.agent_id, agent_id);
    assert_eq!(agent_summary.status, AgentStatus::Alive);
    assert_eq!(agent_summary.heart_balance, ENTRY_BOND);
    
    // Check detailed record
    let agent_detail = client.get_agent_detail(&agent_id);
    assert_eq!(agent_detail.contract_address, agent_contract);
    assert_eq!(agent_detail.season_id, 1);
    assert_eq!(agent_detail.round_joined, 1);
}

#[test]
fn test_cannot_register_duplicate() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    
    client.register(&agent_contract, &agent_id);
    
    // Try to register same agent again - should fail
    let result = client.try_register(&agent_contract, &agent_id);
    assert!(result.is_err());
}

#[test]
fn test_cannot_register_without_season() {
    let env = setup_env();
    let contract_id = env.register_contract(None, GameRegistry);
    let client = GameRegistryClient::new(&env, &contract_id);
    let protocol_fee_address = Address::generate(&env);
    
    // Initialize contract but don't start season
    client.init(&protocol_fee_address);
    
    let (agent_contract, agent_id) = create_agent(&env);
    
    // Try to register without active season - should fail
    let result = client.try_register(&agent_contract, &agent_id);
    assert!(result.is_err());
}

#[test]
fn test_advance_round() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    // Register an agent
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    // Try to advance before round deadline - should fail
    let result = client.try_advance_round();
    assert!(result.is_err());
    
    // Advance past round deadline
    advance_ledger(&env, ROUND_1_DURATION + 1);
    
    // Now can advance
    let new_round = client.advance_round();
    assert_eq!(new_round, 2);
    
    let state = client.get_season_state();
    assert_eq!(state.current_round, 2);
    assert_eq!(state.pulse_cost, ROUND_2_COST);
    assert_eq!(state.pulse_period, ROUND_2_PULSE_PERIOD);
}

#[test]
fn test_advance_through_all_rounds() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    // Advance through rounds 1-5
    let durations = [ROUND_1_DURATION, ROUND_2_DURATION, ROUND_3_DURATION, ROUND_4_DURATION, ROUND_5_DURATION];
    
    for i in 0..5 {
        advance_ledger(&env, durations[i] + 1);
        let round = client.advance_round();
        
        if i < 4 {
            assert_eq!(round, (i + 2) as u32);
        } else {
            // After round 5, season ends
            assert_eq!(round, 5);
        }
    }
    
    let state = client.get_season_state();
    assert!(state.season_ended);
}

#[test]
fn test_new_season_after_end() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    // First season
    client.init_season();
    
    // End the season
    advance_ledger(&env, ROUND_1_DURATION + 1);
    client.advance_round(); // Round 2
    advance_ledger(&env, ROUND_2_DURATION + 1);
    client.advance_round(); // Round 3
    advance_ledger(&env, ROUND_3_DURATION + 1);
    client.advance_round(); // Round 4
    advance_ledger(&env, ROUND_4_DURATION + 1);
    client.advance_round(); // Round 5
    advance_ledger(&env, ROUND_5_DURATION + 1);
    client.advance_round(); // Season ends
    
    // Start new season
    let new_season = client.init_season();
    assert_eq!(new_season, 2);
    
    let state = client.get_season_state();
    assert_eq!(state.season_id, 2);
    assert_eq!(state.current_round, 1);
    assert!(!state.season_ended);
}

#[test]
fn test_update_agent_pulse() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    let initial_prize_pool = client.get_prize_pool();
    assert_eq!(initial_prize_pool, 0);
    
    // Simulate pulse - on time
    let pulse_amount = ROUND_1_COST;
    client.update_agent_pulse(&agent_id, &pulse_amount, &false);
    
    // Check prize pool increased (5% of pulse)
    let prize_pool = client.get_prize_pool();
    let expected_prize_contribution = pulse_amount * 5 / 100; // 5%
    assert_eq!(prize_pool, expected_prize_contribution);
    
    // Check agent stats
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.streak_count, 1);
    assert!(agent.activity_score > 0);
    assert_eq!(agent.total_spent, pulse_amount);
    
    // Agent balance should be reduced (entry bond - 10% total deductions)
    let expected_deductions = pulse_amount * 10 / 100; // 5% protocol + 5% prize
    assert_eq!(agent.heart_balance, ENTRY_BOND - expected_deductions);
}

#[test]
fn test_late_pulse() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    // Simulate late pulse
    let pulse_amount = ROUND_1_COST;
    client.update_agent_pulse(&agent_id, &pulse_amount, &true);
    
    // Check agent is wounded
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Wounded);
    assert_eq!(agent.wound_count, 1);
    assert_eq!(agent.streak_count, 0); // Streak reset
}

#[test]
fn test_recover_from_wounded() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    // Make agent wounded
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &true);
    
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Wounded);
    
    // First on-time pulse - still wounded
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    let agent = client.get_agent_detail(&agent_id);
    // Note: In the actual implementation, the status clears after 2 on-time pulses
    // but the first one doesn't fully clear it yet
    
    // Second on-time pulse - should be alive now
    client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Alive);
}

#[test]
fn test_mark_agent_dead() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    // Mark as dead
    client.mark_agent_dead(&agent_id);
    
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Dead);
    
    // Should appear in dead agents list
    let dead_agents = client.get_dead_agents();
    assert_eq!(dead_agents.len(), 1);
}

#[test]
fn test_get_dead_agents_grace_expired() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    // Get agent detail to find deadline
    let agent = client.get_agent_detail(&agent_id);
    
    // Advance past grace deadline (grace period is 720 ledgers for round 1)
    advance_ledger(&env, ROUND_1_PULSE_PERIOD + ROUND_1_GRACE + 1);
    
    // Should now appear as dead (grace expired)
    let dead_agents = client.get_dead_agents();
    assert_eq!(dead_agents.len(), 1);
    
    let dead_summary = dead_agents.get(0).unwrap();
    assert_eq!(dead_summary.status, AgentStatus::Dead);
    assert_eq!(dead_summary.agent_id, agent_id);
}

#[test]
fn test_transfer_kill_reward() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    // Create killer and victim
    let (killer_contract, killer_id) = create_agent(&env);
    let (_, victim_id) = create_agent(&env);
    
    // Register both (using different agent_ids)
    client.register(&killer_contract, &killer_id);
    
    // For victim, create a different ID
    let victim_contract = Address::generate(&env);
    let victim_id = BytesN::from_array(&env, &[2u8; 32]);
    client.register(&victim_contract, &victim_id);
    
    // Mark victim as dead
    client.mark_agent_dead(&victim_id);
    
    // Get victim's balance
    let victim_before = client.get_agent_detail(&victim_id);
    let victim_balance = victim_before.heart_balance;
    
    // Get killer's balance before
    let killer_before = client.get_agent_detail(&killer_id);
    let killer_balance_before = killer_before.heart_balance;
    
    // Transfer kill reward
    let reward = client.transfer_kill_reward(&victim_id, &killer_id);
    assert_eq!(reward, victim_balance);
    
    // Check killer received reward
    let killer_after = client.get_agent_detail(&killer_id);
    assert_eq!(killer_after.heart_balance, killer_balance_before + victim_balance);
    assert_eq!(killer_after.kill_count, 1);
    assert_eq!(killer_after.total_earned, victim_balance);
    
    // Check victim balance is 0
    let victim_after = client.get_agent_detail(&victim_id);
    assert_eq!(victim_after.heart_balance, 0);
}

#[test]
fn test_process_withdrawal() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    let initial_prize_pool = client.get_prize_pool();
    
    // Process withdrawal
    let refund = client.process_withdrawal(&agent_id);
    
    // Refund should be 80% of entry bond
    assert_eq!(refund, ENTRY_BOND * 80 / 100);
    
    // 20% went to prize pool
    let prize_pool_after = client.get_prize_pool();
    assert_eq!(prize_pool_after, ENTRY_BOND * 20 / 100);
    
    // Agent is marked as withdrawn
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.status, AgentStatus::Withdrawn);
    assert_eq!(agent.heart_balance, 0);
}

#[test]
fn test_get_vulnerable_agents() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    // Register two agents
    let (agent1_contract, agent1_id) = create_agent(&env);
    client.register(&agent1_contract, &agent1_id);
    
    let agent2_contract = Address::generate(&env);
    let agent2_id = BytesN::from_array(&env, &[2u8; 32]);
    client.register(&agent2_contract, &agent2_id);
    
    // Make agent1 wounded
    client.update_agent_pulse(&agent1_id, &ROUND_1_COST, &true);
    
    // Check vulnerable agents
    let vulnerable = client.get_vulnerable_agents();
    assert_eq!(vulnerable.len(), 1);
    assert_eq!(vulnerable.get(0).unwrap().agent_id, agent1_id);
    
    // Advance close to agent2's deadline
    advance_ledger(&env, ROUND_1_PULSE_PERIOD - 100);
    
    let vulnerable = client.get_vulnerable_agents();
    // Now both should be vulnerable
    assert_eq!(vulnerable.len(), 2);
}

#[test]
fn test_season_state() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    // Register some agents
    for i in 0..3 {
        let agent_contract = Address::generate(&env);
        let agent_id = BytesN::from_array(&env, &[i as u8; 32]);
        client.register(&agent_contract, &agent_id);
    }
    
    // Kill one agent
    let dead_agent_id = BytesN::from_array(&env, &[0u8; 32]);
    client.mark_agent_dead(&dead_agent_id);
    
    let state = client.get_season_state();
    assert_eq!(state.season_id, 1);
    assert_eq!(state.current_round, 1);
    assert_eq!(state.total_agents, 3);
    assert_eq!(state.dead_agents, 1);
    assert_eq!(state.alive_agents, 2); // 1 alive + 1 wounded = 2 alive
    assert_eq!(state.pulse_cost, ROUND_1_COST);
    assert!(!state.season_ended);
}

#[test]
fn test_streak_bonus() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    // Pulse multiple times to build streak
    for _ in 0..10 {
        client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    }
    
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.streak_count, 10);
    assert!(agent.activity_score > 100); // Should have bonus
    
    // Continue to next tier
    for _ in 0..15 {
        client.update_agent_pulse(&agent_id, &ROUND_1_COST, &false);
    }
    
    let agent = client.get_agent_detail(&agent_id);
    assert_eq!(agent.streak_count, 25);
}

#[test]
fn test_cannot_update_dead_agent() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    // Mark as dead
    client.mark_agent_dead(&agent_id);
    
    // Try to pulse dead agent - should fail
    let result = client.try_update_agent_pulse(
        &agent_id,
        &ROUND_1_COST,
        &false,
    );
    assert!(result.is_err());
}

#[test]
fn test_cannot_update_withdrawn_agent() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    // Withdraw
    client.process_withdrawal(&agent_id);
    
    // Try to pulse withdrawn agent - should fail
    let result = client.try_update_agent_pulse(
        &agent_id,
        &ROUND_1_COST,
        &false,
    );
    assert!(result.is_err());
}

#[test]
fn test_claim_prize() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    // Register agents
    let (agent1_contract, agent1_id) = create_agent(&env);
    client.register(&agent1_contract, &agent1_id);
    
    let agent2_contract = Address::generate(&env);
    let agent2_id = BytesN::from_array(&env, &[2u8; 32]);
    client.register(&agent2_contract, &agent2_id);
    
    // Add activity to both
    client.update_agent_pulse(&agent1_id, &ROUND_1_COST, &false);
    client.update_agent_pulse(&agent2_id, &ROUND_1_COST, &false);
    
    // Add prize pool contribution
    let contribution = 1000_0000000i128; // 1000 XLM
    client.update_agent_pulse(&agent1_id, &contribution, &false);
    
    // End season
    advance_ledger(&env, ROUND_1_DURATION + 1);
    client.advance_round();
    advance_ledger(&env, ROUND_2_DURATION + 1);
    client.advance_round();
    advance_ledger(&env, ROUND_3_DURATION + 1);
    client.advance_round();
    advance_ledger(&env, ROUND_4_DURATION + 1);
    client.advance_round();
    advance_ledger(&env, ROUND_5_DURATION + 1);
    client.advance_round(); // End season
    
    // Verify season ended
    let state = client.get_season_state();
    assert!(state.season_ended);
    
    // Claim prize
    let prize = client.claim_prize(&agent1_id);
    assert!(prize > 0);
    
    // Check agent received prize
    let agent = client.get_agent_detail(&agent1_id);
    assert!(agent.heart_balance > ENTRY_BOND);
}

#[test]
fn test_cannot_claim_prize_before_season_end() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let (agent_contract, agent_id) = create_agent(&env);
    client.register(&agent_contract, &agent_id);
    
    client.update_agent_pulse(
        &agent_id, &ROUND_1_COST, &false);
    
    // Try to claim before season ends - should fail
    let result = client.try_claim_prize(&agent_id);
    assert!(result.is_err());
}

#[test]
fn test_round_configs() {
    let env = setup_env();
    
    let config1 = get_round_config(&env, 1);
    assert_eq!(config1.duration, ROUND_1_DURATION);
    assert_eq!(config1.pulse_cost, ROUND_1_COST);
    
    let config3 = get_round_config(&env, 3);
    assert_eq!(config3.duration, ROUND_3_DURATION);
    assert_eq!(config3.pulse_cost, ROUND_3_COST);
    
    let config5 = get_round_config(&env, 5);
    assert_eq!(config5.duration, ROUND_5_DURATION);
    assert_eq!(config5.pulse_cost, ROUND_5_COST);
}

#[test]
fn test_get_all_agents_empty() {
    let env = setup_env();
    let (client, _) = setup_contract(&env);
    
    client.init_season();
    
    let agents = client.get_all_agents();
    assert_eq!(agents.len(), 0);
}
