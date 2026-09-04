import sys

from arver.audio.checksums import get_checksums


def main():
    path = sys.argv[1]
    track = int(sys.argv[2])
    tracks = int(sys.argv[3])

    checksums = get_checksums(path, track, tracks)

    print(f"{checksums.arv1:08x} {checksums.arv2:08x}")


if __name__ == "__main__":
    main()
