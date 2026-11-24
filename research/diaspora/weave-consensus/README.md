# Weave fork choice and network file v1

Mining orders the history after the last checkpoint. Checkpoints require a separate validator resource. The goal is for an already-finalized payment to survive a majority miner producing a heavier competing history, as long as the independent signing assumptions hold. This crate is a research fork-choice verifier alongside the C11 Weave model in the Bitcoin Core-derived repository.

## Network identity

Pin the complete network file or its hash independently of peers. The file is the following sequence of bytes with no trailing data or padding.

| Field | Encoding |
| --- | --- |
| Domain | ASCII `DIASPORA:WEAVE:NETWORK:V1` followed by one NUL byte |
| Ledger rules ID | 32 bytes identifying the exact ledger rules |
| Genesis frontier | 32-byte commitment |
| Checkpoint interval | Nonzero unsigned 64-bit integer, big-endian |
| Number of validators | Unsigned 16-bit integer, big-endian. Range 4 through 256 |
| Validator keys | Ordered 32-byte Ed25519 public keys. Index starts at zero |

The network ID is SHA-256 of the entire file. The interval is measured in completed frontiers. The genesis frontier is finalized at height zero. A different key order, interval, genesis or ledger rules ID yields a different network. Reject invalid, weak and equivalent keys. Independent operation of the signers remains an operational assumption.

## Rules

1. Supply fully ledger-validated histories of completed frontiers. Each frontier ID must commit to its full frontier and preceding frontier. The ledger validator must verify canonical bytes, all transactions, work, and unique proof accounting across DAG ancestry. The crate checks frontier links, duplicate frontier IDs, work overflow and positive work. It accepts the validation caller's reported work.
2. Discard histories that omit the local finalized frontier at its exact height. Among survivors pick greatest cumulative verified work. At equal work pick the lexicographically smallest tip ID. A node can move between compatible provisional tips. If nothing is compatible, await data for a compatible history.
3. Advance one checkpoint interval at a time. A certificate must name the preceding finalized checkpoint and identify the target frontier in an available validated descendant history. Count each configured signer once and require strictly more than two-thirds of the keys.
4. Sign the ASCII domain `DIASPORA:WEAVE:CHECKPOINT:V1` followed by a NUL byte, then the 32-byte network ID, 32-byte genesis commitment, parent height (uint64 big-endian), parent ID, target height (same encoding) and target ID. The verifier checks each Ed25519 signature against that preimage. It verifies signed fields before considering finality.
5. A valid certificate for a different target at an already-finalized height latches a halt once the node sees it. An exact replay is idempotent. Keep prior checkpoint links for conflict detection. A future certificate cannot skip an unrecognized parent.

## What the security claim depends on

See the [finalized-history resistance proof](RESISTANCE.md) for the exact witness bound and implementation assumptions.

For a fixed set of `n` validators, quorum `q = floor(2n/3) + 1` means any two quorums intersect in at least `2q - n` validators. With fewer than one-third Byzantine validators, that intersection contains an honest signer. For safety, honest signers must durably refuse to endorse incompatible targets at the same checkpoint height and must only extend a certified parent they have verified. For four validators the quorum is three and any two quorums overlap in at least two signers.

Under those assumptions, additional mining work alone cannot reverse a finalized checkpoint. Majority miners can still control unfinalized ordering, withhold data, censor or stall progress. Too few available signers can stall finality. Splitting honest votes across competing proposals can stall a fixed one-vote rule even after communication returns. A live network needs a reviewed BFT round and locking protocol, durable voter and node state, and independent key operations. A local halt detects conflicting evidence once received and cannot retroactively prevent a split that has already happened. See [CometBFT's consensus specification](https://github.com/cometbft/cometbft/blob/main/spec/consensus/consensus.md) for one reviewed reference design for round changes and locking.

## Run

```sh
cargo test --locked --manifest-path research/diaspora/weave-consensus/Cargo.toml
cargo clippy --locked --manifest-path research/diaspora/weave-consensus/Cargo.toml --all-targets -- -D warnings
```

The tests use deterministic keys and synthetic validated frontiers. They check deterministic selection, heavier incompatible histories, provisional reorganization, partition stalls, certificate replay and conflict handling, invalid votes, malformed network files, and quorum intersection. They do not estimate hash-share reversal probabilities. Connect this crate to canonical Weave frontier validation before building a calibrated partition and mining workload. It does not change Bitcoin Core consensus.
