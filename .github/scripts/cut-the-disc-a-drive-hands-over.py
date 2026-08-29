#!/usr/bin/env python3

# Cuts the disc a drive with a read offset appears to be reading.
#
# Correcting for an offset takes the samples that far further along the reading
# than the sectors they were asked for, so a disc that needs correcting by that
# much is one where the recorded audio sits that far further along it. Moving
# the audio and moving the reading are opposite operations, which is why the
# shift below is the offset turned round.
#
# Whichever end of the disc the move runs off, silence stands in: there is
# nothing recorded past either edge.
#
# The offset given is the drive's own, the same number the app is told, so that
# nothing has to keep two opposite numbers straight.
#
# A frame is one moment of sound on both channels, four bytes of it.
#
# What this cannot stand in for is which way round a real drive is out, since
# the shift is put in here by hand. A run of this only ever says that
# correcting by the same amount brings the recorded audio back.

import sys
from pathlib import Path

BYTES_PER_FRAME = 4


def main():
    recorded = Path(sys.argv[1]).read_bytes()
    # Not argparse: a negative read offset reads as a flag to it. Turned round
    # because the audio sits the opposite way along the disc from the way the
    # drive is out.
    shift = -int(sys.argv[2]) * BYTES_PER_FRAME
    disc = Path(sys.argv[3])

    if shift >= 0:
        disc.write_bytes(recorded[shift:] + bytes(shift))
    else:
        disc.write_bytes(bytes(-shift) + recorded[:shift])


if __name__ == "__main__":
    main()
