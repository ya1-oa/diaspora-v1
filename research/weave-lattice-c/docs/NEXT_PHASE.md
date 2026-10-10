# Diaspora research — next implementation phases

## L0 verified in C11 (this branch)

- SHA3-256 domain-separated parent strands and binder proofs
- minimum 3 parents, width up to 8 in research harness
- deterministically validated historical anchors
- slot-order-independent submissions within each completed wave
- replay, tampering, fork divergence, descendants, capacity tests
- seeded majority hash race baseline
- GCC/Clang builds and ASAN/UBSAN local tests

## L1: Async frontier + fault model (highest priority)

Replace wave barriers with asynchronous DAG frontier rules:
1. Define eligible parents from locally observed graph with deterministic tie-breakers, and prove that equivalent complete views converge.
2. Define a rate-limited admission mechanism; note that public slots without independent eligibility permit producer grinding and Sybil multiplication.
3. Network messages include Cycle ID, parent IDs, strand proofs, binder, archive proof, and origin epoch. Keep wire encoding canonical.
4. Accept orphans; defer validation until all dependencies arrive. Detect equivocation and circular references.
5. Compare competing DAG worlds with a fork-choice function that credits *each cryptographic proof once* and handles overlap, withholding, and ties.
6. Implement a deterministic single-process discrete-event network: delay, partitions, adversarial miners, offline peers, and mining share.

## L2: Research security evaluation

- Model protocol-specific private fork attacks at 30, 45, 51, 55, 70, 99% hash share. Compare at fixed **total** hash budget and same finality depth.
- Measure `(reversal_probability, elapsed_hashes, delay, stall_probability)` with fixed RNG seeds and confidence intervals.
- Add a linear PoW baseline and archive-only DAG control. Treat a higher reference count without extra resource costs as non-security.
- Search for counterexamples first: dishonest slot selection, parent grinding, invalidation asymmetry, withholding, and partition healing.

## L3: Rust implementation

Once L1 invariants are stable, port consensus core to Rust crate `diaspora-weave`, comparing canonical vectors C ↔ Rust byte-for-byte. Use property-based and fuzz tests. Keep C11 research code as a portable reference implementation.

## L4: Independent finality (only if needed)

Specify validator resource, identity/selection, locking, equivocation evidence, checkpoint certificate, and failure recovery. Quantify both the hash-majority and Byzantine quorum threats; do not conflate them.

**Non-goals tonight:** public testnet, real-money transactions, custody, production key management, or any unproven 99% statement.
