#!/usr/bin/env python3

import sys
from pathlib import Path

BYTES_PER_FRAME = 4


def main():
    came_off = Path(sys.argv[1]).read_bytes()
    recorded = Path(sys.argv[2]).read_bytes()
    offset = int(sys.argv[3])

    if len(came_off) != len(recorded):
        sys.exit(f"{len(came_off)} bytes came off a disc holding {len(recorded)}")

    missed = abs(offset) * BYTES_PER_FRAME
    reaches = len(recorded) - missed if offset > 0 else missed

    reached = slice(None, reaches) if offset > 0 else slice(reaches, None)
    unreached = slice(reaches, None) if offset > 0 else slice(None, reaches)

    if came_off[reached] != recorded[reached]:
        sys.exit("what came off the disc is not what was recorded on it")

    if came_off[unreached] != bytes(missed):
        sys.exit("what the drive could not reach did not come back as silence")


if __name__ == "__main__":
    main()
