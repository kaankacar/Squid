#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Bytes,
    Env, Symbol, IntoVal, Val, Vec, xdr::ToXdr, vec,
};

// =============================================================================
// CONSTANTS
// =============================================================================

pub const ENTRY_BOND: i128 = 50_0000000; // 50 XLM in stroops

// Round configurations (ledger-based, ~5s per ledger)
pub const ROUND_1_PULSE_PERIOD: u32 = 4320; // 6h
pub const ROUND_2_PULSE_PERIOD: u32 = 2160; // 3h
pub const ROUND_3_PULSE_PERIOD: u32 = 720;  // 1h
pub const ROUND_4_PULSE_PERIOD: u32 = 360;  // 30min
pub const ROUND_5_PULSE_PERIOD: u32 = 180;  // 15min

pub const ROUND_1_GRACE: u32 = 720;  // 1h
pub const ROUND_2_GRACE: u32 = 360;  // 30min
pub const ROUND_3_GRACE: u32 = 180;  // 15min
pub const ROUND_4_GRACE: u32 = 120;  // 10min
pub const ROUND_5_GRACE: u32 = 60;   // 5min

pub const ROUND_1_COST: i128 = 5000000;  // 0.5 XLM
pub const ROUND_2_COST: i128 = 10000000; // 1.0 XLM
pub const ROUND_3_COST: i128 = 20000000; // 2.0 XLM
pub const ROUND_4_COST: i128 = 30000000; // 3.0 XLM
pub const ROUND_5_COST: i128 = 50000000; // 5.0 XLM

// Storage keys
pub const AGENT_ID_KEY: Symbol = symbol_short!("AGENT_ID");
pub const OWNER_KEY: Symbol = symbol_short!("OWNER");
pub const SEASON_ID_KEY: Symbol = symbol_short!("SEASON");
pub const STATUS_KEY: Symbol = symbol_short!("STATUS");
pub const DEADLINE_KEY: Symbol = symbol_short!("DEADLINE");
pub const GRACE_KEY: Symbol = symbol_short!("GRACE");
pub const LAST_PULSE_KEY: Symbol = symbol_short!("LASTPULSE");
pub const STREAK_KEY: Symbol = symbol_short!("STREAK");
pub const SCORE_KEY: Symbol = symbol_short!("SCORE");
pub const REGISTRY_KEY: Symbol = symbol_short!("REGISTRY");
pub const HEART_BALANCE_KEY: Symbol = symbol_short!("HEART");
pub const TOTAL_EARNED_KEY: Symbol = symbol_short!("EARNED");
pub const TOTAL_SPENT_KEY: Symbol = symbol_short!("SPENT");
pub const KILL_COUNT_KEY: Symbol = symbol_short!("KILLS");
pub const CONSECUTIVE_PULSES_KEY: Symbol = symbol_short!("CONPULSES");

// =============================================================================
// DATA STRUCTURES
// =============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentStatus {
    Alive = 0,
    Wounded = 1,
    Dead = 2,
    Withdrawn = 3,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AgentState {
    pub agent_id: BytesN<32>,
    pub owner: Address,
    pub season_id: u32,
    pub status: AgentStatus,
    pub deadline_ledger: u32,
    pub grace_deadline: u32,
    pub last_pulse_ledger: u32,
    pub streak_count: u32,
    pub activity_score: u64,
    pub heart_balance: i128,
    pub total_earned: i128,
    pub total_spent: i128,
    pub kill_count: u32,
}

// =============================================================================
// ERRORS
// =============================================================================

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotOwner = 3,
    AgentDead = 4,
    AgentWithdrawn = 5,
    InsufficientBalance = 6,
    SeasonEnded = 7,
    InvalidTarget = 8,
    TargetNotDead = 9,
    PrizeClaimFailed = 10,
    WithdrawalFailed = 11,
    PulseFailed = 12,
    NoPrizeToClaim = 13,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn get_round_config(round: u32) -> (u32, u32, i128) {
    match round {
        1 => (ROUND_1_PULSE_PERIOD, ROUND_1_GRACE, ROUND_1_COST),
        2 => (ROUND_2_PULSE_PERIOD, ROUND_2_GRACE, ROUND_2_COST),
        3 => (ROUND_3_PULSE_PERIOD, ROUND_3_GRACE, ROUND_3_COST),
        4 => (ROUND_4_PULSE_PERIOD, ROUND_4_GRACE, ROUND_4_COST),
        5 => (ROUND_5_PULSE_PERIOD, ROUND_5_GRACE, ROUND_5_COST),
        _ => (ROUND_5_PULSE_PERIOD, ROUND_5_GRACE, ROUND_5_COST),
    }
}

