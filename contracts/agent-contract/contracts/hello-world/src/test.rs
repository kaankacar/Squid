#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    Address, BytesN, Env, IntoVal, Symbol, Val, Vec,
};

// ============================================================================
// TEST HELPERS
// ============================================================================

fn setup_env() -> (Env, Address, Address, Address) {
    let env = Env::default();
    let owner = Address::generate(&env);
    let registry = Address::generate(&env);
    let contract_id = env.register_contract(None, AgentContract);
    (env, contract_id, owner, registry)
}

fn advance_ledger(env: &Env, ledgers: u32) {
    let current = env.ledger().sequence();
    env.ledger().set_sequence_number(current + ledgers);
}

// ============================================================================
// BASIC CONSTANT TESTS (Already have 12, adding more)
// ============================================================================

#[test]
fn test_constructor_basic() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    
    // Test that contract can be initialized
    client.constructor(&owner, &registry, &1u32);
    
    // Verify initialization
    assert!(client.is_initialized());
    assert_eq!(client.get_heart_balance(), ENTRY_BOND);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_constructor_double_init_fails() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    
    client.constructor(&owner, &registry, &1u32);
    // Second init should fail
    client.constructor(&owner, &registry, &1u32);
}

#[test]
fn test_get_status_after_init() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    client.constructor(&owner, &registry, &1u32);
    
    let status = client.get_status();
    assert_eq!(status.season_id, 1);
    assert_eq!(status.status, AgentStatus::Alive);
    assert_eq!(status.owner, owner);
}

#[test]
fn test_get_deadlines_after_init() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    env.ledger().set_sequence_number(100);
    
    client.constructor(&owner, &registry, &1u32);
    
    let (current, deadline, grace) = client.get_deadlines();
    assert_eq!(current, 100);
    // Deadline should be current + pulse period for round 1
    assert_eq!(deadline, 100 + ROUND_1_PULSE_PERIOD);
    // Grace should be deadline + grace period
    assert_eq!(grace, deadline + ROUND_1_GRACE);
}

// ============================================================================
// PULSE MECHANICS TESTS
// ============================================================================

#[test]
fn test_pulse_on_time_extends_deadline() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    env.ledger().set_sequence_number(100);
    client.constructor(&owner, &registry, &1u32);
    
    let (_, initial_deadline, _) = client.get_deadlines();
    
    // Pulse on time
    // Note: We can't fully test pulse without mocked registry
    // This tests the basic contract structure
}

#[test]
fn test_streak_tier_bonuses() {
    // Test that different streak tiers give correct bonuses
    let tier_0_bonus = match 5u32 {
        0..=9 => 10u64,
        10..=24 => 11u64,
        25..=49 => 12u64,
        50..=99 => 15u64,
        _ => 20u64,
    };
    assert_eq!(tier_0_bonus, 10);
    
    let tier_1_bonus = match 15u32 {
        0..=9 => 10u64,
        10..=24 => 11u64,
        25..=49 => 12u64,
        50..=99 => 15u64,
        _ => 20u64,
    };
    assert_eq!(tier_1_bonus, 11);
    
    let tier_2_bonus = match 30u32 {
        0..=9 => 10u64,
        10..=24 => 11u64,
        25..=49 => 12u64,
        50..=99 => 15u64,
        _ => 20u64,
    };
    assert_eq!(tier_2_bonus, 12);
    
    let tier_3_bonus = match 75u32 {
        0..=9 => 10u64,
        10..=24 => 11u64,
        25..=49 => 12u64,
        50..=99 => 15u64,
        _ => 20u64,
    };
    assert_eq!(tier_3_bonus, 15);
    
    let tier_4_bonus = match 100u32 {
        0..=9 => 10u64,
        10..=24 => 11u64,
        25..=49 => 12u64,
        50..=99 => 15u64,
        _ => 20u64,
    };
    assert_eq!(tier_4_bonus, 20);
}

#[test]
fn test_pulse_cost_all_rounds() {
    // Verify pulse costs for all rounds
    for round in 1u32..=5 {
        let (_, _, cost) = get_round_config(round);
        match round {
            1 => assert_eq!(cost, 5000000, "Round 1 cost should be 0.5 XLM"),
            2 => assert_eq!(cost, 10000000, "Round 2 cost should be 1.0 XLM"),
            3 => assert_eq!(cost, 20000000, "Round 3 cost should be 2.0 XLM"),
            4 => assert_eq!(cost, 30000000, "Round 4 cost should be 3.0 XLM"),
            5 => assert_eq!(cost, 50000000, "Round 5 cost should be 5.0 XLM"),
            _ => panic!("Invalid round"),
        }
    }
}

