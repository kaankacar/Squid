#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, symbol_short, Address, BytesN, Bytes,
    Env, Symbol, IntoVal, Val, Vec, xdr::ToXdr, vec,
};

// Import AgentSummary from game-registry crate
use game_registry::AgentSummary;

// =============================================================================
// CONSTANTS
// =============================================================================

pub const ENTRY_BOND: i128 = 50_0000000; // 50 XLM in stroops

// Round durations (in ledgers, assuming ~5s per ledger)
pub const ROUND_1_DURATION: u32 = 51840; // 72h = 51840 ledgers
pub const ROUND_2_DURATION: u32 = 34560; // 48h = 34560 ledgers
pub const ROUND_3_DURATION: u32 = 17280; // 24h = 17280 ledgers
pub const ROUND_4_DURATION: u32 = 8640;  // 12h = 8640 ledgers
pub const ROUND_5_DURATION: u32 = 4320;  // 6h = 4320 ledgers

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
pub const WOUND_COUNT_KEY: Symbol = symbol_short!("WOUNDS");

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
    pub wound_count: u32,
}

// =============================================================================
// EVENTS
// =============================================================================

/// Event emitted when agent contract is initialized
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentInitialized {
    pub agent_id: BytesN<32>,
    pub owner: Address,
    pub season_id: u32,
}

/// Event emitted when agent pulses
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentPulsed {
    pub is_late: bool,
    pub cost: i128,
    pub new_deadline: u32,
}

/// Event emitted when agent liquidates another agent
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentLiquidated {
    pub target_agent: BytesN<32>,
    pub reward: i128,
}

/// Event emitted when agent withdraws from the game
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentWithdrawn {
    pub refund: i128,
    pub prize_contribution: i128,
}

