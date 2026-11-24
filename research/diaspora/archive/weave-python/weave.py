"""Diaspora experimental coupled-cycle lattice.

Reference simulator, not production consensus or a claimed 51% defense.
Three or more Cycles advance per wave; each must commit to all the prior
wave's Cycles and complete a separate PoW challenge for each parent, plus
one binding PoW challenge. There are NO persistent lanes.

This is a closed-world, synchronously produced lattice. Network consensus,
leader/witness selection, asynchronous liveness, transactions, and finality
require further protocol work.
"""
from __future__ import annotations
from dataclasses import dataclass, replace
from hashlib import sha3_256
from typing import Iterable
import json


def digest(obj: object) -> str:
    return sha3_256(json.dumps(obj, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def meets_difficulty(hexhash: str, difficulty_bits: int) -> bool:
    if not (1 <= difficulty_bits <= 24):
        raise ValueError("difficulty_bits must be in [1, 24] for simulator")
    return int(hexhash, 16) < (1 << (256 - difficulty_bits))


@dataclass(frozen=True)
class Cycle:
    wave: int
    slot: int                         # Wave-local position, NOT a permanent lane
    parents: tuple[str, ...]         # All preceding frontier Cycles; unordered as history
    archives: tuple[str, ...]        # Deterministically sampled historical witnesses
    payload: str                     # Application data commitment
    strand_nonces: tuple[int, ...]    # One parent-bound PoW per weave dependency
    binder_nonce: int                # Work bound to *all* parent proofs
    cycle_id: str


class InvalidCycle(ValueError):
    pass


class Lattice:
    def __init__(self, width: int = 3, difficulty_bits: int = 8):
        if not (3 <= width <= 12):
            raise ValueError("width must be 3..12")
        if not (1 <= difficulty_bits <= 24):
            raise ValueError("difficulty must be in 1..24")
        self.width = width
        self.difficulty_bits = difficulty_bits
        self.genesis = tuple(digest(["DIASPORA_GENESIS", width, i]) for i in range(width))
        self.waves: list[tuple[Cycle, ...]] = []
        self.pending: dict[int, Cycle] = {}

    @property
    def next_wave(self) -> int:
        return len(self.waves) + 1

    def frontier(self) -> tuple[str, ...]:
        return tuple(sorted(c.cycle_id for c in self.waves[-1])) if self.waves else tuple(sorted(self.genesis))

    def _older_pool(self, wave: int) -> tuple[str, ...]:
        # Historical archives must predate the immediately preceding wave.
        ids = list(self.genesis)
        for past_wave in self.waves[:max(0, wave - 2)]:
            ids.extend(c.cycle_id for c in past_wave)
        return tuple(sorted(set(ids)))

    def archives_for(self, wave: int, slot: int, parents: tuple[str, ...]) -> tuple[str, ...]:
        pool = self._older_pool(wave)
        count = min(3, len(pool))
        selected = []
        seed = digest(["archive-seed", wave, slot, list(parents)])
        # Deterministic without a miner-chosen VRF key; not unpredictable.
        for i in range(count):
            rest = [candidate for candidate in pool if candidate not in selected]
            idx = int(digest([seed, i]), 16) % len(rest)
            selected.append(rest[idx])
        return tuple(selected)

    @staticmethod
    def _base(wave: int, slot: int, parents: tuple[str, ...],
              archives: tuple[str, ...], payload: str) -> list:
        return ["DIASPORA_CYCLE_V1", wave, slot, list(parents), list(archives), payload]

    @staticmethod
    def _strand_hash(base: list, index: int, parent: str, nonce: int) -> str:
        return digest(["STRAND", base, index, parent, nonce])

    @staticmethod
    def _binder_hash(base: list, strands: tuple[str, ...], nonce: int) -> str:
        return digest(["BINDER", base, list(strands), nonce])

    @staticmethod
    def _id(base: list, nonces: tuple[int, ...], binder_nonce: int) -> str:
        return digest(["CYCLE_ID", base, list(nonces), binder_nonce])

    def mine(self, slot: int, payload: str, start_nonce: int = 0) -> Cycle:
        if slot not in range(self.width) or slot in self.pending:
            raise ValueError("slot out of range or already occupied")
        wave = self.next_wave
        parents = self.frontier()
        archives = self.archives_for(wave, slot, parents)
        base = self._base(wave, slot, parents, archives, payload)
        nonces = []
        strand_hashes = []
        for index, parent in enumerate(parents):
            nonce = start_nonce
            while not meets_difficulty(self._strand_hash(base, index, parent, nonce), self.difficulty_bits):
                nonce += 1
            nonces.append(nonce)
            strand_hashes.append(self._strand_hash(base, index, parent, nonce))
        nonce = start_nonce
        strands = tuple(strand_hashes)
        while not meets_difficulty(self._binder_hash(base, strands, nonce), self.difficulty_bits):
            nonce += 1
        proof_nonces = tuple(nonces)
        result = Cycle(wave, slot, parents, archives, payload, proof_nonces, nonce,
                       self._id(base, proof_nonces, nonce))
        self.validate(result)
        return result

    def validate(self, cycle: Cycle) -> None:
        if cycle.wave != self.next_wave:
            raise InvalidCycle("wrong wave")
        if cycle.slot not in range(self.width):
            raise InvalidCycle("invalid wave-local slot")
        parents = self.frontier()
        if cycle.parents != parents or len(set(cycle.parents)) != self.width:
            raise InvalidCycle("missing or incorrect frontier dependency")
        archives = self.archives_for(cycle.wave, cycle.slot, parents)
        if cycle.archives != archives:
            raise InvalidCycle("incorrect historical archive references")
        if len(cycle.strand_nonces) != self.width:
            raise InvalidCycle("one strand proof required per parent")
        if any(type(n) is not int or n < 0 for n in (*cycle.strand_nonces, cycle.binder_nonce)):
            raise InvalidCycle("invalid proof nonce")
        base = self._base(cycle.wave, cycle.slot, parents, archives, cycle.payload)
        strand_hashes = tuple(self._strand_hash(base, i, parent, nonce)
                              for i, (parent, nonce) in enumerate(zip(parents, cycle.strand_nonces)))
        if not all(meets_difficulty(h, self.difficulty_bits) for h in strand_hashes):
            raise InvalidCycle("invalid strand proof")
        if not meets_difficulty(self._binder_hash(base, strand_hashes, cycle.binder_nonce), self.difficulty_bits):
            raise InvalidCycle("invalid cross-strand binding proof")
        if cycle.cycle_id != self._id(base, cycle.strand_nonces, cycle.binder_nonce):
            raise InvalidCycle("incorrect Cycle identifier")

    def submit(self, cycle: Cycle) -> None:
        self.validate(cycle)
        if cycle.slot in self.pending:
            raise InvalidCycle("duplicate wave-local slot")
        self.pending[cycle.slot] = cycle
        if len(self.pending) == self.width:
            self.waves.append(tuple(self.pending[i] for i in range(self.width)))
            self.pending.clear()

    def mine_wave(self, payload_prefix: str = "tx") -> tuple[Cycle, ...]:
        wave = self.next_wave
        for slot in range(self.width):
            self.submit(self.mine(slot, f"{payload_prefix}/{wave}/{slot}"))
        return self.waves[-1]

    def verify(self) -> bool:
        shadow = Lattice(self.width, self.difficulty_bits)
        for wave in self.waves:
            for cycle in wave:
                shadow.submit(cycle)
        for slot in sorted(self.pending):
            shadow.submit(self.pending[slot])
        return shadow.frontier() == self.frontier() and shadow.next_wave == self.next_wave

    def descendant_cone(self, cycle_id: str) -> set[str]:
        """All Cycles invalidated by changing the identified Cycle ID.

        Traverses ALL mandatory parents and archive edges. A changed source
        invalidates a child commitment regardless of whether it is an archive.
        """
        all_cycles = [c for wave in self.waves for c in wave] + list(self.pending.values())
        descendants: set[str] = set()
        queue = [cycle_id]
        while queue:
            changed = queue.pop()
            for child in all_cycles:
                if child.cycle_id not in descendants and changed in (*child.parents, *child.archives):
                    descendants.add(child.cycle_id)
                    queue.append(child.cycle_id)
        return descendants

    def counted_work_units(self) -> int:
        # Each valid puzzle counts ONCE, even if referenced many times.
        # This measures expected SHA3 trials, NOT increased security threshold.
        self.verify()
        cycles = sum(len(w) for w in self.waves) + len(self.pending)
        return cycles * (self.width + 1) * (1 << self.difficulty_bits)

    def graph(self) -> dict:
        return {"width": self.width, "difficulty_bits": self.difficulty_bits,
                "genesis": list(self.genesis),
                "cycles": [{"id": c.cycle_id, "wave": c.wave, "slot": c.slot,
                            "parents": list(c.parents), "archives": list(c.archives),
                            "payload": c.payload, "strand_nonces": list(c.strand_nonces),
                            "binder_nonce": c.binder_nonce}
                           for w in self.waves for c in w]}


def demonstration(width: int = 3, waves: int = 4, difficulty_bits: int = 8) -> dict:
    lattice = Lattice(width, difficulty_bits)
    for _ in range(waves):
        lattice.mine_wave()
    origin = lattice.waves[0][0]
    return {"width": width, "waves": waves, "cycle_count": width * waves,
            "reference_edges": sum(len(c.parents) + len(c.archives) for w in lattice.waves for c in w),
            "expected_unique_work_hashes": lattice.counted_work_units(),
            "rewritten_cycle": origin.cycle_id,
            "invalidated_descendant_cycles": len(lattice.descendant_cone(origin.cycle_id)),
            "valid": lattice.verify(), "sample_graph": lattice.graph()}


if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser()
    p.add_argument("--width", type=int, default=3)
    p.add_argument("--waves", type=int, default=4)
    p.add_argument("--bits", type=int, default=8)
    p.add_argument("--output", choices=["summary", "full"], default="summary")
    args = p.parse_args()
    result = demonstration(args.width, args.waves, args.bits)
    if args.output == "summary":
        result.pop("sample_graph")
    print(json.dumps(result, indent=2))
