import socket
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path

from kiss import wrap
from packet import encode
from weave_radio import BITS, WIDTH, Lattice, apply_wave, load, mine_wave, receive

LAB = Path(__file__).resolve().parent


class WeaveRadioTest(unittest.TestCase):
    def test_invalid_proof_and_missing_parent_leave_state_intact(self):
        miner = Lattice(WIDTH, BITS)
        verifier = Lattice(WIDTH, BITS)
        first = mine_wave(miner, "first")
        corrupted = bytearray(first)
        corrupted[-1] ^= 1
        with self.assertRaises(ValueError):
            apply_wave(verifier, corrupted)
        self.assertEqual(verifier.next_wave, 1)
        self.assertFalse(verifier.pending)
        apply_wave(miner, first)
        second = mine_wave(miner, "second")
        with self.assertRaisesRegex(ValueError, "local frontier"):
            apply_wave(verifier, second)
        apply_wave(verifier, first)
        apply_wave(verifier, second)
        apply_wave(miner, second)
        self.assertEqual(miner.frontier(), verifier.frontier())

    def test_radio_receive_tolerates_split_frames_duplicates_and_order(self):
        miner = Lattice(WIDTH, BITS)
        verifier = Lattice(WIDTH, BITS)
        payload = mine_wave(miner, "radio test")
        parts = encode(payload)
        stream = b"junk" + b"".join(wrap(part, "LABA", "LABB")
                                   for part in [parts[-1], *reversed(parts)])
        rx, tx = socket.socketpair()
        with rx, tx:
            for i in range(0, len(stream), 31):
                tx.sendall(stream[i:i + 31])
            result = receive(rx, "LABA", "LABB", 1, lambda data: apply_wave(verifier, data))
        self.assertEqual(result, payload)
        self.assertEqual(verifier.next_wave, 2)

    def test_two_cli_nodes_over_tcp_kiss(self):
        with tempfile.TemporaryDirectory() as tmp, socket.socket() as tnc:
            sender, receiver = (Path(tmp) / name for name in ("a.state", "b.state"))
            tnc.bind(("127.0.0.1", 0))
            tnc.listen(2)
            tnc.settimeout(5)
            port = str(tnc.getsockname()[1])

            def command(state, *args):
                return [sys.executable, str(LAB / "weave_radio.py"), "--state", str(state), *args]

            mined = subprocess.check_output(command(sender, "mine", "wave one"))
            errors = []

            def relay():
                try:
                    rx, _ = tnc.accept()
                    with rx:
                        tx, _ = tnc.accept()
                        with tx:
                            while data := tx.recv(97):
                                rx.sendall(data)
                except OSError as error:
                    errors.append(error)

            worker = threading.Thread(target=relay, daemon=True)
            worker.start()
            with subprocess.Popen(command(receiver, "receive", "--tnc-port", port,
                                           "--local", "LABB", "--peer", "LABA", "--timeout", "5"),
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
                self.assertEqual(process.stdout.readline(), b"Listening for the next wave.\n")
                subprocess.run(command(sender, "send", "--tnc-port", port,
                                       "--local", "LABA", "--peer", "LABB"),
                               check=True, capture_output=True, timeout=5)
                output, error = process.communicate(timeout=6)
                self.assertEqual(process.returncode, 0, error)
                self.assertEqual(output, mined)
            worker.join(5)
            self.assertFalse(worker.is_alive())
            self.assertFalse(errors)
            self.assertEqual(load(sender)[0].frontier(), load(receiver)[0].frontier())