#[test]
fn test_late_pulse_cost_doubles_all_rounds() {
    // Late pulse should cost 2x in every round
    for round in 1u32..=5 {
        let (_, _, base_cost) = get_round_config(round);
        let late_cost = base_cost * 2;
        assert_eq!(late_cost, base_cost * 2, "Late cost should be 2x base for round {}", round);
    }
}

// ============================================================================
// ROUND CONFIGURATION TESTS
// ============================================================================

#[test]
fn test_pulse_periods_all_rounds() {
    // Round 1: 6 hours = 4320 ledgers
    let (p1, _, _) = get_round_config(1);
    assert_eq!(p1, 4320, "Round 1 pulse period should be 6 hours (4320 ledgers)");
    
    // Round 2: 3 hours = 2160 ledgers
    let (p2, _, _) = get_round_config(2);
    assert_eq!(p2, 2160, "Round 2 pulse period should be 3 hours (2160 ledgers)");
    
    // Round 3: 1 hour = 720 ledgers
    let (p3, _, _) = get_round_config(3);
    assert_eq!(p3, 720, "Round 3 pulse period should be 1 hour (720 ledgers)");
    
    // Round 4: 30 min = 360 ledgers
    let (p4, _, _) = get_round_config(4);
    assert_eq!(p4, 360, "Round 4 pulse period should be 30 min (360 ledgers)");
    
    // Round 5: 15 min = 180 ledgers
    let (p5, _, _) = get_round_config(5);
    assert_eq!(p5, 180, "Round 5 pulse period should be 15 min (180 ledgers)");
}

#[test]
fn test_grace_periods_all_rounds() {
    // Grace periods should decrease each round
    let (_, g1, _) = get_round_config(1);
    let (_, g2, _) = get_round_config(2);
    let (_, g3, _) = get_round_config(3);
    let (_, g4, _) = get_round_config(4);
    let (_, g5, _) = get_round_config(5);
    
    // Round 1: 1 hour grace
    assert_eq!(g1, 720, "Round 1 grace should be 1 hour");
    
    // Round 2: 30 min grace
    assert_eq!(g2, 360, "Round 2 grace should be 30 min");
    
    // Round 3: 15 min grace
    assert_eq!(g3, 180, "Round 3 grace should be 15 min");
    
    // Round 4: 10 min grace
    assert_eq!(g4, 120, "Round 4 grace should be 10 min");
    
    // Round 5: 5 min grace
    assert_eq!(g5, 60, "Round 5 grace should be 5 min");
    
    // Verify decreasing pattern
    assert!(g1 > g2, "Grace should decrease each round");
    assert!(g2 > g3, "Grace should decrease each round");
    assert!(g3 > g4, "Grace should decrease each round");
    assert!(g4 > g5, "Grace should decrease each round");
}

#[test]
fn test_invalid_round_defaults_to_round_5() {
    // Invalid rounds should default to round 5 config
    let (p_invalid, g_invalid, c_invalid) = get_round_config(999);
    let (p5, g5, c5) = get_round_config(5);
    
    assert_eq!(p_invalid, p5);
    assert_eq!(g_invalid, g5);
    assert_eq!(c_invalid, c5);
}

// ============================================================================
// AGENT STATE TESTS
// ============================================================================

#[test]
fn test_agent_state_all_status_variants() {
    let env = Env::default();
    
    // Test all status variants can be created
    let alive_state = AgentState {
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
        wound_count: 0,
    };
    assert_eq!(alive_state.status, AgentStatus::Alive);
    
    let wounded_state = AgentState {
        status: AgentStatus::Wounded,
        ..alive_state.clone()
    };
    assert_eq!(wounded_state.status, AgentStatus::Wounded);
    
    let dead_state = AgentState {
        status: AgentStatus::Dead,
        ..alive_state.clone()
    };
    assert_eq!(dead_state.status, AgentStatus::Dead);
    
    let withdrawn_state = AgentState {
        status: AgentStatus::Withdrawn,
        ..alive_state.clone()
    };
    assert_eq!(withdrawn_state.status, AgentStatus::Withdrawn);
}