/// Legacy event structs for backward compatibility
#[contracttype]
#[derive(Clone, Debug)]
pub struct PulseEvent {
    pub ledger: u32,
    pub cost: i128,
    pub is_late: bool,
    pub new_balance: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct LiquidationEvent {
    pub target_agent_id: BytesN<32>,
    pub reward: i128,
    pub new_balance: i128,
    pub ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct WithdrawalEvent {
    pub refund: i128,
    pub ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PrizeClaimedEvent {
    pub prize_amount: i128,
    pub new_balance: i128,
    pub ledger: u32,
}

/// Event emitted when agent scans for targets
#[contracttype]
#[derive(Clone, Debug)]
pub struct ScanEvent {
    pub agent_id: BytesN<32>,
    pub dead_count: u32,
    pub vulnerable_count: u32,
    pub total_targets: u32,
    pub ledger: u32,
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
    Overflow = 14,         // NEW: Arithmetic overflow
    DivisionByZero = 15,   // NEW: Division by zero
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

    /// @notice Initialize the agent contract
    /// @dev Must be called once during deployment. Entry bond (50 XLM) should be transferred to contract before initialization.
    /// @param owner The address that owns this agent
    /// @param game_registry The address of the GameRegistry contract
    /// @param season_id The current season ID
    /// @return Result<(), Error> Returns Ok(()) on success
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
        env.storage().persistent().set(&WOUND_COUNT_KEY, &0u32);

        Ok(())
    }

    // =========================================================================
    // CORE FUNCTIONS
    // =========================================================================

    /// @notice Pulse - extend deadline and pay pulse cost
    /// @dev Must be called before deadline to stay alive. Cost is split 5%/5%/90% (protocol/prize/TTL).
    /// @return Result<(), Error> Returns Ok(()) on success
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
        
        // Calculate pulse cost (2x if late) with overflow protection
        let pulse_cost = if is_late { 
            base_cost.checked_mul(2).ok_or(Error::Overflow)? 
        } else { 
            base_cost 
        };

        // Check sufficient balance
        let heart_balance: i128 = env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap();
        if heart_balance < pulse_cost {
            return Err(Error::InsufficientBalance);
        }

        // Split pulse cost: 90% TTL, 5% protocol, 5% prize pool
        let protocol_fee = pulse_cost.checked_mul(5).ok_or(Error::Overflow)?
            .checked_div(100).ok_or(Error::DivisionByZero)?;
        let _prize_contribution = pulse_cost.checked_mul(5).ok_or(Error::Overflow)?
            .checked_div(100).ok_or(Error::DivisionByZero)?;
        let _ttl_rent = pulse_cost.checked_mul(90).ok_or(Error::Overflow)?
            .checked_div(100).ok_or(Error::DivisionByZero)?;
        
        // Update heart balance (deduct pulse cost) with overflow protection
        let new_balance = heart_balance.checked_sub(pulse_cost).ok_or(Error::Overflow)?;
        env.storage().persistent().set(&HEART_BALANCE_KEY, &new_balance);
        
        // Update total spent with overflow protection
        let total_spent: i128 = env.storage().persistent().get(&TOTAL_SPENT_KEY).unwrap();
        let new_total_spent = total_spent.checked_add(pulse_cost).ok_or(Error::Overflow)?;
        env.storage().persistent().set(&TOTAL_SPENT_KEY, &new_total_spent);

        // Update streak and status
        if is_late {
            // Late pulse: reset streak, mark wounded, increment wound count
            env.storage().instance().set(&STREAK_KEY, &0u32);
            env.storage().instance().set(&CONSECUTIVE_PULSES_KEY, &0u32);
            env.storage().instance().set(&STATUS_KEY, &AgentStatus::Wounded);
            
            // Increment wound count with overflow protection
            let wound_count: u32 = env.storage().persistent().get(&WOUND_COUNT_KEY).unwrap_or(0);
            let new_wound_count = wound_count.checked_add(1).ok_or(Error::Overflow)?;
            env.storage().persistent().set(&WOUND_COUNT_KEY, &new_wound_count);
        } else {
            // On-time pulse: increment streak and score
            let streak: u32 = env.storage().instance().get(&STREAK_KEY).unwrap();
            let new_streak = streak.checked_add(1).ok_or(Error::Overflow)?;
            env.storage().instance().set(&STREAK_KEY, &new_streak);
            
            // Track consecutive on-time pulses for wound healing
            let consecutive: u32 = env.storage().instance().get(&CONSECUTIVE_PULSES_KEY).unwrap_or(0);
            let new_consecutive = consecutive.checked_add(1).ok_or(Error::Overflow)?;
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
            let new_activity_score = activity_score.checked_add(streak_bonus).ok_or(Error::Overflow)?;
            env.storage().instance().set(&SCORE_KEY, &new_activity_score);
            
            // Clear wounded status after 2 consecutive on-time pulses
            if status == AgentStatus::Wounded && new_consecutive >= 2 {
                env.storage().instance().set(&STATUS_KEY, &AgentStatus::Alive);
                env.storage().instance().set(&CONSECUTIVE_PULSES_KEY, &0u32);
            }
        }

        // Update deadlines with overflow protection
        let new_deadline = current_ledger.checked_add(pulse_period).ok_or(Error::Overflow)?;
        let new_grace_deadline = new_deadline.checked_add(grace_period).ok_or(Error::Overflow)?;
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

        // Emit pulse event
        env.events().publish(
            (symbol_short!("PULSE"),),
            PulseEvent {
                ledger: current_ledger,
                cost: pulse_cost,
                is_late,
                new_balance,
            },
        );

        Ok(())
    }

    /// @notice Liquidate a dead agent - claim 100% of their balance
    /// @dev The target agent must be dead (missed their deadline + grace period)
    /// @param target_agent_id The unique identifier of the dead agent to liquidate
    /// @return Result<i128, Error> The reward amount received from liquidation
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

        // Update heart balance with overflow protection
        let heart_balance: i128 = env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap();
        let new_balance = heart_balance.checked_add(reward).ok_or(Error::Overflow)?;
        env.storage().persistent().set(&HEART_BALANCE_KEY, &new_balance);
        
        // Update total earned with overflow protection
        let total_earned: i128 = env.storage().persistent().get(&TOTAL_EARNED_KEY).unwrap();
        let new_total_earned = total_earned.checked_add(reward).ok_or(Error::Overflow)?;
        env.storage().persistent().set(&TOTAL_EARNED_KEY, &new_total_earned);
        
        // Update kill count with overflow protection
        let kill_count: u32 = env.storage().persistent().get(&KILL_COUNT_KEY).unwrap();
        let new_kill_count = kill_count.checked_add(1).ok_or(Error::Overflow)?;
        env.storage().persistent().set(&KILL_COUNT_KEY, &new_kill_count);

        // Emit liquidation event
        let current_ledger = env.ledger().sequence();
        env.events().publish(
            (symbol_short!("LIQUIDATE"), target_agent_id.clone()),
            LiquidationEvent {
                target_agent_id,
                reward,
                new_balance,
                ledger: current_ledger,
            },
        );

        Ok(reward)
    }

    /// @notice Withdraw - exit the game with 80% refund (20% to prize pool)
    /// @dev The agent must be alive. 20% of balance goes to prize pool as contribution.
    /// @return Result<i128, Error> The refund amount (80% of balance)
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

        // Calculate prize contribution (20% of original balance)
        let heart_balance: i128 = env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap();
        let prize_contribution = heart_balance.checked_mul(20).ok_or(Error::Overflow)?
            .checked_div(100).ok_or(Error::DivisionByZero)?;

        // Mark as withdrawn
        env.storage().instance().set(&STATUS_KEY, &AgentStatus::Withdrawn);

        // Transfer refund to owner
        // In production, this would use the native token contract
        // For now, we track the balance internally
        let _ = owner; // Mark as used

        // Emit withdrawal event
        let current_ledger = env.ledger().sequence();
        env.events().publish(
            (symbol_short!("WITHDRAW"),),
            WithdrawalEvent {
                refund,
                ledger: current_ledger,
            },
        );

