## 2024-03-06 - Precomputing Parsed XDR Hash
**Learning:** Parsing Stellar XDR using `TransactionBuilder.fromXDR` is highly resource-intensive. Executing this within a loop (e.g., checking transaction status across a queue) creates a significant performance bottleneck.
**Action:** When working with Stellar transactions in the relayer, always precompute properties like the transaction hash right after initial deserialization and cache it in the queue object to avoid redundant `fromXDR` calls during subsequent processing or status checks.
