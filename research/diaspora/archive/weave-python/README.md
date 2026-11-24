# Diaspora — Coupled Cycle Lattice (experiment WEAVE-L0)

This is **runnable, verified reference code** for a non-linear *Cycle lattice*.
It is NOT a production blockchain, a finality protocol, or proof of the claimed 99% mining-power threshold.

## What is different here?

- No three A/B/C persistent blockchains. There is one directed acyclic graph.
- A wave contains `k >= 3` separately mined Cycles. `slot` is wave-local,
  not a persistent lane or an identity; no Cycle has a privileged single predecessor.
- Every new Cycle is coupled to **all** the previous wave's `k` Cycles.
- Mining a Cycle requires `k` distinct *parent-bound* hash puzzles plus a final binding puzzle whose input commits to all the proofs. Altering a parent invalidates its corresponding strand proof and the entire binding proof.
- Historical **Warp archives** are additional deterministically checked references to older Cycles (the original D-Protocol's historical idea). Archives are not extra proof-of-work credit.
- The `descendant_cone()` computes all future Cycles invalidated by modifying a Cycle.
- Work counts each actual PoW proof once. Anchors must never amplify the same proof's work.
- Width is configurable (3..12), so we're testing three or more structural dependencies.

## Run

Only Python standard library is needed (3.10+):

```bash
python weave.py --width 3 --waves 4 --bits 8
python weave.py --width 5 --waves 4 --bits 8 --output full
PYTHONPATH=. python -m unittest discover -s tests -v
```

## Security limits — required before any real network

1. **No defense against 55% majority hash control is established.** All puzzles currently use the same SHA3 computational resource. More puzzles increase work for honest miners too. No 99% safety property is implied or advertised.
2. **Wave formation is synchronous and deliberately closed-world.** Without a permissionless eligibility/slot allocation, networking/gossip, bounded delay and non-censorship proofs, a miner could withhold a slot or create alternate wave histories. There is no liveness/consensus proof.
3. **No fork-choice consensus.** `counted_work_units()` is only expected hash-trial accounting for a VALID local DAG, not fork choice and never a security multiplier. Define canonical cone comparison and prove common-prefix/chain-growth analogues before allowing adversarial forks.
4. **No production randomness.** Warp sampling is deterministic and reproducible, but not unpredictable. Miner-controlled VRFs would allow grinding; use robust public randomness/commit-reveal when making the design adversarial.
5. **No signed independent witness finality.** If Diaspora needs sealed history even against majority miners, design separately Sybil-resistant witness voting with quorum intersection and partition safety.
6. **No transactions, UTXO state, economics, P2P, orphan handling, signature validation, checkpointing, slashing, difficulty adjustment or persistence.** Must be built and audited.
7. **Do not treat re-mined descendants as additive security overhead beyond honest-work comparisons.** Compare actual attacker and honest work under the same resource model and network assumptions.

## Next architecture step (Rust)

Port this model, without changing its invariants, into:

- `cycle.rs`: canonical (de)serialization, SHA3-512 Cycle identifiers, vector of parent IDs, separated Warp archival references;
- `mining.rs`: configurable proof-domain challenges, difficulty validation, parent-bound proofs, unique work accounting;
- `lattice.rs`: DAG acceptance, partial views, orphan pool, cycle topological verification, efficient descendants;
- `frontier.rs`: candidate parent selection from an *objectively verifiable* eligible frontier; support asynchronous network races;
- `consensus.rs`: define preference between overlapping DAGs without counting shared ancestors twice; specify availability and safety assumptions;
- `finality.rs`: if required, independent checkpoint certification and network-partition recovery;
- `sim.rs`: adversarial mining allocation and censorship, 30–99% hash-power scenarios, measure depth-specific reversal probability and recovery costs.

Review the original D-Protocol semantics before declaring a consensus rule. This first proof of concept intentionally exposes the real questions instead of fabricating a 99% theorem.