// =============================================================================
// CONTRACT
// =============================================================================

#[contract]
pub struct AgentContract;

#[contractimpl]
impl AgentContract {
    // =========================================================================
    // INITIALIZATION
    // =========================================================================

    /// Initialize the agent contract
    /// Must be called once during deployment
    /// Note: Entry bond (50 XLM) should be transferred to contract before initialization
    pub fn constructor(
        env: Env,
        owner: Address,
        game_registry: Address,
        season_id: u32,
    ) -> Result<(), Error> {
        // Check if already initialized
        if env.storage().instance().has(&OWNER_KEY) {
            return Err(Error::AlreadyInitialized);
        }

        // Generate agent_id from contract address hash
        // We use a simple hash of the contract address as the agent_id
        let contract_addr_bytes: Bytes = env.current_contract_address().to_xdr(&env);
        let mut agent_id = BytesN::<32>::from_array(&env, &[0u8; 32]);
        let len = contract_addr_bytes.len().min(32);
        for i in 0..len {
            agent_id.set(i as u32, contract_addr_bytes.get(i as u32).unwrap_or(0));
        }

        // Store instance data
        env.storage().instance().set(&AGENT_ID_KEY, &agent_id);
        env.storage().instance().set(&OWNER_KEY, &owner);
        env.storage().instance().set(&SEASON_ID_KEY, &season_id);
        env.storage().instance().set(&REGISTRY_KEY, &game_registry);
        
        // Initialize status
        env.storage().instance().set(&STATUS_KEY, &AgentStatus::Alive);
        
        // Set initial deadlines
        let current_ledger = env.ledger().sequence();
        let (pulse_period, grace_period, _) = get_round_config(1);
        let deadline = current_ledger + pulse_period;
        let grace_deadline = deadline + grace_period;
        
        env.storage().instance().set(&DEADLINE_KEY, &deadline);
        env.storage().instance().set(&GRACE_KEY, &grace_deadline);
        env.storage().instance().set(&LAST_PULSE_KEY, &current_ledger);
        
        // Initialize counters
        env.storage().instance().set(&STREAK_KEY, &0u32);
        env.storage().instance().set(&SCORE_KEY, &0u64);
        env.storage().instance().set(&CONSECUTIVE_PULSES_KEY, &0u32);
        
        // Initialize persistent storage
        env.storage().persistent().set(&HEART_BALANCE_KEY, &ENTRY_BOND);
        env.storage().persistent().set(&TOTAL_EARNED_KEY, &0i128);
        env.storage().persistent().set(&TOTAL_SPENT_KEY, &0i128);
        env.storage().persistent().set(&KILL_COUNT_KEY, &0u32);

        Ok(())
    }

    // =========================================================================
    // CORE FUNCTIONS
    // =========================================================================