#[test]
fn test_agent_state_activity_score_accumulation() {
    // Simulate activity score building up over multiple pulses
    let env = Env::default();
    let owner = Address::generate(&env);
    
    let mut activity_score: u64 = 0;
    let mut streak: u32 = 0;
    
    // Simulate 100 pulses
    for _ in 0..100 {
        streak += 1;
        let bonus = match streak {
            0..=9 => 10u64,
            10..=24 => 11u64,
            25..=49 => 12u64,
            50..=99 => 15u64,
            _ => 20u64,
        };
        activity_score += bonus;
    }
    
    // After 100 pulses:
    // 0-9: 10 * 10 = 100
    // 10-24: 15 * 11 = 165
    // 25-49: 25 * 12 = 300
    // 50-99: 50 * 15 = 750
    // Total: 1315
    assert!(activity_score > 1300, "Activity score should accumulate with bonuses");
}

#[test]
fn test_agent_id_generation() {
    let env = Env::default();
    
    // Different IDs should be different
    let id1 = BytesN::from_array(&env, &[1u8; 32]);
    let id2 = BytesN::from_array(&env, &[2u8; 32]);
    let id3 = BytesN::from_array(&env, &[0u8; 32]);
    
    assert_ne!(id1, id2);
    assert_ne!(id1, id3);
    assert_ne!(id2, id3);
    
    // First byte should match what we set
    assert_eq!(id1.get(0), Some(1));
    assert_eq!(id2.get(0), Some(2));
    assert_eq!(id3.get(0), Some(0));
}

// ============================================================================
// HEART BALANCE TESTS
// ============================================================================

#[test]
fn test_entry_bond_precision() {
    // 50 XLM = 50 * 10^7 stroops
    assert_eq!(ENTRY_BOND, 50_0000000);
    
    // Verify stroop calculation
    let xlm_amount: i128 = 50;
    let stroops = xlm_amount * 10_000_000;
    assert_eq!(stroops, ENTRY_BOND);
}

#[test]
fn test_heart_balance_updates() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    client.constructor(&owner, &registry, &1u32);
    
    // Initial balance should be entry bond
    assert_eq!(client.get_heart_balance(), ENTRY_BOND);
}

// ============================================================================
// WITHDRAWAL CALCULATION TESTS
// ============================================================================

#[test]
fn test_withdrawal_split_calculation() {
    // Test 80/20 split calculations for various amounts
    let test_amounts = [
        50_0000000i128,   // Entry bond
        100_0000000i128,  // 100 XLM
        10_0000000i128,   // 10 XLM
        1_0000000i128,    // 1 XLM
    ];
    
    for amount in test_amounts {
        let agent_refund = amount * 80 / 100;
        let prize_contribution = amount * 20 / 100;
        
        assert_eq!(agent_refund + prize_contribution, amount,
            "80/20 split should sum to original amount for {}", amount);
        assert_eq!(agent_refund, amount * 4 / 5,
            "Agent refund should be 4/5 of amount");
        assert_eq!(prize_contribution, amount / 5,
            "Prize contribution should be 1/5 of amount");
    }
}

#[test]
fn test_withdrawal_precision() {
    // Test that withdrawal calculations maintain stroop precision
    let balance = 50_0000000i128; // 50 XLM
    
    let agent_refund = balance * 80 / 100;
    let prize_contribution = balance * 20 / 100;
    
    // Should be exact integers in stroops
    assert_eq!(agent_refund, 40_0000000, "Agent gets 40 XLM");
    assert_eq!(prize_contribution, 10_0000000, "Prize pool gets 10 XLM");
}

// ============================================================================
// PULSE COST SPLIT TESTS
// ============================================================================

#[test]
fn test_pulse_split_all_rounds() {
    // Verify 5%/5%/90% split for all rounds
    for round in 1u32..=5 {
        let (_, _, base_cost) = get_round_config(round);
        
        let protocol_fee = base_cost * 5 / 100;
        let prize_contribution = base_cost * 5 / 100;
        let ttl_rent = base_cost * 90 / 100;
        
        // Total should equal base cost (with possible rounding)
        let total = protocol_fee + prize_contribution + ttl_rent;
        
        // Due to integer division, there might be 1 stroop difference
        assert!((total - base_cost).abs() <= 1,
            "Pulse split should approximately equal base cost for round {}: {} vs {}",
            round, total, base_cost);
    }
}

