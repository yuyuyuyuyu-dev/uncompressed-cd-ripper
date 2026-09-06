# Uncompressed CD Ripper

[![CI](https://github.com/yuyuyuyuyu-dev/uncompressed-cd-ripper/actions/workflows/ci.yml/badge.svg)](https://github.com/yuyuyuyuyu-dev/uncompressed-cd-ripper/actions/workflows/ci.yml)

This app rips CDs without compression. It also supports automatic metadata lookup, secure ripping (reading each track multiple times and comparing the results to verify the ripped data), and AccurateRip.

Currently the only supported format is `FLAC Uncompressed`.

<p align="center">
	<img width="800" alt="This app's screenshot" src="https://github.com/user-attachments/assets/440ea6ff-fc2c-4894-9fed-997438989f2b" />
</p>

## Installation

Download the file for your computer from the [latest release](https://github.com/yuyuyuyuyu-dev/uncompressed-cd-ripper/releases/latest).

| OS | File |
| --- | --- |
| macOS on Apple silicon | `.dmg` |
| Windows on x64 | `x64-setup.exe` |
| Linux on x86_64 | `.AppImage` |

The same release also holds `latest.json`, an `.app.tar.gz` and a few `.sig` files. The app uses those to update itself, so you can ignore them.

### macOS

The app is signed, but not by a paid Apple developer account, so macOS refuses to open it the first time. Open **System Settings**, go to **Privacy & Security**, find the message about the app, and choose **Open Anyway**. macOS asks this once.

### Windows and Linux

These two versions have never been tried. They exist because the build tools can make them, but I don't have Windows or Linux machine so I can't try them.

Open the file the way you open any other installer. Neither build is signed with a paid certificate, so your computer may warn you first.

### Updates

The app looks for a newer release every time it starts and asks whether to install it, so you download an installer only once.

## License

[GNU General Public License v3.0 or later](LICENSE)

```text
Copyright (C) 2026  yu

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
