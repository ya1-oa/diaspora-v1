# Weave finalized-history resistance

**Claim.** Under the assumptions below, even 100% of mining hash power cannot make two honest nodes finalize incompatible Weave frontiers. A conflicting pair of certificates requires at least `2q − n` distinct witnesses to sign both, where `n` is the fixed committee size and `q = floor(2n/3) + 1`. This is a bound on independent witness control, separate from hash power.

| Witnesses | Certificate quorum | Minimum double signers for conflicting certificates |
| --- | --- | --- |
| 4 | 3 | 2 |
| 7 | 5 | 3 |
| 10 | 7 | 4 |

## Assumptions

Both nodes pin the same authentic network manifest and genesis. Frontier commitments bind the complete canonical frontier and its ancestry. Nodes validate the whole history before accepting a new checkpoint. Signatures and commitments are secure. Each honest witness verifies the certified parent, validates its proposed target, and durably signs at most one target per checkpoint height in this network. Node checkpoints do not roll back on restart. The committee is fixed and equal-weight. Deployment must establish independence across witness operators and key infrastructure.

## Proof

1. Suppose two accepted checkpoint histories diverge. Take the first checkpoint height at which their commitments differ. The histories share the previous certified checkpoint, so each certificate signs a different target at the same height with the same parent and network ID.
2. Let their signer sets be A and B. Both have at least q distinct members and their union has at most n. Therefore |A ∩ B| = |A| + |B| − |A ∪ B| ≥ 2q − n. Every signer in the intersection has signed both targets.
3. With fewer than 2q − n double-signing witnesses, the conflicting certificates cannot both exist. Mining work cannot supply a witness signature. On a node that already accepted a checkpoint, fork choice also filters out every competing history missing that checkpoint at its exact height, regardless of work.
4. The intersection bound is tight as a set bound. Two q-member sets can share exactly 2q − n members. Any production claim must preserve the assumptions rather than treating this bound as a guarantee against witness capture.

## Scope

The Lattice's parent-bound proofs make tampering detectable. If a Cycle changes, descendants must commit to the new history and supply valid work. When both sides solve the same puzzles, coupling gives no demonstrated increase in the hash-only threshold. Majority miners can still reorder provisional history, delay inclusion, or withhold data. Finality can stall during a partition or when the quorum is unavailable. A conflicting-certificate halt applies when a node sees the evidence.

The Rust verifier checks signed certificates and local fork choice. Its tests use synthetic validated frontiers. They exhaustively enumerate quorum-set intersections for small committees and compare arbitrarily heavier incompatible histories with a certified branch. The live radio lab does not enforce these certificates. Production work still includes canonical Rust lattice verification and proof accounting, durable witness locks and checkpoint state, and partition/restart tests on independent nodes.