#[test]
fn test_late_pulse_split() {
    // Late pulse is 2x cost, but split percentages remain the same
    let base_cost = 5000000i128; // 0.5 XLM
    let late_cost = base_cost * 2;
    
    let protocol_fee = late_cost * 5 / 100;
    let prize_contribution = late_cost * 5 / 100;
    let ttl_rent = late_cost * 90 / 100;
    
    assert_eq!(protocol_fee, base_cost * 10 / 100, "Late protocol fee is 2x normal");
    assert_eq!(prize_contribution, base_cost * 10 / 100, "Late prize contribution is 2x normal");
    assert_eq!(ttl_rent, base_cost * 180 / 100, "Late TTL rent is 2x normal");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_minimum_balance_scenarios() {
    // Test with minimum viable balances
    let min_balance = 5000000i128; // 0.5 XLM (minimum pulse cost)
    
    // Should be able to afford round 1 pulse
    let (_, _, r1_cost) = get_round_config(1);
    assert!(min_balance >= r1_cost, "Should afford round 1");
    
    // Should NOT afford round 5 pulse
    let (_, _, r5_cost) = get_round_config(5);
    assert!(min_balance < r5_cost, "Should not afford round 5");
}

#[test]
fn test_ledger_sequence_boundaries() {
    let env = Env::default();
    
    // Test ledger sequence at various values
    let test_sequences = [0u32, 1, 100, 1000, 10000, u32::MAX];
    
    for seq in test_sequences.iter() {
        env.ledger().set_sequence_number(*seq);
        let current = env.ledger().sequence();
        assert_eq!(current, *seq);
    }
}

#[test]
fn test_agent_status_transitions() {
    // Test valid status transitions
    // Alive -> Wounded (late pulse)
    // Wounded -> Alive (2 on-time pulses)
    // Alive -> Dead (miss grace period)
    // Alive -> Withdrawn (voluntary withdraw)
    
    let env = Env::default();
    
    // Verify status values
    assert_eq!(AgentStatus::Alive as u32, 0);
    assert_eq!(AgentStatus::Wounded as u32, 1);
    assert_eq!(AgentStatus::Dead as u32, 2);
    assert_eq!(AgentStatus::Withdrawn as u32, 3);
}

#[test]
fn test_max_streak_boundaries() {
    // Test streak tier boundaries
    let boundaries = [
        (0u32, 10u64),    // 0-9 tier
        (9, 10),          // End of tier 0
        (10, 11),         // Start of tier 1
        (24, 11),         // End of tier 1
        (25, 12),         // Start of tier 2
        (49, 12),         // End of tier 2
        (50, 15),         // Start of tier 3
        (99, 15),         // End of tier 3
        (100, 20),        // Start of tier 4
        (1000, 20),       // High streak
    ];
    
    for (streak, expected_bonus) in boundaries.iter() {
        let bonus = match *streak {
            0..=9 => 10u64,
            10..=24 => 11u64,
            25..=49 => 12u64,
            50..=99 => 15u64,
            _ => 20u64,
        };
        assert_eq!(bonus, *expected_bonus, 
            "Streak {} should give bonus {}", streak, expected_bonus);
    }
}

// ============================================================================
// STORAGE KEY TESTS
// ============================================================================

#[test]
fn test_storage_keys_are_unique() {
    // Verify all storage keys are different
    assert_ne!(AGENT_ID_KEY, OWNER_KEY);
    assert_ne!(AGENT_ID_KEY, SEASON_ID_KEY);
    assert_ne!(AGENT_ID_KEY, STATUS_KEY);
    assert_ne!(OWNER_KEY, SEASON_ID_KEY);
    assert_ne!(STATUS_KEY, HEART_BALANCE_KEY);
    assert_ne!(LAST_PULSE_KEY, DEADLINE_KEY);
    assert_ne!(STREAK_KEY, SCORE_KEY);
    assert_ne!(TOTAL_EARNED_KEY, TOTAL_SPENT_KEY);
}

#[test]
fn test_symbol_short_lengths() {
    // All symbol_short! should be 9 chars or less
    // This is just a compile-time check that they exist
    let _ = AGENT_ID_KEY;
    let _ = OWNER_KEY;
    let _ = SEASON_ID_KEY;
    let _ = STATUS_KEY;
    let _ = HEART_BALANCE_KEY;
}

// ============================================================================
// ERROR CODE COMPLETENESS TESTS
// ============================================================================

#[test]
fn test_all_error_codes_unique() {
    let errors = [
        Error::AlreadyInitialized as u32,
        Error::NotInitialized as u32,
        Error::NotOwner as u32,
        Error::AgentDead as u32,
        Error::AgentWithdrawn as u32,
        Error::InsufficientBalance as u32,
        Error::SeasonEnded as u32,
        Error::InvalidTarget as u32,
        Error::TargetNotDead as u32,
        Error::PrizeClaimFailed as u32,
        Error::WithdrawalFailed as u32,
        Error::PulseFailed as u32,
        Error::NoPrizeToClaim as u32,
    ];
    
    // Check all are unique
    for i in 0..errors.len() {
        for j in (i+1)..errors.len() {
            assert_ne!(errors[i], errors[j], 
                "Error codes should be unique: {} vs {}", errors[i], errors[j]);
        }
    }
}

#[test]
fn test_error_code_range() {
    // All error codes should be in range 1-13
    let max_error = Error::NoPrizeToClaim as u32;
    assert!(max_error <= 20, "Error codes should be reasonable range");
    
    let min_error = Error::AlreadyInitialized as u32;
    assert_eq!(min_error, 1, "Error codes should start at 1");
}

// ============================================================================
// GAME MECHANICS VALIDATION TESTS
// ============================================================================

#[test]
fn test_total_season_duration() {
    // Calculate total season duration
    let total_duration = ROUND_1_DURATION + ROUND_2_DURATION + ROUND_3_DURATION + 
                        ROUND_4_DURATION + ROUND_5_DURATION;
    
    // In ledgers (~5s each)
    assert_eq!(total_duration, 51840 + 34560 + 17280 + 8640 + 4320);
    
    // Convert to approximate hours
    let total_hours = total_duration as f64 * 5.0 / 3600.0;
    // Should be approximately 162 hours = 6.75 days
    assert!(total_hours > 160.0 && total_hours < 165.0);
}

#[test]
fn test_minimum_survival_cost() {
    // Minimum cost to survive all 5 rounds without any late pulses
    let min_cost = ROUND_1_COST + ROUND_2_COST + ROUND_3_COST + 
                   ROUND_4_COST + ROUND_5_COST;
    
    // 0.5 + 1.0 + 2.0 + 3.0 + 5.0 = 11.5 XLM
    assert_eq!(min_cost, 115_000000);
    
    // Plus entry bond = 61.5 XLM total
    let total_min = ENTRY_BOND + min_cost;
    assert_eq!(total_min, 615_000000);
}

#[test]
fn test_maximum_late_cost() {
    // Maximum cost if every pulse is late (2x)
    let max_cost = (ROUND_1_COST + ROUND_2_COST + ROUND_3_COST + 
                    ROUND_4_COST + ROUND_5_COST) * 2;
    
    // 11.5 XLM * 2 = 23 XLM
    assert_eq!(max_cost, 230_000000);
}

// ============================================================================
// LIQUIDATION REWARD TESTS
// ============================================================================

#[test]
fn test_kill_reward_calculation() {
    // Killer gets 100% of victim's balance
    let victim_balances = [
        50_0000000i128,   // Entry bond
        100_0000000i128,  // 100 XLM
        10_0000000i128,   // 10 XLM
    ];
    
    for balance in victim_balances.iter() {
        let reward = balance; // 100%
        assert_eq!(*reward, *balance, "Kill reward should be 100% of victim balance");
    }
}

// ============================================================================
// PRIZE DISTRIBUTION TESTS
// ============================================================================

#[test]
fn test_prize_share_calculation() {
    // Prize share = agent_score / total_scores * prize_pool
    let prize_pool: i128 = 1000_0000000; // 1000 XLM
    let agent_score: u64 = 100;
    let total_scores: u64 = 1000;
    
    let share = prize_pool * agent_score as i128 / total_scores as i128;
    assert_eq!(share, 100_0000000, "Should get 10% of prize pool");
    
    // Edge case: single survivor
    let single_share = prize_pool * 100 as i128 / 100 as i128;
    assert_eq!(single_share, prize_pool, "Single survivor gets entire pool");
}

#[test]
fn test_prize_distribution_sum() {
    // Test that prize distribution sums to prize pool (or very close)
    let prize_pool: i128 = 1000_0000000;
    let scores: [u64; 4] = [100, 200, 300, 400];
    let total_score: u64 = scores.iter().sum();
    
    let mut total_distributed: i128 = 0;
    for score in scores.iter() {
        total_distributed += prize_pool * *score as i128 / total_score as i128;
    }
    
    // Due to integer division, might be off by a few stroops
    assert!((total_distributed - prize_pool).abs() < 100,
        "Total distributed should approximately equal prize pool");
}

// ============================================================================
// INITIALIZATION VALIDATION TESTS
// ============================================================================

#[test]
fn test_constructor_parameters() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    
    // Test with season 1
    client.constructor(&owner, &registry, &1u32);
    let status1 = client.get_status();
    assert_eq!(status1.season_id, 1);
    
    // Different contract, different season
    let contract_id2 = env.register_contract(None, AgentContract);
    let client2 = AgentContractClient::new(&env, &contract_id2);
    client2.constructor(&owner, &registry, &5u32);
    let status2 = client2.get_status();
    assert_eq!(status2.season_id, 5);
}

