"""Offline, bounded packet framing for public laboratory payloads."""
import argparse
import hashlib
import math
from pathlib import Path
import struct

HEADER = struct.Struct(">4s32sHHI")
MAGIC = b"DSP1"
CHUNK = 128
MAX_SIZE = 16_384
MAX_PACKETS = MAX_SIZE // CHUNK


def encode(payload):
    if not 1 <= len(payload) <= MAX_SIZE:
        raise ValueError("payload length outside lab limits")
    count = math.ceil(len(payload) / CHUNK)
    digest = hashlib.sha256(payload).digest()
    return [
        HEADER.pack(MAGIC, digest, index, count, len(payload))
        + payload[index * CHUNK:(index + 1) * CHUNK]
        for index in range(count)
    ]


def decode(frames):
    chunks = {}
    identity = None
    for frame in frames:
        if not HEADER.size < len(frame) <= HEADER.size + CHUNK:
            raise ValueError("invalid packet size")
        magic, digest, index, count, size = HEADER.unpack_from(frame)
        if (magic != MAGIC or not 1 <= size <= MAX_SIZE
                or count != math.ceil(size / CHUNK) or index >= count):
            raise ValueError("invalid packet header")
        if identity is None:
            identity = (digest, count, size)
        if identity != (digest, count, size):
            raise ValueError("mixed transfers")
        chunk = frame[HEADER.size:]
        if len(chunk) != min(CHUNK, size - index * CHUNK):
            raise ValueError("invalid fragment length")
        if index in chunks and chunks[index] != chunk:
            raise ValueError("conflicting duplicate")
        chunks[index] = chunk
    if identity is None or len(chunks) != identity[1]:
        raise ValueError("incomplete transfer")
    payload = b"".join(chunks[index] for index in range(identity[1]))
    if hashlib.sha256(payload).digest() != identity[0]:
        raise ValueError("transfer checksum mismatch")
    return payload


def read_bounded(path, limit):
    with path.open("rb") as source:
        return source.read(limit + 1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("encode", "decode"))
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    try:
        if args.mode == "encode":
            frames = encode(read_bounded(args.source, MAX_SIZE))
            args.destination.mkdir()
            for index, frame in enumerate(frames):
                (args.destination / f"{index:03}.pkt").write_bytes(frame)
        else:
            files = []
            for path in args.source.glob("*.pkt"):
                files.append(path)
                if len(files) > MAX_PACKETS * 2:
                    raise ValueError("too many packet files")
            payload = decode(read_bounded(path, HEADER.size + CHUNK) for path in files)
            with args.destination.open("xb") as target:
                target.write(payload)
    except (OSError, ValueError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
