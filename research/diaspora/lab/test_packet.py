import unittest

from packet import MAX_SIZE, decode, encode


class PacketTest(unittest.TestCase):
    def setUp(self):
        self.payload = bytes(range(251)) * 3
        self.frames = encode(self.payload)

    def test_reorder_and_duplicate(self):
        self.assertEqual(decode([self.frames[-1], *reversed(self.frames)]), self.payload)

    def test_loss_is_reported(self):
        with self.assertRaisesRegex(ValueError, "incomplete"):
            decode(self.frames[:-1])

    def test_corruption_is_reported(self):
        first = self.frames[0]
        with self.assertRaisesRegex(ValueError, "checksum"):
            decode([first[:-1] + bytes([first[-1] ^ 1]), *self.frames[1:]])

    def test_mixed_transfers_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "mixed"):
            decode([self.frames[0], encode(b"second transfer")[0]])

    def test_conflicting_duplicates_are_rejected(self):
        first = self.frames[0]
        with self.assertRaisesRegex(ValueError, "conflicting"):
            decode([first, first[:-1] + bytes([first[-1] ^ 1])])

    def test_limits(self):
        for length in (1, 128, 129, MAX_SIZE):
            payload = b"x" * length
            self.assertEqual(decode(encode(payload)), payload)
        for payload in (b"", b"x" * (MAX_SIZE + 1)):
            with self.assertRaises(ValueError):
                encode(payload)
        for frame in (b"", self.frames[0][:-1], self.frames[0] + b"x"):
            with self.assertRaises(ValueError):
                decode([frame])