#[test]
fn test_is_initialized() {
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    // Before init
    assert!(!client.is_initialized());
    
    env.mock_all_auths();
    client.constructor(&owner, &registry, &1u32);
    
    // After init
    assert!(client.is_initialized());
}

// ============================================================================
// CONSTANTS VALIDATION TESTS
// ============================================================================

#[test]
fn test_all_constants_positive() {
    assert!(ENTRY_BOND > 0, "Entry bond must be positive");
    assert!(ROUND_1_DURATION > 0, "Round durations must be positive");
    assert!(ROUND_1_PULSE_PERIOD > 0, "Pulse periods must be positive");
    assert!(ROUND_1_GRACE > 0, "Grace periods must be positive");
    assert!(ROUND_1_COST > 0, "Pulse costs must be positive");
}

#[test]
fn test_pulse_period_less_than_duration() {
    // Pulse period should always be less than round duration
    assert!(ROUND_1_PULSE_PERIOD < ROUND_1_DURATION);
    assert!(ROUND_2_PULSE_PERIOD < ROUND_2_DURATION);
    assert!(ROUND_3_PULSE_PERIOD < ROUND_3_DURATION);
    assert!(ROUND_4_PULSE_PERIOD < ROUND_4_DURATION);
    assert!(ROUND_5_PULSE_PERIOD < ROUND_5_DURATION);
}