        Ok(refund)
    }

    /// @notice Claim prize share at season end
    /// @dev Only survivors (alive agents) can claim. Prize is proportional to activity score.
    /// @return Result<i128, Error> The prize amount claimed
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

        // Update heart balance with overflow protection
        let heart_balance: i128 = env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap();
        let new_balance = heart_balance.checked_add(prize).ok_or(Error::Overflow)?;
        env.storage().persistent().set(&HEART_BALANCE_KEY, &new_balance);
        
        // Update total earned with overflow protection
        let total_earned: i128 = env.storage().persistent().get(&TOTAL_EARNED_KEY).unwrap();
        let new_total_earned = total_earned.checked_add(prize).ok_or(Error::Overflow)?;
        env.storage().persistent().set(&TOTAL_EARNED_KEY, &new_total_earned);

        // Transfer prize to owner
        let _ = owner; // Mark as used - in production, transfer tokens

        // Emit legacy prize claimed event (for backward compatibility)
        let current_ledger = env.ledger().sequence();
        env.events().publish(
            (symbol_short!("PRIZE"),),
            PrizeClaimedEvent {
                prize_amount: prize,
                new_balance,
                ledger: current_ledger,
            },
        );

        Ok(prize)
    }

    // =========================================================================
    // SCAN FUNCTION - Target Discovery
    // =========================================================================

    /// @notice Scan for liquidation targets
    /// @dev Queries GameRegistry for dead and vulnerable agents. This is the core discovery mechanism for the hunt loop.
    /// @return Vec<AgentSummary> Combined list of dead agents (immediate targets) and vulnerable agents (potential future targets)
    pub fn scan(env: Env) -> Result<Vec<AgentSummary>, Error> {
        // Verify initialized
        if !env.storage().instance().has(&OWNER_KEY) {
            return Err(Error::NotInitialized);
        }

        // Get registry address
        let registry: Address = env.storage().instance().get(&REGISTRY_KEY).ok_or(Error::NotInitialized)?;

        // Query GameRegistry for dead agents (immediate liquidation targets)
        let args: Vec<Val> = Vec::new(&env);
        let dead_agents: Vec<AgentSummary> = env.invoke_contract(
            &registry,
            &Symbol::new(&env, "get_dead_agents"),
            args.into_val(&env),
        );

        // Query GameRegistry for vulnerable agents (wounded, potential future targets)
        let args2: Vec<Val> = Vec::new(&env);
        let vulnerable_agents: Vec<AgentSummary> = env.invoke_contract(
            &registry,
            &Symbol::new(&env, "get_vulnerable_agents"),
            args2.into_val(&env),
        );

        // Combine results - dead agents first (priority targets), then vulnerable
        let mut targets = vec![&env];
        
        // Add dead agents first (immediate liquidation opportunities)
        for agent in dead_agents.iter() {
            targets.push_back(agent);
        }
        
        // Add vulnerable agents (track for potential future liquidation)
        for agent in vulnerable_agents.iter() {
            targets.push_back(agent);
        }

        // Emit scan event
        let agent_id: BytesN<32> = env.storage().instance().get(&AGENT_ID_KEY).unwrap();
        let current_ledger = env.ledger().sequence();
        env.events().publish(
            (symbol_short!("SCAN"),),
            ScanEvent {
                agent_id,
                dead_count: dead_agents.len(),
                vulnerable_count: vulnerable_agents.len(),
                total_targets: targets.len(),
                ledger: current_ledger,
            },
        );

        Ok(targets)
    }

    // =========================================================================
    // READ-ONLY FUNCTIONS
    // =========================================================================

    /// @notice Get full agent status
    /// @return Result<AgentState, Error> The complete state of the agent
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
            wound_count: env.storage().persistent().get(&WOUND_COUNT_KEY).unwrap_or(0),
        })
    }

    /// @notice Get current heart balance
    /// @return i128 The agent's heart balance in stroops
    pub fn get_heart_balance(env: Env) -> i128 {
        env.storage().persistent().get(&HEART_BALANCE_KEY).unwrap_or(0)
    }

    /// @notice Check if agent is initialized
    /// @return bool True if the contract has been initialized
    pub fn is_initialized(env: Env) -> bool {
        env.storage().instance().has(&OWNER_KEY)
    }

    /// @notice Get current deadline info
    /// @return (u32, u32, u32) Tuple of (current_ledger, deadline_ledger, grace_deadline)
    pub fn get_deadlines(env: Env) -> (u32, u32, u32) {
        let current_ledger = env.ledger().sequence();
        let deadline: u32 = env.storage().instance().get(&DEADLINE_KEY).unwrap_or(current_ledger);
        let grace: u32 = env.storage().instance().get(&GRACE_KEY).unwrap_or(current_ledger);
        (current_ledger, deadline, grace)
    }
}

mod test;
