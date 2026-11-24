# Living Tapestry and Storyteller

The original integration drafts describe a 1,024 by 1,024 world with elemental terrain, energy, Seed life, network structures, and a reference from each cell to its creating Weave. They specify the CA interfaces but leave the transition rules incomplete. This Rust draft implements a sparse grid and introduces explicit draft rules for growth, maturation, provenance, and rollback. It shares the existing Rust fork-choice verifier.

## Run the first draft

From the repository root:

```sh
cargo test --locked --all-targets --manifest-path research/diaspora/weave-consensus/Cargo.toml
cargo run --locked --manifest-path research/diaspora/weave-consensus/Cargo.toml --example storyteller -- tapestry.html
```

Open `tapestry.html` in any browser. The standalone file contains three observations in order. The first frontier grows a Conduit. A different provisional frontier replaces it and Storyteller leaves the original snapshot in the archive, marks where the structure was orphaned, and shows its creating frontier. A later observation records a heavier branch that disagrees with the certified frontier. That branch stays out of the selected world.

## Draft ecology

Earth, Water, Air and Fire provide neighborhood conditions. A Seed gains energy near Water and loses energy without it. At the growth threshold, Fire, Air and Earth can mature it into a Forge, Conduit or Vault respectively, in that precedence order. Connected Seeds and mature structures form an organism. The first 12 by 12 cells render on the board. The provenance detail below the board includes the full sparse world. Void clears a cell. Every occupied cell keeps the ID of the frontier that placed it, including after a Seed matures.

Each frontier supplies ordered impacts. One synchronous tick follows each frontier. Given identical history and impacts, every replay has the same result. An alternate history can yield a different visible world, or it can yield visually identical cells with different provenance. The signed frontier determines which history is accepted. The ecology shows its consequences.

## Observation boundary

A node can report competing frontiers as they arrive, provisional reorganizations, branches rejected by a pinned checkpoint, and a verifier halt on conflicting certificates. A private or unseen fork does not appear. A fork by itself does not identify an attacker or establish intent. Orphan markers show cells whose creating frontier fell out of the selected path. Other changes, such as cells aging or a structure reverting because its surroundings changed, can be inspected by comparing snapshots.

The demo uses synthetic prevalidated frontiers and fixture signatures. The observer API emits an observation synchronously when called. There is no live peer feed, on-disk event journal, or automatic browser refresh yet. The HTML archive can be copied to other readers. It is a record of one observer's view, not a signed ledger proof.

The ledger integration must derive the effects from actual authenticated Weave events and bind them to the frontier before calling the observer. Human-authored actions, independent observation logs, healing between disconnected pockets, and network duties for structures are follow-on protocol work. They do not yet change consensus.
