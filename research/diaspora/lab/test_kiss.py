import unittest

from kiss import END, ESC, address, unwrap, wrap
from packet import decode, encode


class KissTest(unittest.TestCase):
    source = "LABA-1"
    destination = "LABB-2"

    def test_round_trip_with_reserved_bytes(self):
        payload = bytes([END, ESC, 0, 42]) * 200
        capture = b"".join(wrap(packet, self.source, self.destination)
                           for packet in reversed(encode(payload)))
        self.assertEqual(decode(unwrap(capture, self.source, self.destination)), payload)

    def test_rejects_wrong_receiver(self):
        capture = wrap(encode(b"test")[0], self.source, self.destination)
        with self.assertRaisesRegex(ValueError, "unexpected"):
            list(unwrap(capture, self.source, "OTHER"))

    def test_rejects_invalid_escape(self):
        capture = bytes([END, ESC, 0xEF, END])
        with self.assertRaisesRegex(ValueError, "escape"):
            list(unwrap(capture, self.source, self.destination))

    def test_callsign_encoding(self):
        self.assertEqual(address("LABA-1", True), b"\x98\x82\x84\x82\x40\x40\x63")
        for invalid in ("", "A-16", "TOOLONG"):
            with self.assertRaises(ValueError):
                address(invalid, False)
