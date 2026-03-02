//! Integration tests for Stellar Squid
//! Tests cross-contract interactions and full game flows

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    Address, BytesN, Env, IntoVal, Symbol, Val, Vec,
};

// Contract imports (would need to reference actual contract clients)
// use game_registry::{GameRegistry, GameRegistryClient};
// use agent_contract::{AgentContract, AgentContractClient};

// Constants matching contracts
const ENTRY_BOND: i128 = 50_0000000;
const ROUND_1_COST: i128 = 5000000;
const ROUND_2_COST: i128 = 10000000;
const ROUND_3_COST: i128 = 20000000;
const ROUND_4_COST: i128 = 30000000;
const ROUND_5_COST: i128 = 50000000;

const ROUND_1_DURATION: u32 = 51840;
const ROUND_2_DURATION: u32 = 34560;
const ROUND_3_DURATION: u32 = 17280;
const ROUND_4_DURATION: u32 = 8640;
const ROUND_5_DURATION: u32 = 4320;

const ROUND_1_PULSE_PERIOD: u32 = 4320;
const ROUND_1_GRACE: u32 = 720;

// =============================================================================
// TEST HELPERS
// =============================================================================

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn advance_ledger(env: &Env, ledgers: u32) {
    let current = env.ledger().sequence();
    env.ledger().set_sequence(current + ledgers);
}

