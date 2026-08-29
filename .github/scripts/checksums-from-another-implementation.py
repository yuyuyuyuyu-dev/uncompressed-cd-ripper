# Asks ARver, which verifies rips against AccurateRip for a living, what a
# track comes to, and prints the two checksums as eight hex digits each.
#
# Only the part of ARver that works checksums out is reached for. The rest of
# it wants a CD drive and a network, neither of which a job has, and reaching
# it would drag in libraries that have to be built against libcdio.
#
# No shebang: this has to run under the interpreter ARver was installed into,
# not whichever python happens to be first on the path.

import sys

from arver.audio.checksums import get_checksums


def main():
    path = sys.argv[1]
    # Which track this is of how many. AccurateRip leaves a stretch out of the
    # first track on a disc and another out of the last, so a checksum cannot
    # be worked out without knowing whether this is either of them.
    track = int(sys.argv[2])
    tracks = int(sys.argv[3])

    checksums = get_checksums(path, track, tracks)

    print(f"{checksums.arv1:08x} {checksums.arv2:08x}")


if __name__ == "__main__":
    main()
