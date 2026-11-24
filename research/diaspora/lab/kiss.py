"""Wrap public lab packets in AX.25 UI frames for a KISS TNC bench setup."""
import argparse
from pathlib import Path
import re

from packet import CHUNK, HEADER, MAX_PACKETS, decode, read_bounded

END, ESC, ESC_END, ESC_ESC = 0xC0, 0xDB, 0xDC, 0xDD
UI = b"\x03\xf0"
AX25_HEADER = 16
MAX_FRAME = 1 + AX25_HEADER + HEADER.size + CHUNK
MAX_CAPTURE = MAX_PACKETS * 2 * (MAX_FRAME * 2 + 2)


def address(call, last):
    match = re.fullmatch(r"([A-Z0-9]{1,6})(?:-([0-9]{1,2}))?", call)
    if not match or int(match[2] or 0) > 15:
        raise ValueError("use callsigns in CALL or CALL-SSID format")
    shifted = bytes(byte << 1 for byte in match[1].ljust(6).encode("ascii"))
    flags = 0x61 if last else 0xE0  # source extension bit, destination command bit
    return shifted + bytes([flags | (int(match[2] or 0) << 1)])


def wrap(packet, source, destination):
    if not HEADER.size < len(packet) <= HEADER.size + CHUNK:
        raise ValueError("invalid packet size")
    frame = b"\0" + address(destination, False) + address(source, True) + UI + packet
    return (
        bytes([END])
        + frame.replace(bytes([ESC]), bytes([ESC, ESC_ESC]))
               .replace(bytes([END]), bytes([ESC, ESC_END]))
        + bytes([END])
    )


def unwrap(capture, source, destination):
    prefix = b"\0" + address(destination, False) + address(source, True) + UI
    if not capture or capture[0] != END or capture[-1] != END:
        raise ValueError("capture must start and finish with a KISS delimiter")
    raw_frames = capture.split(bytes([END]))
    frames = 0
    for raw in raw_frames:
        if not raw:
            continue
        frames += 1
        if frames > MAX_PACKETS * 2:
            raise ValueError("too many KISS frames")
        if len(raw) > MAX_FRAME * 2:
            raise ValueError("oversized KISS frame")
        frame = bytearray()
        escaped = False
        for byte in raw:
            if escaped:
                if byte not in (ESC_END, ESC_ESC):
                    raise ValueError("invalid KISS escape")
                frame.append(END if byte == ESC_END else ESC)
                escaped = False
            elif byte == ESC:
                escaped = True
            else:
                frame.append(byte)
        if escaped or len(frame) > MAX_FRAME or not frame.startswith(prefix):
            raise ValueError("unexpected KISS port, callsigns, or UI header")
        yield bytes(frame[len(prefix):])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("wrap", "unwrap"))
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--source", required=True)
    parser.add_argument("--destination", required=True)
    args = parser.parse_args()
    try:
        if args.mode == "wrap":
            files = []
            for path in args.input.glob("*.pkt"):
                files.append(path)
                if len(files) > MAX_PACKETS * 2:
                    raise ValueError("too many packet files")
            packets = [read_bounded(path, HEADER.size + CHUNK) for path in sorted(files)]
            decode(packets)
            result = b"".join(wrap(packet, args.source, args.destination) for packet in packets)
        else:
            capture = read_bounded(args.input, MAX_CAPTURE)
            if len(capture) > MAX_CAPTURE:
                raise ValueError("oversized KISS capture")
            result = decode(unwrap(capture, args.source, args.destination))
        with args.output.open("xb") as output:
            output.write(result)
    except (OSError, ValueError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