    /// Pulse - extend deadline and pay pulse cost
    /// Must be called before deadline to stay alive
    pub fn pulse(env: Env) -> Result<(), Error> {
        // Verify initialized
        let owner: Address = env.storage().instance().get(&OWNER_KEY).ok_or(Error::NotInitialized)?;
        owner.require_auth();

        // Check status
        let status: AgentStatus = env.storage().instance().get(&STATUS_KEY).unwrap();
        match status {
            AgentStatus::Dead => return Err(Error::AgentDead),
            AgentStatus::Withdrawn => return Err(Error::AgentWithdrawn),
            _ => {}
        }

        let current_ledger = env.ledger().sequence();
        let deadline: u32 = env.storage().instance().get(&DEADLINE_KEY).unwrap();
        let grace_deadline: u32 = env.storage().instance().get(&GRACE_KEY).unwrap();

        // Determine if pulse is late (within grace period)
        let is_late = current_ledger > deadline;
        let is_past_grace = current_ledger > grace_deadline;

        if is_past_grace {
            // Agent is dead - mark as dead
            env.storage().instance().set(&STATUS_KEY, &AgentStatus::Dead);
            
            // Notify registry
            let registry: Address = env.storage().instance().get(&REGISTRY_KEY).unwrap();
            let agent_id: BytesN<32> = env.storage().instance().get(&AGENT_ID_KEY).unwrap();
            
            let args: Vec<Val> = vec![&env, agent_id.into_val(&env)];
            let _: () = env.invoke_contract(
                &registry,
                &Symbol::new(&env, "mark_agent_dead"),
                args.into_val(&env),
            );
            
            return Err(Error::AgentDead);
        }

        // Get current round config (default to round 1 for now)
        let round: u32 = env.storage().instance().get(&Symbol::new(&env, "ROUND")).unwrap_or(1);
        let (pulse_period, grace_period, base_cost) = get_round_config(round);
        
        // Calculate pulse cost (2x if late)
        let pulse_cost = if is_late { base_cost * 2 } else { base_cost };

        // Check sufficient balance
        let heart_balance: i128 = env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap();
        if heart_balance < pulse_cost {
            return Err(Error::InsufficientBalance);
        }

        // Split pulse cost: 90% TTL, 5% protocol, 5% prize pool
        let protocol_fee = pulse_cost * 5 / 100;
        let _prize_contribution = pulse_cost * 5 / 100;
        let _ttl_rent = pulse_cost * 90 / 100;
        
        // Update heart balance (deduct pulse cost)
        let new_balance = heart_balance - pulse_cost;
        env.storage().persistent().set(&HEART_BALANCE_KEY, &new_balance);
        
        // Update total spent
        let total_spent: i128 = env.storage().persistent().get(&TOTAL_SPENT_KEY).unwrap();
        env.storage().persistent().set(&TOTAL_SPENT_KEY, &(total_spent + pulse_cost));

        // Update streak and status
        if is_late {
            // Late pulse: reset streak, mark wounded
            env.storage().instance().set(&STREAK_KEY, &0u32);
            env.storage().instance().set(&CONSECUTIVE_PULSES_KEY, &0u32);
            env.storage().instance().set(&STATUS_KEY, &AgentStatus::Wounded);
        } else {
            // On-time pulse: increment streak and score
            let streak: u32 = env.storage().instance().get(&STREAK_KEY).unwrap();
            let new_streak = streak + 1;
            env.storage().instance().set(&STREAK_KEY, &new_streak);
            
            // Track consecutive on-time pulses for wound healing
            let consecutive: u32 = env.storage().instance().get(&CONSECUTIVE_PULSES_KEY).unwrap_or(0);
            let new_consecutive = consecutive + 1;
            env.storage().instance().set(&CONSECUTIVE_PULSES_KEY, &new_consecutive);
            
            // Calculate score with streak bonus
            let streak_bonus = match new_streak {
                0..=9 => 10u64,
                10..=24 => 11u64,
                25..=49 => 12u64,
                50..=99 => 15u64,
                _ => 20u64,
            };
            
            let activity_score: u64 = env.storage().instance().get(&SCORE_KEY).unwrap();
            env.storage().instance().set(&SCORE_KEY, &(activity_score + streak_bonus));
            
            // Clear wounded status after 2 consecutive on-time pulses
            if status == AgentStatus::Wounded && new_consecutive >= 2 {
                env.storage().instance().set(&STATUS_KEY, &AgentStatus::Alive);
                env.storage().instance().set(&CONSECUTIVE_PULSES_KEY, &0u32);
            }
        }

        // Update deadlines
        let new_deadline = current_ledger + pulse_period;
        let new_grace_deadline = new_deadline + grace_period;
        env.storage().instance().set(&DEADLINE_KEY, &new_deadline);
        env.storage().instance().set(&GRACE_KEY, &new_grace_deadline);
        env.storage().instance().set(&LAST_PULSE_KEY, &current_ledger);

        // Notify GameRegistry of pulse
        let registry: Address = env.storage().instance().get(&REGISTRY_KEY).unwrap();
        let agent_id: BytesN<32> = env.storage().instance().get(&AGENT_ID_KEY).unwrap();
        
        let pulse_args: Vec<Val> = vec![
            &env,
            agent_id.into_val(&env),
            pulse_cost.into_val(&env),
            is_late.into_val(&env),
        ];
        let _: () = env.invoke_contract(
            &registry,
            &Symbol::new(&env, "update_agent_pulse"),
            pulse_args.into_val(&env),
        );

        // Transfer protocol fee to fee address
        if protocol_fee > 0 {
            let fee_addr: Address = env.invoke_contract(
                &registry,
                &Symbol::new(&env, "get_protocol_fee_address"),
                Vec::<Val>::new(&env).into_val(&env),
            );
            
            // For XLM transfers, we would use the native token contract
            // This is simplified - in production, use proper token contract calls
            let _ = fee_addr; // Mark as used for now
        }

        Ok(())
    }

