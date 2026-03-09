## 2025-03-09 - Tracking Global State in Soroban Contracts
**Learning:** Iterating over maps in Soroban smart contracts to count items or aggregate state is an O(N) operation that becomes increasingly expensive as the number of entries grows. Tracking these aggregates as separate variables in instance storage reduces the complexity to O(1).
**Action:** Always consider if global aggregates (like counts, totals, or averages) should be tracked incrementally during state transitions instead of being re-calculated on demand.
