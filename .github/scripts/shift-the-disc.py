#!/usr/bin/env python3

# Writes what a disc image comes back as once a drive's read offset has been
# taken off it: the same samples, moved along the disc by that many frames,
# with silence standing in at whichever end the move reached past. A frame is
# one moment of sound on both channels, four bytes of it.
#
# A drive that reads ahead hands over what sits further along than what was
# asked for, so putting it right means taking the samples from further along
# and running out of disc at the end. A drive that reads behind is the same
# the other way round.

import sys
from pathlib import Path

BYTES_PER_FRAME = 4


def main():
    disc = Path(sys.argv[1]).read_bytes()
    # Not argparse: a negative offset reads as a flag to it.
    offset = int(sys.argv[2]) * BYTES_PER_FRAME
    written = Path(sys.argv[3])

    if offset >= 0:
        shifted = disc[offset:] + bytes(offset)
    else:
        shifted = bytes(-offset) + disc[:offset]

    written.write_bytes(shifted)


if __name__ == "__main__":
    main()