    /// Liquidate a dead agent - claim 100% of their balance
    pub fn liquidate(env: Env, target_agent_id: BytesN<32>) -> Result<i128, Error> {
        // Verify caller is owner
        let owner: Address = env.storage().instance().get(&OWNER_KEY).ok_or(Error::NotInitialized)?;
        owner.require_auth();

        // Check this agent is still alive
        let status: AgentStatus = env.storage().instance().get(&STATUS_KEY).unwrap();
        if status == AgentStatus::Dead {
            return Err(Error::AgentDead);
        }
        if status == AgentStatus::Withdrawn {
            return Err(Error::AgentWithdrawn);
        }

        // Get registry
        let registry: Address = env.storage().instance().get(&REGISTRY_KEY).unwrap();
        
        // Get this agent's ID
        let agent_id: BytesN<32> = env.storage().instance().get(&AGENT_ID_KEY).unwrap();
        
        // Call registry to transfer kill reward
        let args: Vec<Val> = vec![
            &env,
            target_agent_id.into_val(&env),
            agent_id.into_val(&env),
        ];
        let reward: i128 = env.invoke_contract(
            &registry,
            &Symbol::new(&env, "transfer_kill_reward"),
            args.into_val(&env),
        );

        if reward <= 0 {
            return Err(Error::TargetNotDead);
        }

        // Update heart balance
        let heart_balance: i128 = env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap();
        env.storage().persistent().set(&HEART_BALANCE_KEY, &(heart_balance + reward));
        
        // Update total earned
        let total_earned: i128 = env.storage().persistent().get(&TOTAL_EARNED_KEY).unwrap();
        env.storage().persistent().set(&TOTAL_EARNED_KEY, &(total_earned + reward));
        
        // Update kill count
        let kill_count: u32 = env.storage().persistent().get(&KILL_COUNT_KEY).unwrap();
        env.storage().persistent().set(&KILL_COUNT_KEY, &(kill_count + 1));

        Ok(reward)
    }

    /// Withdraw - exit the game with 80% refund (20% to prize pool)
    pub fn withdraw(env: Env) -> Result<i128, Error> {
        // Verify owner
        let owner: Address = env.storage().instance().get(&OWNER_KEY).ok_or(Error::NotInitialized)?;
        owner.require_auth();

        // Check status
        let status: AgentStatus = env.storage().instance().get(&STATUS_KEY).unwrap();
        if status == AgentStatus::Dead {
            return Err(Error::AgentDead);
        }
        if status == AgentStatus::Withdrawn {
            return Err(Error::AgentWithdrawn);
        }

        // Get registry
        let registry: Address = env.storage().instance().get(&REGISTRY_KEY).unwrap();
        let agent_id: BytesN<32> = env.storage().instance().get(&AGENT_ID_KEY).unwrap();
        
        // Call registry to process withdrawal
        let args: Vec<Val> = vec![&env, agent_id.into_val(&env)];
        let refund: i128 = env.invoke_contract(
            &registry,
            &Symbol::new(&env, "process_withdrawal"),
            args.into_val(&env),
        );

        // Mark as withdrawn
        env.storage().instance().set(&STATUS_KEY, &AgentStatus::Withdrawn);

        // Transfer refund to owner
        // In production, this would use the native token contract
        // For now, we track the balance internally
        let _ = owner; // Mark as used

        Ok(refund)
    }

