#!/usr/bin/env python3

# Writes the disc the tests rip: ten seconds of CD audio and a cue sheet.
#
# Not noise. libcdio-paranoia works out which way round a drive returns its
# samples by running an FFT over the audio and taking whichever way looks
# audible, so audio with nothing audible in it leaves that to chance, and a
# disc that came back byte-swapped once in four runs is what led here.
#
# The two channels carry different notes, so samples arriving in the wrong
# order shows up as more than a byte swap.
#
# The same disc every time. This asserts that a disc can be ripped as
# uncompressed FLAC, and it should fail when that stops being true rather than
# when a generator happens to produce something awkward.

import math
import struct
import sys
from pathlib import Path

SAMPLE_RATE = 44100
BYTES_PER_SECTOR = 2352
SECTORS = 750
FRAMES = SECTORS * BYTES_PER_SECTOR // 4

LEFT = 220.0
RIGHT = 330.0
# Enough harmonics to look like an instrument rather than a test tone.
HARMONICS = ((1, 1.0), (2, 0.5), (3, 0.25))
# Short of full scale, so summing the harmonics cannot clip.
AMPLITUDE = 0.6 * 32767


def voice(frame, fundamental):
    t = frame / SAMPLE_RATE
    # A slow swell, so no long stretch of the disc is the same as any other.
    envelope = 0.55 + 0.45 * math.sin(2 * math.pi * 0.5 * t)
    tone = sum(
        weight * math.sin(2 * math.pi * fundamental * harmonic * t)
        for harmonic, weight in HARMONICS
    ) / sum(weight for _, weight in HARMONICS)

    return int(AMPLITUDE * envelope * tone)


def main():
    directory = Path(sys.argv[1])
    directory.mkdir(parents=True, exist_ok=True)

    audio = bytearray()
    for frame in range(FRAMES):
        audio += struct.pack("<hh", voice(frame, LEFT), voice(frame, RIGHT))

    (directory / "disc.bin").write_bytes(bytes(audio))
    (directory / "disc.cue").write_text(
        'FILE "disc.bin" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n'
    )


main()
