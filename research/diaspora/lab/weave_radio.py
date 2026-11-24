"""Mine, relay and verify complete Weave lab waves over a TCP KISS TNC."""
import argparse
import os
from pathlib import Path
import re
import socket
import struct
import sys
import tempfile
import time

from kiss import END, MAX_FRAME, address, unwrap, wrap
from packet import HEADER, MAX_SIZE, decode, encode

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "archive" / "weave-python"))
from weave import Cycle, Lattice, digest  # Reference consensus for this bench profile.

WIDTH = 3
BITS = 4
WAVE = struct.Struct(">4sI32s")
CYCLE = struct.Struct(">4I32sB")
MAGIC = b"DW01"
MAX_WAVES = 128
MAX_LABEL = 32
LABEL = re.compile(rb"[A-Za-z0-9 _./-]{1,32}")


def apply_wave(lattice, payload):
    if not WAVE.size <= len(payload) <= MAX_SIZE:
        raise ValueError("invalid wave envelope size")
    magic, height, parent = WAVE.unpack_from(payload)
    parents = lattice.frontier()
    if (magic != MAGIC or height != lattice.next_wave or height > MAX_WAVES
            or parent.hex() != digest(["DW01_FRONTIER", list(parents)])):
        raise ValueError("wave does not extend the local frontier")
    offset = WAVE.size
    cycles = []
    for slot in range(WIDTH):
        if offset + CYCLE.size > len(payload):
            raise ValueError("truncated Cycle")
        n1, n2, n3, binder, cycle_id, length = CYCLE.unpack_from(payload, offset)
        offset += CYCLE.size
        label = payload[offset:offset + length]
        if len(label) != length or not LABEL.fullmatch(label):
            raise ValueError("invalid public Cycle label")
        offset += length
        cycle = Cycle(height, slot, parents, lattice.archives_for(height, slot, parents),
                      label.decode("ascii"), (n1, n2, n3), binder, cycle_id.hex())
        lattice.validate(cycle)
        cycles.append(cycle)
    if offset != len(payload):
        raise ValueError("trailing wave data")
    for cycle in cycles:
        lattice.submit(cycle)


def mine_wave(lattice, label):
    label_bytes = label.encode("ascii")
    if not LABEL.fullmatch(label_bytes):
        raise ValueError(f"use 1–{MAX_LABEL} ASCII letters, numbers or simple punctuation")
    if lattice.next_wave > MAX_WAVES:
        raise ValueError("bench history is full")
    parent = bytes.fromhex(digest(["DW01_FRONTIER", list(lattice.frontier())]))
    payload = bytearray(WAVE.pack(MAGIC, lattice.next_wave, parent))
    for slot in range(WIDTH):
        cycle = lattice.mine(slot, label)
        payload.extend(CYCLE.pack(*cycle.strand_nonces, cycle.binder_nonce,
                                  bytes.fromhex(cycle.cycle_id), len(label_bytes)))
        payload.extend(label_bytes)
    return bytes(payload)


def load(path):
    lattice = Lattice(WIDTH, BITS)
    waves = []
    try:
        with path.open("rb") as source:
            for line in iter(lambda: source.readline(2 * MAX_SIZE + 2), b""):
                if len(waves) >= MAX_WAVES or not line.endswith(b"\n"):
                    raise ValueError("invalid bench state file")
                payload = bytes.fromhex(line.decode("ascii"))
                apply_wave(lattice, payload)
                waves.append(payload)
    except FileNotFoundError:
        pass
    return lattice, waves


def save(path, waves):
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as output:
        staged = Path(output.name)
        try:
            for payload in waves:
                output.write(payload.hex().encode("ascii") + b"\n")
            output.flush()
            os.fsync(output.fileno())
            output.close()
            staged.replace(path)
        finally:
            staged.unlink(missing_ok=True)


def receive(tnc, peer, local, timeout, accept):
    """Reassemble a matching transfer and return the first locally validated wave."""
    deadline = time.monotonic() + timeout
    raw = bytearray()
    framed = False
    transfers = {}
    while (remaining := deadline - time.monotonic()) > 0:
        tnc.settimeout(remaining)
        try:
            data = tnc.recv(4096)
        except TimeoutError:
            break
        if not data:
            raise ValueError("TNC closed before a valid wave arrived")
        for byte in data:
            if byte != END:
                if framed and len(raw) < 2 * MAX_FRAME:
                    raw.append(byte)
                else:
                    framed = False
                continue
            capture = bytes([END]) + raw + bytes([END]) if framed and raw else b""
            framed = True
            raw.clear()
            if not capture:
                continue
            try:
                fragment, = unwrap(capture, peer, local)
                if len(fragment) <= HEADER.size:
                    continue
                magic, sha, index, count, size = HEADER.unpack_from(fragment)
                if (magic != b"DSP1" or not 1 <= size <= MAX_SIZE
                        or not 1 <= count <= 128 or index >= count):
                    continue
                key = sha, count, size
                if key not in transfers and len(transfers) >= 4:
                    del transfers[next(iter(transfers))]
                parts = transfers.setdefault(key, {})
                if index in parts and parts[index] != fragment:
                    del transfers[key]
                    continue
                parts[index] = fragment
                if len(parts) == count:
                    del transfers[key]
                    payload = decode(parts.values())
                    accept(payload)
                    return payload
            except ValueError:
                continue
    raise ValueError("timed out waiting for a complete valid next wave")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--state", type=Path, required=True)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("status")
    mine = commands.add_parser("mine")
    mine.add_argument("label")
    for name in ("send", "receive"):
        command = commands.add_parser(name)
        command.add_argument("--tnc-port", type=int, default=8001)
        command.add_argument("--local", required=True)
        command.add_argument("--peer", required=True)
        command.add_argument("--timeout", type=int, default=90)
        if name == "send":
            command.add_argument("--height", type=int)
    args = parser.parse_args()
    try:
        lattice, waves = load(args.state)
        if args.command == "mine":
            payload = mine_wave(lattice, args.label)
            apply_wave(lattice, payload)
            save(args.state, [*waves, payload])
        elif args.command in ("send", "receive"):
            address(args.local, True)
            address(args.peer, True)
            if not 1 <= args.timeout <= 600:
                raise ValueError("timeout must be 1..600 seconds")
            if args.command == "send":
                height = args.height if args.height is not None else len(waves)
                if not 1 <= height <= len(waves):
                    raise ValueError("requested wave is absent")
                with socket.create_connection(("127.0.0.1", args.tnc_port), args.timeout) as tnc:
                    for frame in encode(waves[height - 1]):
                        tnc.sendall(wrap(frame, args.local, args.peer))
                print(f"Queued wave {height} to local TNC. Delivery is unconfirmed.")
                return
            with socket.create_connection(("127.0.0.1", args.tnc_port), args.timeout) as tnc:
                print("Listening for the next wave.", flush=True)
                payload = receive(tnc, args.peer, args.local, args.timeout,
                                  lambda candidate: apply_wave(lattice, candidate))
                save(args.state, [*waves, payload])
        print(f"height={lattice.next_wave - 1} frontier={digest(['DW01_FRONTIER', list(lattice.frontier())])}")
    except (OSError, ValueError, struct.error) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