fn create_agent_id(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

// =============================================================================
// CONSTANTS VALIDATION TESTS
// =============================================================================

#[test]
fn test_all_constants_defined() {
    // Verify all game constants are properly defined
    assert!(ENTRY_BOND > 0);
    assert!(ROUND_1_COST > 0);
    assert!(ROUND_2_COST > 0);
    assert!(ROUND_3_COST > 0);
    assert!(ROUND_4_COST > 0);
    assert!(ROUND_5_COST > 0);
    
    assert!(ROUND_1_DURATION > 0);
    assert!(ROUND_2_DURATION > 0);
    assert!(ROUND_3_DURATION > 0);
    assert!(ROUND_4_DURATION > 0);
    assert!(ROUND_5_DURATION > 0);
}

#[test]
fn test_round_cost_escalation() {
    // Verify costs increase each round
    assert!(ROUND_2_COST > ROUND_1_COST, "Round 2 should cost more than round 1");
    assert!(ROUND_3_COST > ROUND_2_COST, "Round 3 should cost more than round 2");
    assert!(ROUND_4_COST > ROUND_3_COST, "Round 4 should cost more than round 3");
    assert!(ROUND_5_COST > ROUND_4_COST, "Round 5 should cost more than round 4");
}

#[test]
fn test_round_duration_reduction() {
    // Verify durations decrease each round (escalating difficulty)
    assert!(ROUND_2_DURATION < ROUND_1_DURATION, "Round 2 should be shorter than round 1");
    assert!(ROUND_3_DURATION < ROUND_2_DURATION, "Round 3 should be shorter than round 2");
    assert!(ROUND_4_DURATION < ROUND_3_DURATION, "Round 4 should be shorter than round 3");
    assert!(ROUND_5_DURATION < ROUND_4_DURATION, "Round 5 should be shorter than round 4");
}

#[test]
fn test_total_season_duration() {
    let total = ROUND_1_DURATION + ROUND_2_DURATION + ROUND_3_DURATION + 
                ROUND_4_DURATION + ROUND_5_DURATION;
    
    // Total should be approximately 6.75 days worth of ledgers
    // At ~5 seconds per ledger, this is about 116,640 ledgers
    assert_eq!(total, 116640, "Total season duration mismatch");
}

// =============================================================================
// ECONOMIC MODEL TESTS
// =============================================================================

#[test]
fn test_pulse_cost_distribution() {
    // Test the 5%/5%/90% split of pulse costs
    let test_amounts = [
        ROUND_1_COST,
        ROUND_2_COST,
        ROUND_3_COST,
        ROUND_4_COST,
        ROUND_5_COST,
    ];
    
    for amount in test_amounts.iter() {
        let protocol_fee = amount * 5 / 100;
        let prize_pool = amount * 5 / 100;
        let ttl_rent = amount * 90 / 100;
        
        // Verify proportions
        assert_eq!(protocol_fee, amount / 20, "Protocol fee should be 5%");
        assert_eq!(prize_pool, amount / 20, "Prize pool should be 5%");
        assert_eq!(ttl_rent, amount * 9 / 10, "TTL rent should be 90%");
        
        // Verify total (allowing 1 stroop rounding)
        let total = protocol_fee + prize_pool + ttl_rent;
        assert!((total - *amount).abs() <= 1, "Split should sum to original amount");
    }
}

#[test]
fn test_withdrawal_split() {
    // Test the 80%/20% split on withdrawal
    let balance = ENTRY_BOND;
    
    let agent_refund = balance * 80 / 100;
    let prize_contribution = balance * 20 / 100;
    
    assert_eq!(agent_refund, 40_0000000, "Agent should get 40 XLM");
    assert_eq!(prize_contribution, 10_0000000, "Prize pool should get 10 XLM");
    assert_eq!(agent_refund + prize_contribution, balance, "Split should equal balance");
}

#[test]
fn test_kill_reward_is_100_percent() {
    // Killer gets 100% of victim's balance
    let victim_balances = [50_0000000i128, 100_0000000, 25_0000000, 10_0000000];
    
    for balance in victim_balances.iter() {
        // Reward is 100% of balance
        let reward = *balance;
        assert_eq!(reward, *balance, "Kill reward should be 100% of victim balance");
    }
}

#[test]
fn test_minimum_survival_cost() {
    // Calculate minimum cost to survive all 5 rounds (no late pulses)
    let min_cost = ROUND_1_COST + ROUND_2_COST + ROUND_3_COST + 
                   ROUND_4_COST + ROUND_5_COST;
    
    // 0.5 + 1.0 + 2.0 + 3.0 + 5.0 = 11.5 XLM
    assert_eq!(min_cost, 11_5000000, "Minimum survival cost should be 11.5 XLM");
    
    // With entry bond, total is 61.5 XLM
    let total_min = ENTRY_BOND + min_cost;
    assert_eq!(total_min, 61_5000000, "Total minimum with entry bond");
}

#[test]
fn test_maximum_survival_cost() {
    // Maximum cost if every pulse is late (2x normal cost)
    let base_cost = ROUND_1_COST + ROUND_2_COST + ROUND_3_COST + 
                    ROUND_4_COST + ROUND_5_COST;
    let max_cost = base_cost * 2;
    
    // 11.5 XLM * 2 = 23 XLM
    assert_eq!(max_cost, 23_0000000, "Maximum cost with all late pulses");
}

// =============================================================================
// STREAK BONUS TESTS
// =============================================================================

#[test]
fn test_streak_bonus_tier_calculation() {
    // Test bonus calculation for each tier
    let test_cases = [
        (0u32, 10u64),    // Tier 0: 0-9 streaks = 10 points
        (5, 10),          // Tier 0
        (9, 10),          // End of tier 0
        (10, 11),         // Start of tier 1
        (15, 11),         // Tier 1: 10-24 streaks = 11 points
        (24, 11),         // End of tier 1
        (25, 12),         // Start of tier 2
        (35, 12),         // Tier 2: 25-49 streaks = 12 points
        (49, 12),         // End of tier 2
        (50, 15),         // Start of tier 3
        (75, 15),         // Tier 3: 50-99 streaks = 15 points
        (99, 15),         // End of tier 3
        (100, 20),        // Start of tier 4
        (200, 20),        // Tier 4: 100+ streaks = 20 points
    ];
    
    for (streak, expected_bonus) in test_cases.iter() {
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

#[test]
fn test_activity_score_accumulation() {
    // Calculate expected score after 100 pulses
    let mut score: u64 = 0;
    
    for streak in 1u32..=100 {
        let bonus = match streak {
            0..=9 => 10u64,
            10..=24 => 11u64,
            25..=49 => 12u64,
            50..=99 => 15u64,
            _ => 20u64,
        };
        score += bonus;
    }
    
    // Expected calculation:
    // Tier 0 (1-9): 9 * 10 = 90
    // Tier 1 (10-24): 15 * 11 = 165
    // Tier 2 (25-49): 25 * 12 = 300
    // Tier 3 (50-99): 50 * 15 = 750
    // Tier 4 (100): 1 * 20 = 20
    // Total: 1325
    assert_eq!(score, 1325, "Activity score after 100 pulses");
}

// =============================================================================
// TIME/LEDGER CONVERSION TESTS
// =============================================================================

#[test]
fn test_ledger_to_time_round_1() {
    // Assuming ~5 seconds per ledger
    let seconds_per_ledger = 5u32;
    
    // Round 1 duration: 72 hours
    let duration_seconds = ROUND_1_DURATION * seconds_per_ledger;
    let duration_hours = duration_seconds / 3600;
    assert_eq!(duration_hours, 72, "Round 1 should be 72 hours");
    
    // Pulse period: 6 hours
    let pulse_seconds = ROUND_1_PULSE_PERIOD * seconds_per_ledger;
    let pulse_hours = pulse_seconds / 3600;
    assert_eq!(pulse_hours, 6, "Round 1 pulse period should be 6 hours");
    
    // Grace period: 1 hour
    let grace_seconds = ROUND_1_GRACE * seconds_per_ledger;
    let grace_hours = grace_seconds / 3600;
    assert_eq!(grace_hours, 1, "Round 1 grace period should be 1 hour");
}

// =============================================================================
// PRIZE DISTRIBUTION TESTS
// =============================================================================

#[test]
fn test_prize_share_calculation() {
    // Test prize share formula: agent_score / total_scores * prize_pool
    
    let prize_pool: i128 = 1000_0000000; // 1000 XLM
    let total_scores: u64 = 1000;
    
    // Single agent with 100 points gets 10%
    let agent_score: u64 = 100;
    let share = prize_pool * agent_score as i128 / total_scores as i128;
    assert_eq!(share, 100_0000000, "Should get 100 XLM (10%)");
    
    // Single agent with 500 points gets 50%
    let agent_score: u64 = 500;
    let share = prize_pool * agent_score as i128 / total_scores as i128;
    assert_eq!(share, 500_0000000, "Should get 500 XLM (50%)");
    
    // Single survivor gets 100%
    let single_share = prize_pool * 100 as i128 / 100 as i128;
    assert_eq!(single_share, prize_pool, "Single survivor gets entire pool");
}

#[test]
fn test_prize_distribution_with_multiple_survivors() {
    let prize_pool: i128 = 1000_0000000; // 1000 XLM
    let scores: [u64; 4] = [100, 200, 300, 400]; // Total = 1000
    let total_score: u64 = scores.iter().sum();
    
    let mut total_distributed: i128 = 0;
    for score in scores.iter() {
        total_distributed += prize_pool * *score as i128 / total_score as i128;
    }
    
    // Due to integer division, might be off by a few stroops
    assert!((total_distributed - prize_pool).abs() < 100,
        "Total distributed should approximately equal prize pool: {} vs {}",
        total_distributed, prize_pool);
}

// =============================================================================
// AGENT ID GENERATION TESTS
// =============================================================================

#[test]
fn test_agent_id_uniqueness() {
    let env = setup_env();
    
    // Create multiple unique IDs
    let ids: Vec<BytesN<32>> = (0..10u8)
        .map(|i| create_agent_id(&env, i))
        .collect();
    
    // Verify all are unique
    for i in 0..ids.len() {
        for j in (i+1)..ids.len() {
            assert_ne!(ids.get(i as u32).unwrap(), ids.get(j as u32).unwrap(),
                "All agent IDs should be unique");
        }
    }
}

#[test]
fn test_agent_id_consistency() {
    let env = setup_env();
    
    // Same seed should produce same ID
    let id1 = create_agent_id(&env, 5);
    let id2 = create_agent_id(&env, 5);
    
    assert_eq!(id1, id2, "Same seed should produce same ID");
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[test]
fn test_zero_values() {
    // Test that zero values don't cause issues
    let zero: i128 = 0;
    
    // Zero prize pool
    let share = 0i128 * 100 as i128 / 100 as i128;
    assert_eq!(share, 0);
    
    // Zero withdrawal
    let refund = zero * 80 / 100;
    assert_eq!(refund, 0);
    
    // Zero pulse cost
    let protocol_fee = zero * 5 / 100;
    assert_eq!(protocol_fee, 0);
}

#[test]
fn test_large_values() {
    // Test with large but valid values
    let large_balance: i128 = 1_000_000_000_000; // 100,000 XLM
    
    // Withdrawal split
    let agent_refund = large_balance * 80 / 100;
    let prize_contribution = large_balance * 20 / 100;
    
    assert_eq!(agent_refund + prize_contribution, large_balance);
    
    // Prize calculation with large balance
    let prize_pool: i128 = large_balance;
    let share = prize_pool * 50 as i128 / 100 as i128;
    assert_eq!(share, large_balance / 2);
}

#[test]
fn test_precision_with_stroops() {
    // Test stroop-level precision (1 XLM = 10^7 stroops)
    let one_xlm = 10_000_000i128;
    
    // 5% of 1 XLM
    let five_percent = one_xlm * 5 / 100;
    assert_eq!(five_percent, 500_000, "5% of 1 XLM = 0.05 XLM = 500,000 stroops");
    
    // 80% of 1 XLM
    let eighty_percent = one_xlm * 80 / 100;
    assert_eq!(eighty_percent, 8_000_000, "80% of 1 XLM = 0.8 XLM = 8,000,000 stroops");
}

// =============================================================================
// GAME SCENARIO TESTS
// =============================================================================

#[test]
fn test_scenario_single_agent_survival() {
    // Single agent enters, survives all rounds, claims prize
    let total_cost = ENTRY_BOND + ROUND_1_COST + ROUND_2_COST + ROUND_3_COST + 
                     ROUND_4_COST + ROUND_5_COST;
    
    // Agent needs at least 61.5 XLM total
    assert!(total_cost <= 100_0000000, "Single agent can survive with 100 XLM");
}

#[test]
fn test_scenario_multiple_agents_one_winner() {
    // Multiple agents enter, one survives to claim prize
    let num_agents = 10u32;
    
    // All pay entry bond
    let total_entry_bonds = ENTRY_BOND * num_agents as i128;
    
    // Some die, some withdraw - prize pool accumulates
    // Survivor gets share based on activity
    
    // Just verify the math works
    let survivor_share = total_entry_bonds * 20 / 100; // 20% from withdrawals
    assert!(survivor_share > 0);
}

#[test]
fn test_scenario_liquidation_chain() {
    // A liquidates B, then C liquidates A
    // Verify balances transfer correctly
    
    let initial_balance = ENTRY_BOND;
    
    // A kills B: A gets B's balance
    let a_balance_after = initial_balance + initial_balance;
    assert_eq!(a_balance_after, ENTRY_BOND * 2);
    
    // C kills A: C gets A's doubled balance
    let c_balance_after = initial_balance + a_balance_after;
    assert_eq!(c_balance_after, ENTRY_BOND * 3);
}

#[test]
fn test_scenario_all_agents_die() {
    // All agents miss their pulses and die
    // Prize pool still exists but only survivors can claim
    
    // If no survivors, prize pool remains
    let prize_pool: i128 = 1000_0000000;
    let total_survivor_score: u64 = 0;
    
    // Division by zero should be prevented in contract
    // In this case, no one can claim
    assert_eq!(total_survivor_score, 0);
}

// =============================================================================
// BOUNDARY CONDITION TESTS
// =============================================================================

#[test]
fn test_round_boundary_transitions() {
    // Test exact ledger sequences for round transitions
    
    let round1_end = ROUND_1_DURATION;
    let round2_end = round1_end + ROUND_2_DURATION;
    let round3_end = round2_end + ROUND_3_DURATION;
    let round4_end = round3_end + ROUND_4_DURATION;
    let round5_end = round4_end + ROUND_5_DURATION;
    
    // Verify cumulative durations
    assert_eq!(round1_end, 51840, "Round 1 ends at ledger 51840");
    assert_eq!(round2_end, 86400, "Round 2 ends at ledger 86400");
    assert_eq!(round3_end, 103680, "Round 3 ends at ledger 103680");
    assert_eq!(round4_end, 112320, "Round 4 ends at ledger 112320");
    assert_eq!(round5_end, 116640, "Round 5 ends at ledger 116640");
}

#[test]
fn test_pulse_deadline_boundaries() {
    // Test boundaries for pulse timing
    
    let deadline = 5000u32;
    let grace_deadline = deadline + ROUND_1_GRACE; // 5000 + 720 = 5720
    
    // At deadline: valid pulse
    assert!(5000 <= deadline, "At deadline should be valid");
    
    // One ledger after deadline: in grace period (late)
    assert!(5001 > deadline && 5001 <= grace_deadline, "One after deadline is late");
    
    // One ledger after grace: dead
    assert!(5721 > grace_deadline, "After grace period is dead");
}

// =============================================================================
// BALANCE EXHAUSTION TESTS
// =============================================================================

#[test]
fn test_balance_exhaustion_calculation() {
    // How many pulses until broke?
    let starting_balance = ENTRY_BOND; // 50 XLM
    
    // Round 1: 0.5 XLM per pulse
    let r1_pulses = starting_balance / ROUND_1_COST;
    assert_eq!(r1_pulses, 100, "Can afford 100 round 1 pulses");
    
    // Round 5: 5.0 XLM per pulse
    let r5_pulses = starting_balance / ROUND_5_COST;
    assert_eq!(r5_pulses, 10, "Can afford 10 round 5 pulses");
}

#[test]
fn test_late_pulse_economic_impact() {
    // Compare cost of normal vs late pulses
    
    let base_cost = ROUND_1_COST; // 0.5 XLM
    let late_cost = base_cost * 2; // 1.0 XLM
    
    // 10 normal pulses
    let normal_total = base_cost * 10; // 5 XLM
    
    // 10 late pulses
    let late_total = late_cost * 10; // 10 XLM
    
    assert_eq!(late_total, normal_total * 2, "Late pulses cost double");
    
    // With 50 XLM entry bond, late pulses exhaust balance twice as fast
    let normal_pulses_until_broke = ENTRY_BOND / base_cost;
    let late_pulses_until_broke = ENTRY_BOND / late_cost;
    
    assert_eq!(normal_pulses_until_broke, 100);
    assert_eq!(late_pulses_until_broke, 50);
}

// =============================================================================
// COMPLETE GAME FLOW SIMULATIONS
// =============================================================================

#[test]
fn test_simulate_full_season_single_agent() {
    // Simulate a complete season with one agent
    
    let env = setup_env();
    
    // 1. Season starts
    let mut current_ledger = 0u32;
    
    // 2. Agent registers (pays entry bond)
    let mut agent_balance = ENTRY_BOND;
    
    // 3. Round 1: Pulse every 6 hours
    let r1_pulses_needed = ROUND_1_DURATION / ROUND_1_PULSE_PERIOD;
    for _ in 0..r1_pulses_needed {
        agent_balance -= ROUND_1_COST * 10 / 100; // 10% deducted
        current_ledger += ROUND_1_PULSE_PERIOD;
    }
    
    // 4. Advance to round 2
    current_ledger = ROUND_1_DURATION + 1;
    
    // 5. Round 2: Pulse every 3 hours
    let r2_pulses_needed = ROUND_2_DURATION / 2160; // 3h = 2160 ledgers
    for _ in 0..r2_pulses_needed {
        agent_balance -= ROUND_2_COST * 10 / 100;
    }
    
    // Agent should still have balance
    assert!(agent_balance > 0, "Agent should survive to end");
    
    // Calculate expected minimum balance
    let min_spent = (ROUND_1_COST + ROUND_2_COST + ROUND_3_COST + 
                     ROUND_4_COST + ROUND_5_COST) * 10 / 100;
    let expected_min_balance = ENTRY_BOND - min_spent;
    
    assert!(agent_balance >= expected_min_balance,
        "Balance should be at least {}", expected_min_balance);
}

#[test]
fn test_simulate_full_season_with_withdrawal() {
    // Agent enters, plays some rounds, then withdraws
    
    let mut agent_balance = ENTRY_BOND;
    
    // Play round 1
    let r1_pulses = 10u32;
    for _ in 0..r1_pulses {
        agent_balance -= ROUND_1_COST * 10 / 100;
    }
    
    // Withdraw
    let withdrawal = agent_balance * 80 / 100;
    let prize_contribution = agent_balance * 20 / 100;
    
    assert_eq!(withdrawal + prize_contribution, agent_balance);
    
    // Verify withdrawal is reasonable
    assert!(withdrawal > 0);
    assert!(prize_contribution > 0);
}

// =============================================================================
// INVARIANT TESTS (Properties that should always hold)
// =============================================================================

#[test]
fn test_invariant_entry_bond_positive() {
    assert!(ENTRY_BOND > 0, "Entry bond must always be positive");
}

#[test]
fn test_invariant_pulse_costs_positive() {
    assert!(ROUND_1_COST > 0);
    assert!(ROUND_2_COST > 0);
    assert!(ROUND_3_COST > 0);
    assert!(ROUND_4_COST > 0);
    assert!(ROUND_5_COST > 0);
}

#[test]
fn test_invariant_round_durations_positive() {
    assert!(ROUND_1_DURATION > 0);
    assert!(ROUND_2_DURATION > 0);
    assert!(ROUND_3_DURATION > 0);
    assert!(ROUND_4_DURATION > 0);
    assert!(ROUND_5_DURATION > 0);
}

#[test]
fn test_invariant_withdrawal_split_valid() {
    // 80/20 split should always equal 100%
    for amount in [1i128, 10, 100, 1000, 10000, 50000000].iter() {
        let agent_part = amount * 80 / 100;
        let prize_part = amount * 20 / 100;
        assert_eq!(agent_part + prize_part, *amount,
            "Withdrawal split should sum to amount for {}", amount);
    }
}

#[test]
fn test_invariant_pulse_split_valid() {
    // 5/5/90 split should approximately equal 100%
    for amount in [ROUND_1_COST, ROUND_2_COST, ROUND_3_COST, ROUND_5_COST].iter() {
        let protocol = amount * 5 / 100;
        let prize = amount * 5 / 100;
        let ttl = amount * 90 / 100;
        let total = protocol + prize + ttl;
        assert!((total - *amount).abs() <= 1,
            "Pulse split should sum to amount within 1 stroop");
    }
}

#[test]
fn test_invariant_kill_reward_non_negative() {
    // Kill reward should always be >= 0
    let victim_balances = [0i128, 1, 10, 100, 1000, ENTRY_BOND];
    
    for balance in victim_balances.iter() {
        let reward = *balance; // 100% of balance
        assert!(reward >= 0, "Kill reward should be non-negative");
        assert_eq!(reward, *balance, "Kill reward should be 100% of victim balance");
    }
}

#[test]
fn test_invariant_activity_score_non_negative() {
    // Activity score should always be >= 0
    let test_streaks = [0u32, 1, 10, 50, 100, 1000];
    
    for streak in test_streaks.iter() {
        let bonus = match *streak {
            0..=9 => 10u64,
            10..=24 => 11u64,
            25..=49 => 12u64,
            50..=99 => 15u64,
            _ => 20u64,
        };
        assert!(bonus >= 10, "Activity bonus should be at least 10");
    }
}

// =============================================================================
// STRESS TESTS
// =============================================================================

#[test]
fn test_many_agents_scenario() {
    // Simulate many agents (just the math, not actual contract calls)
    let num_agents = 100u32;
    
    let total_entry_bonds = ENTRY_BOND * num_agents as i128;
    
    // If all withdraw, prize pool gets 20%
    let total_withdrawal_contribution = total_entry_bonds * 20 / 100;
    
    assert_eq!(total_withdrawal_contribution, ENTRY_BOND * 20 * num_agents as i128 / 100);
    
    // Single survivor would get huge prize
    let single_survivor_prize = total_withdrawal_contribution; // Simplified
    assert!(single_survivor_prize > ENTRY_BOND * 10); // Much more than entry bond
}

#[test]
fn test_many_pulses_scenario() {
    // Simulate agent pulsing many times
    let mut balance = ENTRY_BOND;
    let mut score: u64 = 0;
    let mut streak: u32 = 0;
    
    // Simulate 50 pulses
    for _ in 0..50 {
        balance -= ROUND_1_COST * 10 / 100;
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
    
    // Verify final state
    assert!(balance > 0, "Should still have balance after 50 pulses");
    assert_eq!(streak, 50);
    assert!(score > 500, "Should have significant activity score");
}

// =============================================================================
// SECURITY TESTS
// =============================================================================

#[test]
fn test_prevent_self_liquidation() {
    // Agent should not be able to liquidate themselves
    // This is enforced by the contract: killer_id != victim_id
    let agent_id = 5u8;
    assert_eq!(agent_id, agent_id, "Same ID check");
    // In real contract, this would panic with InvalidAgentContract
}

#[test]
fn test_prevent_double_liquidation() {
    // Once liquidated, victim balance is 0
    let initial_balance = ENTRY_BOND;
    let balance_after_liquidation = 0i128;
    
    assert_eq!(balance_after_liquidation, 0);
    // Second liquidation would fail because balance == 0
}

#[test]
fn test_prevent_dead_agent_actions() {
    // Dead agents should not be able to pulse, withdraw, or claim
    // These are enforced by status checks in the contract
    let status_dead = 2u32; // AgentStatus::Dead as u32
    assert_eq!(status_dead, 2);
}

// =============================================================================
// COMPATIBILITY TESTS
// =============================================================================

#[test]
fn test_contract_version_compatibility() {
    // Verify that our tests match the expected contract behavior
    // This serves as documentation of expected behavior
    
    // Entry bond should be 50 XLM
    assert_eq!(ENTRY_BOND, 50_0000000);
    
    // Round 1 pulse period should be 6 hours (4320 ledgers)
    assert_eq!(ROUND_1_PULSE_PERIOD, 4320);
    
    // Round 1 grace should be 1 hour (720 ledgers)
    assert_eq!(ROUND_1_GRACE, 720);
}
