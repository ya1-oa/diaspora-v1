# What a mining majority can change

I want Diaspora to keep settling honestly when a mining pool dominates production. The lattice makes a Cycle depend on a full preceding frontier. It lets every node verify where a rewrite touches the history. A miner that can perform the same work faster can still produce a different valid frontier. Every added puzzle also costs honest miners work, so descendant counts alone give us no higher hash-power threshold.

## Rules to carry forward

- **Finality over the full frontier.** Use a canonical frontier commitment that binds the prior checkpoint, the ordered Cycle IDs, the ledger state root and the exact rules version. Apply the Rust certificate verifier after every Cycle and the resulting state are available and validated. Witness operators need independent control, durable signing locks and a reviewed round-change protocol. Hash power by itself then cannot replace a certified frontier under the witness assumptions.
- **Inclusion with signed evidence.** Define a witness-certified inclusion list for a future frontier. A fully specified rule would have witnesses verify the transaction and its availability before certifying the list. The target frontier must include listed transactions or carry a deterministic exclusion reason, such as a conflicting earlier spend. Bound list size and publication delay. A mining majority can still stall or withhold, and witness capture can compromise the rule. This is a proposed mechanism and needs a separate protocol and tests.
- **Visibility across transports.** Compare finalized frontiers and observed provisional changes across independently operated IP and radio gateways. Surface missing certified data, shallow reorgs, late inclusion, and equivocation evidence. Monitoring can detect divergence or repeated symptoms. It cannot prove why a miner omitted a transaction.

## Acceptance tests

1. Given valid competing histories with different verified work, all observers retain the certified frontier. Repeat across arrival orders, disconnection and restart.
2. Under quorum loss, accepted finalized history stays fixed. Verify restart, delayed certificates, and conflicting certificates before measuring liveness.
3. With certified inclusion requirements, omit a listed transaction from a proposed frontier and verify rejection. Repeat with unavailable disclosures, conflicting spends, and resource limits.
4. Run a calibrated comparison under equal total hash budgets and equal network latency, with and without checkpoints. Publish finalized reversals, provisional reversals, inclusion delays, and stall rates separately. A valid PoW-only history and an independently certified history answer different security questions.

The radio bench validates the lattice structure and proof dependencies. The Rust checkpoint verifier is a separate research component. Connecting them requires a canonical frontier and state validator plus durable voting and checkpoint state.