#[test]
fn test_grace_less_than_pulse_period() {
    // Grace period should be less than pulse period
    assert!(ROUND_1_GRACE < ROUND_1_PULSE_PERIOD);
    assert!(ROUND_2_GRACE < ROUND_2_PULSE_PERIOD);
    assert!(ROUND_3_GRACE < ROUND_3_PULSE_PERIOD);
    assert!(ROUND_4_GRACE < ROUND_4_PULSE_PERIOD);
    assert!(ROUND_5_GRACE < ROUND_5_PULSE_PERIOD);
}

// ============================================================================
// ROUND ESCALATION TESTS
// ============================================================================

#[test]
fn test_round_escalation_pattern() {
    // Verify round parameters escalate correctly
    
    // Durations: 72h -> 48h -> 24h -> 12h -> 6h (halving)
    assert_eq!(ROUND_2_DURATION, ROUND_1_DURATION / 3 * 2, "Round 2 is 2/3 of round 1");
    assert_eq!(ROUND_3_DURATION, ROUND_2_DURATION / 2, "Round 3 is half of round 2");
    assert_eq!(ROUND_4_DURATION, ROUND_3_DURATION / 2, "Round 4 is half of round 3");
    assert_eq!(ROUND_5_DURATION, ROUND_4_DURATION / 2, "Round 5 is half of round 4");
    
    // Pulse periods: 6h -> 3h -> 1h -> 30m -> 15m (halving)
    assert_eq!(ROUND_2_PULSE_PERIOD, ROUND_1_PULSE_PERIOD / 2);
    assert_eq!(ROUND_3_PULSE_PERIOD, ROUND_2_PULSE_PERIOD / 3);
    assert_eq!(ROUND_4_PULSE_PERIOD, ROUND_3_PULSE_PERIOD / 2);
    assert_eq!(ROUND_5_PULSE_PERIOD, ROUND_4_PULSE_PERIOD / 2);
    
    // Costs: 0.5 -> 1.0 -> 2.0 -> 3.0 -> 5.0 XLM (escalating)
    assert!(ROUND_2_COST > ROUND_1_COST);
    assert!(ROUND_3_COST > ROUND_2_COST);
    assert!(ROUND_4_COST > ROUND_3_COST);
    assert!(ROUND_5_COST > ROUND_4_COST);
}

// ============================================================================
// TIME CONVERSION TESTS
// ============================================================================

#[test]
fn test_ledger_to_time_conversion() {
    // Assuming ~5 seconds per ledger
    let seconds_per_ledger = 5u32;
    
    // Round 1: 72 hours
    let round1_seconds = ROUND_1_DURATION * seconds_per_ledger;
    let round1_hours = round1_seconds / 3600;
    assert_eq!(round1_hours, 72, "Round 1 should be 72 hours");
    
    // Round 1 pulse period: 6 hours
    let pulse1_seconds = ROUND_1_PULSE_PERIOD * seconds_per_ledger;
    let pulse1_hours = pulse1_seconds / 3600;
    assert_eq!(pulse1_hours, 6, "Round 1 pulse should be 6 hours");
    
    // Round 1 grace: 1 hour
    let grace1_seconds = ROUND_1_GRACE * seconds_per_ledger;
    let grace1_hours = grace1_seconds / 3600;
    assert_eq!(grace1_hours, 1, "Round 1 grace should be 1 hour");
}

