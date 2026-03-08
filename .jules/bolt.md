## 2026-03-08 - Cache Transaction Hash to Avoid Repeated XDR Parsing
**Learning:** Parsing Stellar XDR using `TransactionBuilder.fromXDR` is a resource-intensive operation. Executing this within loops or frequently accessed methods (like `getTransactionStatus`) creates a significant performance bottleneck.
**Action:** Precompute and cache properties like transaction hashes when the transaction is first processed, and use the cached value for subsequent lookups to reduce O(n) XDR parsing down to an O(n) string comparison.