    /// Claim prize share at season end
    pub fn claim_prize(env: Env) -> Result<i128, Error> {
        // Verify owner
        let owner: Address = env.storage().instance().get(&OWNER_KEY).ok_or(Error::NotInitialized)?;
        owner.require_auth();

        // Check agent is still alive (only survivors can claim)
        let status: AgentStatus = env.storage().instance().get(&STATUS_KEY).unwrap();
        if status != AgentStatus::Alive && status != AgentStatus::Wounded {
            return Err(Error::NoPrizeToClaim);
        }

        // Get registry
        let registry: Address = env.storage().instance().get(&REGISTRY_KEY).unwrap();
        let agent_id: BytesN<32> = env.storage().instance().get(&AGENT_ID_KEY).unwrap();
        
        // Call registry to claim prize
        let args: Vec<Val> = vec![&env, agent_id.into_val(&env)];
        let prize: i128 = env.invoke_contract(
            &registry,
            &Symbol::new(&env, "claim_prize"),
            args.into_val(&env),
        );

        if prize <= 0 {
            return Err(Error::NoPrizeToClaim);
        }

        // Update heart balance
        let heart_balance: i128 = env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap();
        env.storage().persistent().set(&HEART_BALANCE_KEY, &(heart_balance + prize));
        
        // Update total earned
        let total_earned: i128 = env.storage().persistent().get(&TOTAL_EARNED_KEY).unwrap();
        env.storage().persistent().set(&TOTAL_EARNED_KEY, &(total_earned + prize));

        // Transfer prize to owner
        let _ = owner; // Mark as used - in production, transfer tokens

        Ok(prize)
    }

    // =========================================================================
    // READ-ONLY FUNCTIONS
    // =========================================================================

    /// Get full agent status
    pub fn get_status(env: Env) -> Result<AgentState, Error> {
        // Verify initialized
        if !env.storage().instance().has(&OWNER_KEY) {
            return Err(Error::NotInitialized);
        }

        Ok(AgentState {
            agent_id: env.storage().instance().get(&AGENT_ID_KEY).unwrap(),
            owner: env.storage().instance().get(&OWNER_KEY).unwrap(),
            season_id: env.storage().instance().get(&SEASON_ID_KEY).unwrap(),
            status: env.storage().instance().get(&STATUS_KEY).unwrap(),
            deadline_ledger: env.storage().instance().get(&DEADLINE_KEY).unwrap(),
            grace_deadline: env.storage().instance().get(&GRACE_KEY).unwrap(),
            last_pulse_ledger: env.storage().instance().get(&LAST_PULSE_KEY).unwrap(),
            streak_count: env.storage().instance().get(&STREAK_KEY).unwrap(),
            activity_score: env.storage().instance().get(&SCORE_KEY).unwrap(),
            heart_balance: env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap(),
            total_earned: env.storage().persistent().get(&TOTAL_EARNED_KEY).unwrap(),
            total_spent: env.storage().persistent().get(&TOTAL_SPENT_KEY).unwrap(),
            kill_count: env.storage().persistent().get(&KILL_COUNT_KEY).unwrap(),
        })
    }

    /// Get current heart balance
    pub fn get_heart_balance(env: Env) -> i128 {
        env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap_or(0)
    }

    /// Check if agent is initialized
    pub fn is_initialized(env: Env) -> bool {
        env.storage().instance().has(&OWNER_KEY)
    }

    /// Get current deadline info
    pub fn get_deadlines(env: Env) -> (u32, u32, u32) {
        let current_ledger = env.ledger().sequence();
        let deadline: u32 = env.storage().instance().get(&DEADLINE_KEY).unwrap_or(current_ledger);
        let grace: u32 = env.storage().instance().get(&GRACE_KEY).unwrap_or(current_ledger);
        (current_ledger, deadline, grace)
    }
}

mod test;