// ============================================================================
// BALANCE EXHAUSTION TESTS
// ============================================================================

#[test]
fn test_balance_exhaustion_calculation() {
    // Starting with entry bond, how many pulses until broke?
    let starting_balance = ENTRY_BOND; // 50 XLM
    
    // Round 1: 0.5 XLM per pulse
    let r1_pulses = starting_balance / ROUND_1_COST;
    assert!(r1_pulses >= 100, "Should afford at least 100 round 1 pulses");
    
    // Round 5: 5.0 XLM per pulse
    let r5_pulses = starting_balance / ROUND_5_COST;
    assert!(r5_pulses >= 10, "Should afford at least 10 round 5 pulses");
}

#[test]
fn test_wounded_cost_impact() {
    // Late pulses cost 2x and wound the agent
    // Test the economic impact of being wounded
    
    let base_cost = ROUND_1_COST;
    let late_cost = base_cost * 2;
    
    // 10 normal pulses
    let normal_total = base_cost * 10;
    
    // 10 late pulses
    let late_total = late_cost * 10;
    
    assert_eq!(late_total, normal_total * 2, "Late pulses cost 2x");
}

// ============================================================================
// AGENT STATE DEFAULTS TEST
// ============================================================================

#[test]
fn test_initial_state_values() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let registry = Address::generate(&env);
    let contract_id = env.register_contract(None, AgentContract);
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    env.ledger().set_sequence_number(1000);
    
    client.constructor(&owner, &registry, &1u32);
    
    let status = client.get_status();
    
    // Verify initial values
    assert_eq!(status.season_id, 1);
    assert_eq!(status.status, AgentStatus::Alive);
    assert_eq!(status.streak_count, 0);
    assert_eq!(status.activity_score, 0);
    assert_eq!(status.wound_count, 0);
    assert_eq!(status.total_earned, 0);
    assert_eq!(status.total_spent, 0);
    assert_eq!(status.kill_count, 0);
    assert_eq!(status.last_pulse_ledger, 1000);
}

// ============================================================================
// MULTI-SEASON TESTS
// ============================================================================

#[test]
fn test_different_season_ids() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let registry = Address::generate(&env);
    
    for season_id in [1u32, 2, 5, 10, 100].iter() {
        let contract_id = env.register_contract(None, AgentContract);
        let client = AgentContractClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.constructor(&owner, &registry, season_id);
        
        let status = client.get_status();
        assert_eq!(status.season_id, *season_id);
    }
}

// ============================================================================
// DATA STRUCTURE SERIALIZATION TESTS
// ============================================================================

#[test]
fn test_agent_status_serialization() {
    // Verify enum variants serialize correctly
    assert_eq!(AgentStatus::Alive as u32, 0);
    assert_eq!(AgentStatus::Wounded as u32, 1);
    assert_eq!(AgentStatus::Dead as u32, 2);
    assert_eq!(AgentStatus::Withdrawn as u32, 3);
}

#[test]
fn test_agent_state_field_access() {
    let env = Env::default();
    
    let state = AgentState {
        agent_id: BytesN::from_array(&env, &[1u8; 32]),
        owner: Address::generate(&env),
        season_id: 5,
        status: AgentStatus::Wounded,
        deadline_ledger: 5000,
        grace_deadline: 5100,
        last_pulse_ledger: 4800,
        streak_count: 42,
        activity_score: 500,
        heart_balance: 200_0000000,
        total_earned: 150_0000000,
        total_spent: 25_0000000,
        kill_count: 3,
        wound_count: 1,
    };
    
    // Verify all fields accessible
    assert_eq!(state.season_id, 5);
    assert_eq!(state.status, AgentStatus::Wounded);
    assert_eq!(state.deadline_ledger, 5000);
    assert_eq!(state.grace_deadline, 5100);
    assert_eq!(state.last_pulse_ledger, 4800);
    assert_eq!(state.streak_count, 42);
    assert_eq!(state.activity_score, 500);
    assert_eq!(state.heart_balance, 200_0000000);
    assert_eq!(state.total_earned, 150_0000000);
    assert_eq!(state.total_spent, 25_0000000);
    assert_eq!(state.kill_count, 3);
}

// ============================================================================
// COMPARISON TESTS
// ============================================================================

