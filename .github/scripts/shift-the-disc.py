#!/usr/bin/env python3

# Cuts a disc that stands in for one read by a drive with a read offset.
#
# A drive that runs late hands over what sits further along the disc than what
# was asked for, so the disc it appears to be reading is one where the audio
# sits that much further along than it should. Delaying the recorded audio by
# that many frames makes such a disc; advancing it makes the disc a drive
# running early appears to read. Whichever end the move runs off, silence
# stands in, as there is nothing recorded past either edge of a disc.
#
# A frame is one moment of sound on both channels, four bytes of it.
#
# What this cannot stand in for is which way round a real drive is out. The
# shift is put in here by hand, so a run of this only ever says that correcting
# by the same amount brings the recorded audio back.

import sys
from pathlib import Path

BYTES_PER_FRAME = 4


def main():
    recorded = Path(sys.argv[1]).read_bytes()
    # Not argparse: a negative number of frames reads as a flag to it.
    frames = int(sys.argv[2]) * BYTES_PER_FRAME
    disc = Path(sys.argv[3])

    if frames >= 0:
        shifted = recorded[frames:] + bytes(frames)
    else:
        shifted = bytes(-frames) + recorded[:frames]

    disc.write_bytes(shifted)


if __name__ == "__main__":
    main()
