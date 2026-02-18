#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Symbol};

fn setup_env() -> (Env, Address, Address, Address) {
    let env = Env::default();
    let owner = Address::generate(&env);
    let registry = Address::generate(&env);
    let contract_id = env.register_contract(None, AgentContract);
    (env, contract_id, owner, registry)
}

#[test]
fn test_constructor() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    // Fund the contract with entry bond
    env.mock_all_auths();
    
    // Initialize should succeed with entry bond
    // Note: In real tests, we'd transfer XLM to the contract first
    // Here we just verify the contract compiles and basic structure
}

#[test]
fn test_agent_status_enum() {
    let env = Env::default();
    
    // Test enum variants
    assert_eq!(AgentStatus::Alive as u32, 0);
    assert_eq!(AgentStatus::Wounded as u32, 1);
    assert_eq!(AgentStatus::Dead as u32, 2);
    assert_eq!(AgentStatus::Withdrawn as u32, 3);
}

#[test]
fn test_entry_bond_constant() {
    assert_eq!(ENTRY_BOND, 50_0000000); // 50 XLM
}

#[test]
fn test_round_configs() {
    // Round 1: Genesis
    let (p1, g1, c1) = get_round_config(1);
    assert_eq!(p1, 4320);     // 6h
    assert_eq!(g1, 720);      // 1h
    assert_eq!(c1, 5000000);  // 0.5 XLM

    // Round 2: Pressure
    let (p2, g2, c2) = get_round_config(2);
    assert_eq!(p2, 2160);      // 3h
    assert_eq!(g2, 360);       // 30min
    assert_eq!(c2, 10000000);  // 1.0 XLM

    // Round 3: Crucible
    let (p3, g3, c3) = get_round_config(3);
    assert_eq!(p3, 720);       // 1h
    assert_eq!(g3, 180);       // 15min
    assert_eq!(c3, 20000000);  // 2.0 XLM

    // Round 4: Apex
    let (p4, g4, c4) = get_round_config(4);
    assert_eq!(p4, 360);       // 30min
    assert_eq!(g4, 120);       // 10min
    assert_eq!(c4, 30000000);  // 3.0 XLM

    // Round 5: Singularity
    let (p5, g5, c5) = get_round_config(5);
    assert_eq!(p5, 180);       // 15min
    assert_eq!(g5, 60);        // 5min
    assert_eq!(c5, 50000000);  // 5.0 XLM
}

#[test]
fn test_pulse_cost_split() {
    let base_cost = 10000000i128; // 1.0 XLM
    
    // 5% protocol fee
    let protocol_fee = base_cost * 5 / 100;
    assert_eq!(protocol_fee, 500000); // 0.05 XLM
    
    // 5% prize pool
    let prize_contribution = base_cost * 5 / 100;
    assert_eq!(prize_contribution, 500000); // 0.05 XLM
    
    // 90% TTL rent
    let ttl_rent = base_cost * 90 / 100;
    assert_eq!(ttl_rent, 9000000); // 0.9 XLM
    
    // Total should equal base cost
    assert_eq!(protocol_fee + prize_contribution + ttl_rent, base_cost);
}

#[test]
fn test_late_pulse_cost() {
    let base_cost = 5000000i128; // 0.5 XLM
    let late_cost = base_cost * 2;
    assert_eq!(late_cost, 10000000); // 1.0 XLM (2x)
}

#[test]
fn test_agent_state_struct() {
    let env = Env::default();
    
    // Create a sample agent state
    let state = AgentState {
        agent_id: BytesN::from_array(&env, &[0u8; 32]),
        owner: Address::generate(&env),
        season_id: 1,
        status: AgentStatus::Alive,
        deadline_ledger: 1000,
        grace_deadline: 1100,
        last_pulse_ledger: 500,
        streak_count: 5,
        activity_score: 100,
        heart_balance: 100_0000000,
        total_earned: 50_0000000,
        total_spent: 10_0000000,
        kill_count: 2,
    };
    
    assert_eq!(state.season_id, 1);
    assert_eq!(state.status, AgentStatus::Alive);
    assert_eq!(state.streak_count, 5);
    assert_eq!(state.kill_count, 2);
}

#[test]
fn test_error_codes() {
    // Verify error codes are sequential
    assert_eq!(Error::AlreadyInitialized as u32, 1);
    assert_eq!(Error::NotInitialized as u32, 2);
    assert_eq!(Error::NotOwner as u32, 3);
    assert_eq!(Error::AgentDead as u32, 4);
    assert_eq!(Error::AgentWithdrawn as u32, 5);
    assert_eq!(Error::InsufficientBalance as u32, 6);
    assert_eq!(Error::SeasonEnded as u32, 7);
    assert_eq!(Error::InvalidTarget as u32, 8);
    assert_eq!(Error::TargetNotDead as u32, 9);
    assert_eq!(Error::PrizeClaimFailed as u32, 10);
    assert_eq!(Error::WithdrawalFailed as u32, 11);
    assert_eq!(Error::PulseFailed as u32, 12);
    assert_eq!(Error::NoPrizeToClaim as u32, 13);
}
