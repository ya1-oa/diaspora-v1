# Diaspora Weave L0 — Security model and 99% threshold research

## Scope

This document separates (a) **structural rewrite coupling**, (b) **hash-power majority attacks**, and (c) **independently authenticated finality**. These are different properties. An edge in the DAG is a binding commitment, not an additional unit of work.

## Why 3+ strands are necessary but not sufficient

A Cycle C references an objectively specified set of width `k` parent Cycle IDs. C must pass `k` parent-bound proof challenges plus a binding challenge, every proof SHA3-256 with fixed `bits` leading zeroes. Expected hashes per Cycle are:

`E[trials per Cycle] ≈ (k + 1) × 2^bits`

Changing any parent ID invalidates the corresponding proof, binding proof, and Cycle ID. Its descendants also need reconstruction. However the *honest network* must do exactly the same number of challenges. If both parties use the same fungible hash resource, multiplying honest and adversarial work by `(k+1)` does not change the sign of the majority-work race.

There are no privileged A/B/C chains in this L0 structure. Slots are ephemeral labels within a synchronized experimental wave. The correct production goal is an asynchronous DAG, not k independent chains.

## Majority hash-power calculation: baseline

Let `q = H_attacker / (H_attacker + H_honest)` be the attacker's fraction of compatible computational throughput, and `p = 1 - q`. In an idealized independent proof-arrival race where the attacker starts `z` units behind, their probability of **ever** catching up is:

- if `0 ≤ q < p`: `Pr(catch up) = (q / p)^z`;
- if `q ≥ p`: `Pr(catch up eventually) = 1`.

This is an infinite-horizon **random-walk result**, not a statement that every real attack succeeds instantly, nor a proof that the current L0 DAG has this exact stochastic process. Difficulty, rewards, propagation, availability, economics, validator behavior, and network delays require their own model.

For `q = 0.55` and `p = 0.45`, attacker expected proof production is higher than honest production. Merely requiring three or five hashes per Cycle scales both rates, and cannot turn the attacker fraction into 0.99.

The included baseline simulator checks the bounded-horizon version using seeded trials. It does *not* model network topology or producer eligibility, so results may not be reported as a Diaspora attack success measurement.

## What a literal 99% hash-power threshold would demand

If the network uses only homogeneous, transferable hash computation and honest and malicious miners have the same algorithm and access, achieving a universal theorem that a 55% attacker cannot privately replace unfinalized history but a 99% attacker can is **not supported by the design**. A security rule would need to change the relative resource advantage:

`attacker_effective_rate = q H × e_A`
`honest_effective_rate   = (1-q) H × e_H`

For the attack to become favorable only at `q ≥ 0.99`, we would require

`0.99 e_A = 0.01 e_H` → `e_H / e_A = 99`.

That is, honest miners would need **99 times the accepted progress per hash** compared with adversarial miners. A public permissionless hash algorithm has no reliable honesty oracle that can award honest hashes 99x preferential weight. Assigning this ratio by fiat is not a valid security design.

Note this argument rules out a general *hash-only honest-vs-malicious asymmetric reward* mechanism under equal access; it does not rule out bounded-time probabilistic defenses, tradeoffs with synchronization, or security grounded in additional independent resources.

## One route to extreme finality safety (but NOT a 99% hash threshold)

Introduce a genuinely independent Sybil-resistant voting resource. With N equal-weight validators and a quorum Q, **two conflicting certificates** signed by quorums of size Q must overlap in at least `2Q-N` validators. If honest validators never certify conflicting history under the protocol's lock rules, an adversary needs to compromise this overlap to produce conflicting certificates.

For example, N=1000, Q=995, the intersection is at least 990, i.e. **99% of validator identities**, *not hash rate*. This illustrative near-unanimity design is extremely brittle for availability: withholding just `N-Q+1 = 6` validator signatures (0.6%) prevents new certificates. Threshold selection is a tradeoff between safety, liveness, quorum independence, and network partitions; 99% witness agreement is probably unacceptable for a live permissionless system.

A more practical quorum could be around 2/3 with corresponding <1/3 Byzantine safety assumption, plus independently measurable validator distributions. A majority miner could then reorganize **unfinalized** history or censor but would not have enough authority to overwrite finalized history merely by mining. If all authentication is ultimately controlled by the same actor as mining, no independence has been achieved.

If requirements insist on a *literal 99% of network hashes* rather than an independent-finality guarantee, define exactly the attack event, duration, difficulty adjustment, and Sybil model and prove the threshold from explicit assumptions; do not claim this result on DAG geometry alone.

## Open research hypotheses

H1: Multi-parent commitments create a larger invalidation cone than a simple predecessor-only graph, holding DAG size fixed (structural claim, demonstrated locally for selected rewrites).

H2: Coupled commitments measurably increase private rewrite **work** beyond ordinary cumulative unique hash work under an objectively selected frontier (unproven; must measure honest costs and adversarial strategy, including withheld blocks).

H3: An asynchronous permissionless frontier rule can achieve both safety and liveness against substantial hash concentration without trusted eligibility or external finality (unproven; construct adversarial forks and counterexamples).

H4: Independently selected finality committees maintain safety even when majority hash power produces competing unfinalized Cycles (depends on committee Sybil resistance and proof of quorum independence).

## Acceptance criteria

- Publish a deterministic simulation input format and adversarial scenario labels.
- Show per-hash work for adversary and honest miners, not 'number of rewritten descendants' as security score.
- Model partitions, withholding, selfish mining, eclipse views, VRF grinding, and Sybil producer identities.
- Compare equal total hash budgets across a Bitcoin-like linear control, archive-only control, and candidate lattice.
- Report confidence intervals, not just one observed fraction, and clearly distinguish safety from liveness.
- No 99% claim until peer-reviewable theorem and attack implementation agree under the same declared threat model.