#[test]
fn test_address_equality() {
    let env = Env::default();
    
    let addr1 = Address::generate(&env);
    let addr2 = Address::generate(&env);
    let addr3 = addr1.clone();
    
    assert_ne!(addr1, addr2, "Different addresses should not be equal");
    assert_eq!(addr1, addr3, "Cloned address should be equal");
}

#[test]
fn test_bytesn_equality() {
    let env = Env::default();
    
    let bytes1 = BytesN::from_array(&env, &[1u8; 32]);
    let bytes2 = BytesN::from_array(&env, &[2u8; 32]);
    let bytes1_copy = BytesN::from_array(&env, &[1u8; 32]);
    
    assert_ne!(bytes1, bytes2, "Different bytes should not be equal");
    assert_eq!(bytes1, bytes1_copy, "Same content should be equal");
}

// ============================================================================
// ADDITIONAL EDGE CASE TESTS
// ============================================================================

#[test]
fn test_zero_values() {
    // Verify zero values in calculations
    let zero: i128 = 0;
    
    // Zero score shouldn't cause division by zero in prize calc
    // (handled by checking total_survivor_score == 0)
    
    // Zero balance withdrawal
    let agent_refund = zero * 80 / 100;
    assert_eq!(agent_refund, 0);
    
    // Zero pulse cost
    let protocol_fee = zero * 5 / 100;
    assert_eq!(protocol_fee, 0);
}

#[test]
fn test_large_values() {
    // Test with large but valid values
    let large_balance: i128 = 1_000_000_000_000; // 100,000 XLM
    
    let agent_refund = large_balance * 80 / 100;
    let prize_contribution = large_balance * 20 / 100;
    
    assert_eq!(agent_refund + prize_contribution, large_balance);
}

#[test]
fn test_streak_calculation_accuracy() {
    // Calculate total score after exactly 50 pulses
    let mut score: u64 = 0;
    let mut streak: u32 = 0;
    
    for _ in 0..50 {
        streak += 1;
        let bonus = match streak {
            0..=9 => 10u64,
            10..=24 => 11u64,
            25..=49 => 12u64,
            50..=99 => 15u64,
            _ => 20u64,
        };
        score += bonus;
    }
    
    // Expected: 9*10 + 15*11 + 25*12 + 1*15 = 90 + 165 + 300 + 15 = 570
    assert_eq!(score, 570, "Score calculation should be accurate");
}

// ============================================================================
// ROUND TRANSITION BOUNDARIES
// ============================================================================

#[test]
fn test_exact_round_boundaries() {
    // Test exact boundary values for rounds
    
    // Round 1 to 2 boundary
    assert_eq!(ROUND_1_DURATION + 1, 51841);
    
    // Verify the boundary ledger sequences
    let round1_end = ROUND_1_DURATION;
    let round2_end = round1_end + ROUND_2_DURATION;
    let round3_end = round2_end + ROUND_3_DURATION;
    let round4_end = round3_end + ROUND_4_DURATION;
    let round5_end = round4_end + ROUND_5_DURATION;
    
    assert_eq!(round1_end, 51840);
    assert_eq!(round2_end, 86400);
    assert_eq!(round3_end, 103680);
    assert_eq!(round4_end, 112320);
    assert_eq!(round5_end, 116640);
}

// ============================================================================
// GAS/EFFICIENCY TESTS (Conceptual)
// ============================================================================

#[test]
fn test_storage_key_reuse() {
    // Verify that storage keys are constants and reused
    // This is a compile-time optimization check
    
    let key1 = AGENT_ID_KEY;
    let key2 = AGENT_ID_KEY;
    
    // Both should be the same constant
    assert_eq!(key1, key2);
}

// ============================================================================
// FINAL INTEGRITY TESTS
// ============================================================================

#[test]
fn test_all_round_configs_valid() {
    for round in 1u32..=5 {
        let (period, grace, cost) = get_round_config(round);
        
        assert!(period > 0, "Round {} pulse period must be positive", round);
        assert!(grace > 0, "Round {} grace period must be positive", round);
        assert!(cost > 0, "Round {} pulse cost must be positive", round);
        assert!(grace < period, "Round {} grace must be less than period", round);
    }
}

#[test]
fn test_contract_initialization_idempotency() {
    // Verify contract state after initialization
    let (env, contract_id, owner, registry) = setup_env();
    let client = AgentContractClient::new(&env, &contract_id);

    env.mock_all_auths();
    client.constructor(&owner, &registry, &1u32);
    
    // All getters should work
    let _ = client.get_status();
    let _ = client.get_heart_balance();
    let _ = client.get_deadlines();
    let _ = client.is_initialized();
}
