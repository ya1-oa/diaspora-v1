# Diaspora production target

I started Diaspora to build peer-to-peer money for the African diaspora. I want someone to hold their own keys, verify a payment, and stay connected when their usual internet connection fails. Each of those promises needs a test I can run on my own machines.

## Ledger and security

The inherited Bitcoin Core node is the wallet and transaction baseline. Weave is a second ledger design. In the original Loom draft, each Cycle cites three VRF-selected historical anchors and the protocol proposes cumulative historical continuity. In L0, each Cycle commits to every Cycle of the prior wave with a separate parent-bound proof and a final binding proof. The older Warp references check history and earn zero work credit. Those are two different designs. The archived drafts preserve both.

The first production specification must freeze canonical bytes, hash domains, eligible parents, difficulty, fork choice, transaction ordering, genesis, and network identity. Equivalent complete views must select the same history independent of arrival order. Credit each valid proof once across shared ancestry. Keep wallet signing independent of fork choice and relay credentials. All nodes must reject invalid transactions, invalid work, and invalid historical references before applying state. Persist validated state so restart and snapshot replay reproduce the same commitment. Start on an isolated development network with dedicated address prefixes and genesis before considering any public deployment.

### Release gates

| Subsystem | Acceptance before release |
| --- | --- |
| Weave ledger | Independent decoders agree byte for byte. Nodes converge after duplicate or delayed delivery and after restart. Invalid Cycles cannot mutate the UTXO set. Document reorganization bounds and what the wallet displays when a payment loses confirmation. |
| Majority resistance | Define which resource is controlled and publish assumptions. Compare reversal frequency, liveness, settlement time, and hash cost against linear PoW under equal total work and elapsed time. Report results at 30%, 45%, 51%, 55%, 70%, and 99%, with confidence intervals and failed runs. Independent validation must reproduce any threshold claim. |
| Witness finality, if adopted | Specify the independent source of authority, quorum and intersection, vote locking, durable equivocation records, validator replacement and recovery. Verify conflicting histories cannot both finalize inside the assumed fault bound. Measure partition stalls and eventual recovery separately. |
| Wallet | Dedicated network identity, descriptor creation, watch-only mode, offline signing with independently verified recipient and fee, encrypted backup, recovery on a clean machine, and clear pending, confirmed and reorg statuses. The current lab exercises Bitcoin Core regtest wallet RPCs. |
| Fractal Mesh | Begin with bounded relays for public transaction or compact announcement data. Specify framing, delivery limits, retry policies, persistence and authentication. Demonstrate duplicate suppression, recovery after loss, gateway restart and reconnection through IP. Evaluate D-AX.25, LoRa and receive-only satellite as separate adapters. |
| Fracture and Heal | Preserve spend safety across disconnected groups. Initially queue transactions during partition and show them as unconfirmed. Ship local settlement only with a defined trust model and reconciliation rule that handles conflicting spends deterministically. |
| Node roles and economics | Define Unity, Super, Relay and Mining node responsibilities and distinguish transport receipts from consensus work. Prove relay accounting is reproducible and review supply, subsidies and Proof-of-Relay incentives before freezing parameters. The July 2025 design's 21M cap and 90/10 split remain historical proposals. |
| Township and Guilds | Establish independent state, authorization, asset issuance and lossless exits for any sub-Weaves. Define governance actions and upgrades separately from monetary validation. |
| Soma, Store and Echo | Keep communications and knowledge data outside base-layer consensus unless consensus needs a commitment. Document protocol versions, key recovery, metadata exposure, availability, and deletion semantics. Demonstrate a replicated encrypted store and a reproducible read on a clean node. |
| Engine, Society and Storyteller | Define deterministic execution, resource limits, metering and governance authorization before accepting contracts or paid jobs. Produce a read-only history view before adding write access. Keep STBL, real-world assets, oracles and compute scheduling behind dedicated release decisions. |
| RootDevice and MelaninOS | Inventory supported radios and host devices. Test key backup and restore, battery budgeting, interrupted updates and recoverable boot on real hardware. |
| Cryptography | Review archived NTRU-family, Kyber-family and SHA3 proposals independently. Pin algorithms, versions, encodings, and test vectors before implementing any new wire format. Do not circulate private keys through a transport adapter. |

## How I read the results

Changing an early Cycle invalidated 9 descendants at width 3 and 15 at width 5 in four-wave L0 runs. That establishes causal coupling. Both honest and adversarial miners incur the additional proof costs. The checked-in majority simulation is a linear baseline and the current L0 model has no global fork choice. A higher security threshold remains an open research result.
