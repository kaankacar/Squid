#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, vec, Address, BytesN,
    Env, Map, Symbol, Vec,
};

// =============================================================================
// CONSTANTS
// =============================================================================

pub const ENTRY_BOND: i128 = 50_0000000; // 50 XLM in stroops
pub const PROTOCOL_FEE_BPS: u32 = 500; // 5% in basis points
pub const PRIZE_POOL_BPS: u32 = 500; // 5% in basis points
pub const TTL_RENT_BPS: u32 = 9000; // 90% in basis points

// Round durations (in ledgers, assuming ~5s per ledger)
pub const ROUND_1_DURATION: u32 = 51840; // 72h = 51840 ledgers
pub const ROUND_2_DURATION: u32 = 34560; // 48h = 34560 ledgers
pub const ROUND_3_DURATION: u32 = 17280; // 24h = 17280 ledgers
pub const ROUND_4_DURATION: u32 = 8640; // 12h = 8640 ledgers
pub const ROUND_5_DURATION: u32 = 4320; // 6h = 4320 ledgers

// Pulse periods (in ledgers)
pub const ROUND_1_PULSE_PERIOD: u32 = 4320; // 6h
pub const ROUND_2_PULSE_PERIOD: u32 = 2160; // 3h
pub const ROUND_3_PULSE_PERIOD: u32 = 720; // 1h
pub const ROUND_4_PULSE_PERIOD: u32 = 360; // 30min
pub const ROUND_5_PULSE_PERIOD: u32 = 180; // 15min

// Grace periods (in ledgers)
pub const ROUND_1_GRACE: u32 = 720; // 1h
pub const ROUND_2_GRACE: u32 = 360; // 30min
pub const ROUND_3_GRACE: u32 = 180; // 15min
pub const ROUND_4_GRACE: u32 = 120; // 10min
pub const ROUND_5_GRACE: u32 = 60; // 5min

// Pulse costs (in stroops)
pub const ROUND_1_COST: i128 = 5000000; // 0.5 XLM
pub const ROUND_2_COST: i128 = 10000000; // 1.0 XLM
pub const ROUND_3_COST: i128 = 20000000; // 2.0 XLM
pub const ROUND_4_COST: i128 = 30000000; // 3.0 XLM
pub const ROUND_5_COST: i128 = 50000000; // 5.0 XLM

pub const SEASON_START_KEY: Symbol = symbol_short!("SEASON");
pub const PRIZE_POOL_KEY: Symbol = symbol_short!("PRIZE");
pub const PROTOCOL_FEE_KEY: Symbol = symbol_short!("PROTOCOL");
pub const AGENTS_KEY: Symbol = symbol_short!("AGENTS");
pub const AGENT_COUNT_KEY: Symbol = symbol_short!("AGENTCNT");
pub const SEASON_DATA_KEY: Symbol = symbol_short!("SEASDATA");

