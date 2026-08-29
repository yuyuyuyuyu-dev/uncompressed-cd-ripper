#!/usr/bin/env python3

# Holds what came off a disc against what was recorded on it, where the disc
# was cut to stand in for one a drive with a read offset hands over.
#
# Everything the drive could reach has to come back as it was recorded. What
# it could not reach has to come back as silence: a drive running late is
# already past the end of the disc at the end of the last track, and one
# running early is short of the start of the disc at the beginning of the
# first, and there is nothing recorded past either edge to hand over.
#
# A frame is one moment of sound on both channels, four bytes of it.

import sys
from pathlib import Path

BYTES_PER_FRAME = 4


def main():
    came_off = Path(sys.argv[1]).read_bytes()
    recorded = Path(sys.argv[2]).read_bytes()
    # Not argparse: a negative number of frames reads as a flag to it.
    offset = int(sys.argv[3])

    if len(came_off) != len(recorded):
        sys.exit(f"{len(came_off)} bytes came off a disc holding {len(recorded)}")

    # Counted from the front rather than back from the end, so that a drive
    # that is not out at all reaches the whole disc instead of none of it.
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
