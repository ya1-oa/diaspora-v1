import dataclasses
import unittest
from weave import Lattice, InvalidCycle, demonstration


class LatticeTests(unittest.TestCase):
    def setUp(self):
        self.l = Lattice(width=3, difficulty_bits=5)

    def test_three_or_more_non_lane_frontier_dependencies(self):
        first = self.l.mine_wave()
        second = self.l.mine_wave()
        assert all(c.parents == tuple(sorted(x.cycle_id for x in first)) for c in second)
        assert all(len(c.parents) == 3 for c in second)
        self.assertTrue(self.l.verify())

    def test_scales_to_five_strands(self):
        l = Lattice(width=5, difficulty_bits=4)
        l.mine_wave()
        l.mine_wave()
        self.assertEqual(5, len(l.waves[-1][0].strand_nonces))
        self.assertTrue(l.verify())

    def test_missing_parent_rejected(self):
        c = self.l.mine(0, "a")
        with self.assertRaises(InvalidCycle):
            self.l.submit(dataclasses.replace(c, parents=c.parents[:-1]))

    def test_mutated_payload_breaks_parent_bound_pow(self):
        c = self.l.mine(0, "a")
        with self.assertRaises(InvalidCycle):
            self.l.submit(dataclasses.replace(c, payload="evil"))

    def test_missing_one_strand_rejected(self):
        c = self.l.mine(0, "a")
        with self.assertRaises(InvalidCycle):
            self.l.submit(dataclasses.replace(c, strand_nonces=c.strand_nonces[:-1]))

    def test_duplicate_slot_rejected(self):
        self.l.submit(self.l.mine(0, "a"))
        with self.assertRaises(InvalidCycle):
            self.l.submit(self.l.mine(1, "b").__class__(**{
                **dataclasses.asdict(self.l.pending[0]), "slot": 0
            }))

    def test_archives_deterministic_and_validated(self):
        self.l.mine_wave()
        c = self.l.mine(0, "a")
        self.assertGreater(len(c.archives), 0)
        with self.assertRaises(InvalidCycle):
            self.l.submit(dataclasses.replace(c, archives=()))

    def test_rewrite_propagates_cross_lattice(self):
        for _ in range(4):
            self.l.mine_wave()
        old = self.l.waves[0][0]
        cone = self.l.descendant_cone(old.cycle_id)
        # All Cycles of every later wave depend on all preceding cycles.
        self.assertEqual(3 * 3, len(cone))
        self.assertTrue(self.l.verify())

    def test_remine_ancestor_cannot_reuse_descendant_proofs(self):
        self.l.mine_wave()
        self.l.mine_wave()
        alternate = Lattice(width=3, difficulty_bits=5)
        replacement = alternate.mine(0, "alternate-history")
        assert replacement.cycle_id != self.l.waves[0][0].cycle_id
        self.l.waves[0] = (replacement, *self.l.waves[0][1:])
        with self.assertRaises(InvalidCycle):
            self.l.verify()

    def test_unique_work_accounting(self):
        for _ in range(3):
            self.l.mine_wave()
        self.assertEqual(3 * 3 * 4 * 2**5, self.l.counted_work_units())

    def test_demo(self):
        result = demonstration(width=3, waves=2, difficulty_bits=4)
        self.assertTrue(result["valid"])
        self.assertGreater(result["reference_edges"], 0)


if __name__ == "__main__":
    unittest.main()