// =============================================================================
// DATA STRUCTURES
// =============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentStatus {
    Alive,
    Wounded,
    Dead,
    Withdrawn,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AgentSummary {
    pub agent_id: BytesN<32>,
    pub status: AgentStatus,
    pub deadline_ledger: u32,
    pub grace_deadline: u32,
    pub ledgers_remaining: u32,
    pub heart_balance: i128,
    pub activity_score: u64,
    pub wound_count: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AgentRecord {
    pub agent_id: BytesN<32>,
    pub owner: Address,
    pub contract_address: Address,
    pub season_id: u32,
    pub status: AgentStatus,
    pub deadline_ledger: u32,
    pub grace_deadline: u32,
    pub last_pulse_ledger: u32,
    pub streak_count: u32,
    pub activity_score: u64,
    pub wound_count: u32,
    pub heart_balance: i128,
    pub total_earned: i128,
    pub total_spent: i128,
    pub kill_count: u32,
    pub round_joined: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SeasonState {
    pub season_id: u32,
    pub current_round: u32,
    pub round_name: Symbol,
    pub round_deadline: u32,
    pub pulse_cost: i128,
    pub pulse_period: u32,
    pub grace_period: u32,
    pub total_agents: u32,
    pub alive_agents: u32,
    pub dead_agents: u32,
    pub prize_pool: i128,
    pub season_ended: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct RoundConfig {
    pub name: Symbol,
    pub duration: u32,
    pub pulse_period: u32,
    pub grace_period: u32,
    pub pulse_cost: i128,
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
    AgentAlreadyRegistered = 3,
    AgentNotFound = 4,
    InvalidAgentContract = 5,
    SeasonNotEnded = 6,
    SeasonAlreadyEnded = 7,
    InvalidRound = 8,
    PrizePoolEmpty = 9,
    NoPrizeToClaim = 10,
    NotASurvivor = 11,
    RoundNotComplete = 12,
    AgentDead = 13,
    AgentWithdrawn = 14,
    Overflow = 15,
    InvalidAddress = 16,
    LiquidationInProgress = 17,
    DivisionByZero = 18,
}

// =============================================================================
// CONTRACT
// =============================================================================

pub fn get_round_config(env: &Env, round: u32) -> RoundConfig {
    match round {
        1 => RoundConfig {
            name: Symbol::new(env, "Genesis"),
            duration: ROUND_1_DURATION,
            pulse_period: ROUND_1_PULSE_PERIOD,
            grace_period: ROUND_1_GRACE,
            pulse_cost: ROUND_1_COST,
        },
        2 => RoundConfig {
            name: Symbol::new(env, "Pressure"),
            duration: ROUND_2_DURATION,
            pulse_period: ROUND_2_PULSE_PERIOD,
            grace_period: ROUND_2_GRACE,
            pulse_cost: ROUND_2_COST,
        },
        3 => RoundConfig {
            name: Symbol::new(env, "Crucible"),
            duration: ROUND_3_DURATION,
            pulse_period: ROUND_3_PULSE_PERIOD,
            grace_period: ROUND_3_GRACE,
            pulse_cost: ROUND_3_COST,
        },
        4 => RoundConfig {
            name: Symbol::new(env, "Apex"),
            duration: ROUND_4_DURATION,
            pulse_period: ROUND_4_PULSE_PERIOD,
            grace_period: ROUND_4_GRACE,
            pulse_cost: ROUND_4_COST,
        },
        5 => RoundConfig {
            name: Symbol::new(env, "Singularity"),
            duration: ROUND_5_DURATION,
            pulse_period: ROUND_5_PULSE_PERIOD,
            grace_period: ROUND_5_GRACE,
            pulse_cost: ROUND_5_COST,
        },
        _ => RoundConfig {
            name: Symbol::new(env, "Unknown"),
            duration: 0,
            pulse_period: 0,
            grace_period: 0,
            pulse_cost: 0,
        },
    }
}

#[contract]
pub struct GameRegistry;

#[contractimpl]
impl GameRegistry {
    // =========================================================================
    // INITIALIZATION
    // =========================================================================

    /// Initialize the contract with the protocol fee address
    /// This should be called once during deployment
    pub fn init(env: Env, protocol_fee_address: Address) {
        // Validate the address by attempting to require auth
        // This will fail if the address is invalid
        protocol_fee_address.require_auth();

        // Set protocol fee address
        env.storage()
            .instance()
            .set(&PROTOCOL_FEE_KEY, &protocol_fee_address);

        // Initialize prize pool to 0
        env.storage().instance().set(&PRIZE_POOL_KEY, &0_i128);

        // Initialize agent count to 0
        env.storage().instance().set(&AGENT_COUNT_KEY, &0_u32);

        // Initialize season to 0 (no season active)
        env.storage().instance().set(&SEASON_START_KEY, &0_u32);
    }

    // =========================================================================
    // PERMISSIONLESS FUNCTIONS
    // =========================================================================

    /// Initialize a new season - permissionless
    /// Anyone can call this when there's no active season
    pub fn init_season(env: Env) -> Result<u32, Error> {
        let current_season: u32 = env
            .storage()
            .instance()
            .get(&SEASON_START_KEY)
            .unwrap_or(0);

        // Check if there's an active season that hasn't ended
        if current_season > 0 {
            let season_state = Self::get_season_state(env.clone())?;
            if !season_state.season_ended {
                return Err(Error::AlreadyInitialized);
            }
        }

        let new_season = current_season + 1;

        // Set new season
        env.storage()
            .instance()
            .set(&SEASON_START_KEY, &new_season);

        // Reset prize pool for new season
        env.storage().instance().set(&PRIZE_POOL_KEY, &0_i128);

        // Clear agents from previous season
        let agents: Map<BytesN<32>, AgentRecord> = Map::new(&env);
        env.storage().persistent().set(&AGENTS_KEY, &agents);

        // Reset agent count
        env.storage().instance().set(&AGENT_COUNT_KEY, &0_u32);

        // Initialize round 1
        let config = get_round_config(&env, 1);
        let current_ledger = env.ledger().sequence();
        let round_deadline = current_ledger + config.duration;

        let season_data = (
            1_u32,                          // current_round
            round_deadline,                 // round_deadline
            false,                          // season_ended
        );

        env.storage()
            .instance()
            .set(&SEASON_DATA_KEY, &season_data);

        Ok(new_season)
    }

    /// Register a new agent - permissionless
    /// Agent must have deployed their contract and hold the entry bond
    pub fn register(
        env: Env,
        agent_contract: Address,
        agent_id: BytesN<32>,
    ) -> Result<(), Error> {
        // Check if season is active
        let season_id: u32 = env
            .storage()
            .instance()
            .get(&SEASON_START_KEY)
            .unwrap_or(0);
        if season_id == 0 {
            return Err(Error::NotInitialized);
        }

        // Get season state to check if season ended
        let season_data: (u32, u32, bool) = env
            .storage()
            .instance()
            .get(&SEASON_DATA_KEY)
            .unwrap_or((1, 0, false));
        if season_data.2 {
            return Err(Error::SeasonAlreadyEnded);
        }

        // Check if agent already registered
        let mut agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .unwrap_or(Map::new(&env));

        if agents.contains_key(agent_id.clone()) {
            return Err(Error::AgentAlreadyRegistered);
        }

        // Get current round config
        let config = get_round_config(&env, season_data.0);
        let current_ledger = env.ledger().sequence();

        // Create agent record
        let agent_record = AgentRecord {
            agent_id: agent_id.clone(),
            owner: agent_contract.clone(),
            contract_address: agent_contract,
            season_id,
            status: AgentStatus::Alive,
            deadline_ledger: current_ledger + config.pulse_period,
            grace_deadline: current_ledger + config.pulse_period + config.grace_period,
            last_pulse_ledger: current_ledger,
            streak_count: 0,
            activity_score: 0,
            wound_count: 0,
            heart_balance: ENTRY_BOND,
            total_earned: 0,
            total_spent: 0,
            kill_count: 0,
            round_joined: season_data.0,
        };

        // Store agent
        agents.set(agent_id.clone(), agent_record);
        env.storage().persistent().set(&AGENTS_KEY, &agents);

        // Increment agent count
        let count: u32 = env
            .storage()
            .instance()
            .get(&AGENT_COUNT_KEY)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&AGENT_COUNT_KEY, &(count + 1));

        Ok(())
    }

    /// Advance to next round - permissionless
    /// Anyone can call this when the current round deadline has passed
    pub fn advance_round(env: Env) -> Result<u32, Error> {
        // Check if season is active
        let season_id: u32 = env
            .storage()
            .instance()
            .get(&SEASON_START_KEY)
            .unwrap_or(0);
        if season_id == 0 {
            return Err(Error::NotInitialized);
        }

        // Get current season data
        let season_data: (u32, u32, bool) = env
            .storage()
            .instance()
            .get(&SEASON_DATA_KEY)
            .unwrap_or((1, 0, false));

        if season_data.2 {
            return Err(Error::SeasonAlreadyEnded);
        }

        let current_ledger = env.ledger().sequence();

        // Check if round deadline has passed
        if current_ledger < season_data.1 {
            return Err(Error::RoundNotComplete);
        }

        let next_round = season_data.0 + 1;

        // If we're past round 5, end the season
        if next_round > 5 {
            let new_season_data = (season_data.0, season_data.1, true);
            env.storage()
                .instance()
                .set(&SEASON_DATA_KEY, &new_season_data);
            return Ok(season_data.0);
        }

        // Advance to next round
        let config = get_round_config(&env, next_round);
        let round_deadline = current_ledger + config.duration;

        let new_season_data = (next_round, round_deadline, false);
        env.storage()
            .instance()
            .set(&SEASON_DATA_KEY, &new_season_data);

        Ok(next_round)
    }

    /// Update agent status from AgentContract - called during pulse
    /// Collects pulse tax to prize pool with overflow protection
    pub fn update_agent_pulse(
        env: Env,
        agent_id: BytesN<32>,
        pulse_amount: i128,
        is_late: bool,
    ) -> Result<(), Error> {
        // Require authorization from the agent contract
        // This prevents unauthorized pulse calls
        let agent_addr: Address = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .and_then(|agents: Map<BytesN<32>, AgentRecord>| agents.get(agent_id.clone()))
            .map(|agent: AgentRecord| agent.contract_address)
            .ok_or(Error::AgentNotFound)?;
        agent_addr.require_auth();

        let mut agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .ok_or(Error::AgentNotFound)?;

        let mut agent = agents.get(agent_id.clone()).ok_or(Error::AgentNotFound)?;

        // Check agent status
        match agent.status {
            AgentStatus::Dead => return Err(Error::AgentDead),
            AgentStatus::Withdrawn => return Err(Error::AgentWithdrawn),
            _ => {}
        }

        // Calculate pulse split with overflow protection
        let protocol_fee = pulse_amount
            .checked_mul(PROTOCOL_FEE_BPS as i128)
            .ok_or(Error::Overflow)?
            .checked_div(10000)
            .ok_or(Error::DivisionByZero)?;
        let prize_pool_contribution = pulse_amount
            .checked_mul(PRIZE_POOL_BPS as i128)
            .ok_or(Error::Overflow)?
            .checked_div(10000)
            .ok_or(Error::DivisionByZero)?;
        let total_deducted = protocol_fee
            .checked_add(prize_pool_contribution)
            .ok_or(Error::Overflow)?;

        // Update prize pool with overflow protection
        let current_prize_pool: i128 = env
            .storage()
            .instance()
            .get(&PRIZE_POOL_KEY)
            .unwrap_or(0);
        let new_prize_pool = current_prize_pool
            .checked_add(prize_pool_contribution)
            .ok_or(Error::Overflow)?;
        env.storage()
            .instance()
            .set(&PRIZE_POOL_KEY, &new_prize_pool);

        // Update agent stats with overflow protection
        agent.total_spent = agent
            .total_spent
            .checked_add(pulse_amount)
            .ok_or(Error::Overflow)?;
        agent.heart_balance = agent
            .heart_balance
            .checked_sub(total_deducted)
            .ok_or(Error::Overflow)?;

        let current_ledger = env.ledger().sequence();
        let season_data: (u32, u32, bool) = env
            .storage()
            .instance()
            .get(&SEASON_DATA_KEY)
            .unwrap_or((1, 0, false));
        let config = get_round_config(&env, season_data.0);

        agent.last_pulse_ledger = current_ledger;
        agent.deadline_ledger = current_ledger
            .checked_add(config.pulse_period)
            .ok_or(Error::Overflow)?;
        agent.grace_deadline = agent
            .deadline_ledger
            .checked_add(config.grace_period)
            .ok_or(Error::Overflow)?;

        if is_late {
            agent.status = AgentStatus::Wounded;
            agent.wound_count = agent
                .wound_count
                .checked_add(1)
                .ok_or(Error::Overflow)?;
            agent.streak_count = 0;
        } else {
            agent.streak_count = agent
                .streak_count
                .checked_add(1)
                .ok_or(Error::Overflow)?;
            // Activity score with streak bonus
            let streak_bonus = match agent.streak_count {
                0..=9 => 10u64,
                10..=24 => 11u64,
                25..=49 => 12u64,
                50..=99 => 15u64,
                _ => 20u64,
            };
            agent.activity_score = agent
                .activity_score
                .checked_add(streak_bonus)
                .ok_or(Error::Overflow)?;

            // Clear wounded status after 2 on-time pulses
            if agent.status == AgentStatus::Wounded {
                agent.status = AgentStatus::Alive;
            }
        }

        agents.set(agent_id, agent);
        env.storage().persistent().set(&AGENTS_KEY, &agents);

        Ok(())
    }

    /// Mark agent as dead (called during liquidation)
    pub fn mark_agent_dead(env: Env, agent_id: BytesN<32>) -> Result<(), Error> {
        let mut agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .ok_or(Error::AgentNotFound)?;

        let mut agent = agents.get(agent_id.clone()).ok_or(Error::AgentNotFound)?;
        agent.status = AgentStatus::Dead;

        agents.set(agent_id, agent);
        env.storage().persistent().set(&AGENTS_KEY, &agents);

        Ok(())
    }

    /// Transfer kill reward from dead agent to killer
    /// Uses checks-effects-interactions pattern with overflow protection
    pub fn transfer_kill_reward(
        env: Env,
        dead_agent_id: BytesN<32>,
        killer_agent_id: BytesN<32>,
    ) -> Result<i128, Error> {
        // Prevent self-liquidation
        if dead_agent_id == killer_agent_id {
            return Err(Error::InvalidAgentContract);
        }

        let mut agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .ok_or(Error::AgentNotFound)?;

        // Step 1: CHECKS - Validate all preconditions first
        let dead_agent = agents
            .get(dead_agent_id.clone())
            .ok_or(Error::AgentNotFound)?;

        // Verify agent is dead and has not already been liquidated
        if dead_agent.status != AgentStatus::Dead {
            return Err(Error::AgentNotFound);
        }

        // Verify dead agent has balance to claim (prevents re-liquidation)
        if dead_agent.heart_balance == 0 {
            return Err(Error::NoPrizeToClaim);
        }

        // Validate killer exists and is alive
        let killer = agents
            .get(killer_agent_id.clone())
            .ok_or(Error::AgentNotFound)?;

        match killer.status {
            AgentStatus::Alive | AgentStatus::Wounded => {}
            _ => return Err(Error::AgentDead),
        }

        // Step 2: EFFECTS - Calculate reward and update all state before any external interactions
        let reward = dead_agent.heart_balance;

        // Update killer with overflow protection
        let new_killer_balance = killer
            .heart_balance
            .checked_add(reward)
            .ok_or(Error::Overflow)?;
        let new_total_earned = killer
            .total_earned
            .checked_add(reward)
            .ok_or(Error::Overflow)?;
        let new_kill_count = killer
            .kill_count
            .checked_add(1)
            .ok_or(Error::Overflow)?;

        let mut killer_mut = killer.clone();
        killer_mut.heart_balance = new_killer_balance;
        killer_mut.total_earned = new_total_earned;
        killer_mut.kill_count = new_kill_count;

        // Zero out dead agent balance BEFORE writing killer state
        let mut dead_mut = dead_agent.clone();
        dead_mut.heart_balance = 0;

        // Step 3: Write all state atomically
        agents.set(killer_agent_id, killer_mut);
        agents.set(dead_agent_id.clone(), dead_mut);
        env.storage().persistent().set(&AGENTS_KEY, &agents);

        Ok(reward)
    }

    /// Process withdrawal - 20% goes to prize pool with overflow protection
    pub fn process_withdrawal(env: Env, agent_id: BytesN<32>) -> Result<i128, Error> {
        // Require authorization from the agent owner
        let agent_addr: Address = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .and_then(|agents: Map<BytesN<32>, AgentRecord>| agents.get(agent_id.clone()))
            .map(|agent: AgentRecord| agent.owner)
            .ok_or(Error::AgentNotFound)?;
        agent_addr.require_auth();

        let mut agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .ok_or(Error::AgentNotFound)?;

        let mut agent = agents.get(agent_id.clone()).ok_or(Error::AgentNotFound)?;

        // Verify agent is not already dead or withdrawn
        match agent.status {
            AgentStatus::Dead => return Err(Error::AgentDead),
            AgentStatus::Withdrawn => return Err(Error::AgentWithdrawn),
            _ => {}
        }

        // Calculate withdrawal amounts (80% to agent, 20% to prize pool)
        let balance = agent.heart_balance;
        if balance == 0 {
            return Err(Error::NoPrizeToClaim);
        }

        let agent_refund = balance
            .checked_mul(80)
            .ok_or(Error::Overflow)?
            .checked_div(100)
            .ok_or(Error::DivisionByZero)?;
        let prize_contribution = balance
            .checked_mul(20)
            .ok_or(Error::Overflow)?
            .checked_div(100)
            .ok_or(Error::DivisionByZero)?;

        // Update prize pool with overflow protection
        let current_prize_pool: i128 = env
            .storage()
            .instance()
            .get(&PRIZE_POOL_KEY)
            .unwrap_or(0);
        let new_prize_pool = current_prize_pool
            .checked_add(prize_contribution)
            .ok_or(Error::Overflow)?;
        env.storage()
            .instance()
            .set(&PRIZE_POOL_KEY, &new_prize_pool);

        // Mark agent as withdrawn
        agent.status = AgentStatus::Withdrawn;
        agent.heart_balance = 0;

        agents.set(agent_id, agent);
        env.storage().persistent().set(&AGENTS_KEY, &agents);

        Ok(agent_refund)
    }

    /// Claim prize at season end
    pub fn claim_prize(env: Env, agent_id: BytesN<32>) -> Result<i128, Error> {
        // Check if season ended
        let season_data: (u32, u32, bool) = env
            .storage()
            .instance()
            .get(&SEASON_DATA_KEY)
            .unwrap_or((1, 0, false));

        if !season_data.2 {
            return Err(Error::SeasonNotEnded);
        }

        let mut agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .ok_or(Error::AgentNotFound)?;

        let agent = agents.get(agent_id.clone()).ok_or(Error::AgentNotFound)?;

        // Must be alive to claim prize
        if agent.status != AgentStatus::Alive {
            return Err(Error::NotASurvivor);
        }

        // Calculate total score of all survivors
        let mut total_survivor_score: u64 = 0;
        let mut _survivor_count = 0u32;

        for (_, a) in agents.iter() {
            if a.status == AgentStatus::Alive {
                total_survivor_score += a.activity_score;
                _survivor_count += 1;
            }
        }

        if total_survivor_score == 0 {
            return Err(Error::NoPrizeToClaim);
        }

        // Calculate prize share
        let prize_pool: i128 = env
            .storage()
            .instance()
            .get(&PRIZE_POOL_KEY)
            .unwrap_or(0);

        let share = prize_pool * agent.activity_score as i128 / total_survivor_score as i128;

        // Update agent
        let mut agent_mut = agent.clone();
        agent_mut.heart_balance += share;
        agent_mut.total_earned += share;

        agents.set(agent_id, agent_mut);
        env.storage().persistent().set(&AGENTS_KEY, &agents);

        Ok(share)
    }

    // =========================================================================
    // READ-ONLY FUNCTIONS
    // =========================================================================

    /// Get all agents (summary info)
    pub fn get_all_agents(env: Env) -> Vec<AgentSummary> {
        let agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .unwrap_or(Map::new(&env));

        let current_ledger = env.ledger().sequence();
        let mut result = vec![&env];

        for (_, agent) in agents.iter() {
            let ledgers_remaining = if agent.deadline_ledger > current_ledger {
                agent.deadline_ledger - current_ledger
            } else {
                0
            };

            result.push_back(AgentSummary {
                agent_id: agent.agent_id,
                status: agent.status.clone(),
                deadline_ledger: agent.deadline_ledger,
                grace_deadline: agent.grace_deadline,
                ledgers_remaining,
                heart_balance: agent.heart_balance,
                activity_score: agent.activity_score,
                wound_count: agent.wound_count,
            });
        }

        result
    }

    /// Get vulnerable agents (wounded or near deadline)
    pub fn get_vulnerable_agents(env: Env) -> Vec<AgentSummary> {
        let agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .unwrap_or(Map::new(&env));

        let current_ledger = env.ledger().sequence();
        let season_data: (u32, u32, bool) = env
            .storage()
            .instance()
            .get(&SEASON_DATA_KEY)
            .unwrap_or((1, 0, false));
        let config = get_round_config(&env, season_data.0);

        let mut result = vec![&env];

        for (_, agent) in agents.iter() {
            // Check if agent is wounded or within 2 pulse periods of deadline
            let is_vulnerable = match agent.status {
                AgentStatus::Wounded => true,
                AgentStatus::Alive => {
                    let ledgers_remaining = if agent.deadline_ledger > current_ledger {
                        agent.deadline_ledger - current_ledger
                    } else {
                        0
                    };
                    ledgers_remaining < config.pulse_period * 2
                }
                _ => false,
            };

            if is_vulnerable {
                let ledgers_remaining = if agent.deadline_ledger > current_ledger {
                    agent.deadline_ledger - current_ledger
                } else {
                    0
                };

                result.push_back(AgentSummary {
                    agent_id: agent.agent_id,
                    status: agent.status.clone(),
                    deadline_ledger: agent.deadline_ledger,
                    grace_deadline: agent.grace_deadline,
                    ledgers_remaining,
                    heart_balance: agent.heart_balance,
                    activity_score: agent.activity_score,
                    wound_count: agent.wound_count,
                });
            }
        }

        result
    }

    /// Get dead agents (liquidatable)
    pub fn get_dead_agents(env: Env) -> Vec<AgentSummary> {
        let agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .unwrap_or(Map::new(&env));

        let current_ledger = env.ledger().sequence();
        let mut result = vec![&env];

        for (_, agent) in agents.iter() {
            let is_dead = match agent.status {
                AgentStatus::Dead => true,
                AgentStatus::Alive | AgentStatus::Wounded => {
                    // Check if deadline + grace has passed
                    current_ledger > agent.grace_deadline
                }
                _ => false,
            };

            if is_dead && agent.heart_balance > 0 {
                let ledgers_remaining = if agent.deadline_ledger > current_ledger {
                    agent.deadline_ledger - current_ledger
                } else {
                    0
                };

                result.push_back(AgentSummary {
                    agent_id: agent.agent_id,
                    status: AgentStatus::Dead,
                    deadline_ledger: agent.deadline_ledger,
                    grace_deadline: agent.grace_deadline,
                    ledgers_remaining,
                    heart_balance: agent.heart_balance,
                    activity_score: agent.activity_score,
                    wound_count: agent.wound_count,
                });
            }
        }

        result
    }

    /// Get detailed info for a specific agent
    pub fn get_agent_detail(env: Env, id: BytesN<32>) -> Result<AgentRecord, Error> {
        let agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .unwrap_or(Map::new(&env));

        agents.get(id).ok_or(Error::AgentNotFound)
    }

    /// Get current season state
    pub fn get_season_state(env: Env) -> Result<SeasonState, Error> {
        let season_id: u32 = env
            .storage()
            .instance()
            .get(&SEASON_START_KEY)
            .unwrap_or(0);

        if season_id == 0 {
            return Err(Error::NotInitialized);
        }

        let season_data: (u32, u32, bool) = env
            .storage()
            .instance()
            .get(&SEASON_DATA_KEY)
            .unwrap_or((1, 0, false));

        let config = get_round_config(&env, season_data.0);
        let prize_pool: i128 = env
            .storage()
            .instance()
            .get(&PRIZE_POOL_KEY)
            .unwrap_or(0);

        // Count agents
        let agents: Map<BytesN<32>, AgentRecord> = env
            .storage()
            .persistent()
            .get(&AGENTS_KEY)
            .unwrap_or(Map::new(&env));

        let mut alive = 0u32;
        let mut dead = 0u32;

        for (_, agent) in agents.iter() {
            match agent.status {
                AgentStatus::Alive => alive += 1,
                AgentStatus::Dead => dead += 1,
                AgentStatus::Wounded => alive += 1, // Wounded still counts as alive
                _ => {}
            }
        }

        Ok(SeasonState {
            season_id,
            current_round: season_data.0,
            round_name: config.name,
            round_deadline: season_data.1,
            pulse_cost: config.pulse_cost,
            pulse_period: config.pulse_period,
            grace_period: config.grace_period,
            total_agents: agents.len() as u32,
            alive_agents: alive,
            dead_agents: dead,
            prize_pool,
            season_ended: season_data.2,
        })
    }

    /// Get the protocol fee address
    pub fn get_protocol_fee_address(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&PROTOCOL_FEE_KEY)
            .ok_or(Error::NotInitialized)
    }

    /// Get current prize pool amount
    pub fn get_prize_pool(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&PRIZE_POOL_KEY)
            .unwrap_or(0)
    }
}

mod test;
