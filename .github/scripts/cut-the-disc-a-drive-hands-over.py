#!/usr/bin/env python3

import sys
from pathlib import Path

BYTES_PER_FRAME = 4


def main():
    recorded = Path(sys.argv[1]).read_bytes()
    shift = -int(sys.argv[2]) * BYTES_PER_FRAME
    disc = Path(sys.argv[3])

    if shift >= 0:
        disc.write_bytes(recorded[shift:] + bytes(shift))
    else:
        disc.write_bytes(bytes(-shift) + recorded[:shift])


if __name__ == "__main__":
    main()
