#!/usr/bin/env python3

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
HARMONICS = ((1, 1.0), (2, 0.5), (3, 0.25))
AMPLITUDE = 0.6 * 32767


def voice(frame, fundamental):
    t = frame / SAMPLE_RATE
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
