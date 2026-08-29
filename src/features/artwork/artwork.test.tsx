import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import App from "@/App";
import type { Album, Artwork, TrackTags } from "@/bindings";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";
const DRIVE_NAME = "MARINA BLUE  CD-RW MB-1";
// What the store plugin hands back for an opened settings file, which nothing
// here looks at beyond passing it back.
const SETTINGS = 1;
const CHOSEN = "/Users/someone/Pictures/artwork.png";

const ALBUM: Album = {
	id: "8f468b26-4d5f-4c2d-9e5d-3f1c2b7a9e01",
	title: "Sea Change",
	artist: "Marina Blue",
	released: "1998-03-02",
	country: "GB",
	tracks: [{ number: 1, title: "Harbour Lights", artist: "Marina Blue" }],
};

// A picture rather than something standing in for one: one red pixel, which is
// about the smallest a PNG comes.
const ARTWORK: Artwork = {
	mediaType: "image/png",
	data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC",
};

// The drive, the disc, the archive, the picker and the folder are all across
// the IPC. Every command that crosses it is collected, and with them every
// release the archive was asked about, every file the backend was asked to
// read, and the tags each track was ripped with, so that the test can say what
// was asked for and what ended up in the files.
function mockBackend() {
	const called: string[] = [];
	const askedFor: string[] = [];
	const read: string[] = [];
	const ripped: (TrackTags | null)[] = [];

	mockIPC((command, payload) => {
		called.push(command);

		if (command === "drives") {
			return [DRIVE];
		}
		if (command === "drive_name") {
			return DRIVE_NAME;
		}
		// The settings file with nothing in it: no read offset has ever been
		// kept for this drive, which is a machine the app has not been set up on.
		if (command === "plugin:store|load") {
			return SETTINGS;
		}
		if (command === "plugin:store|get") {
			return [null, false];
		}
		if (command === "tracks") {
			return [{ number: 1, sectors: 7500 }];
		}
		if (command === "look_up_disc") {
			return [ALBUM];
		}
		if (command === "look_up_artwork") {
			askedFor.push((payload as { release: string }).release);

			return ARTWORK;
		}
		if (command === "plugin:dialog|open") {
			// One picker serves both buttons, and only what it was opened with
			// tells them apart.
			const { options } = payload as { options: { directory?: boolean } };

			return options.directory === true ? FOLDER : CHOSEN;
		}
		if (command === "read_artwork") {
			read.push((payload as { path: string }).path);

			return ARTWORK;
		}
		if (command === "already_there") {
			return [];
		}
		if (command === "rip_track") {
			ripped.push((payload as { tags: TrackTags | null }).tags);

			return { file: "", checksums: { v1: 0, v2: 0 } };
		}
		if (command === "plugin:notification|is_permission_granted") {
			return true;
		}
		if (command === "plugin:notification|notify") {
			return null;
		}
		// What the window did, which is filed rather than answered: the trail
		// itself is stated by the logging cases.
		if (command === "record") {
			return null;
		}
		throw new Error(`the test did not expect ${command}`);
	});

	return { called, askedFor, read, ripped };
}

afterEach(() => {
	clearMocks();
});

test("should fetch the album artwork from the internet", async () => {
	// Arrange
	const { called, askedFor } = mockBackend();
	await render(<App />);

	// Act
	await page.getByRole("button", { name: "Look this disc up" }).click();
	await page.getByRole("button", { name: "Look it up" }).click();

	// Assert
	await expect.poll(() => called).toContain("look_up_artwork");
	await expect.poll(() => askedFor).toEqual([ALBUM.id]);
	await expect
		.element(page.getByRole("img", { name: "Album artwork" }))
		.toHaveAttribute("src", `data:${ARTWORK.mediaType};base64,${ARTWORK.data}`);
});

test("should let the album artwork be chosen from this computer", async () => {
	// Arrange
	const { read, ripped } = mockBackend();
	await render(<App />);

	// Act
	await page.getByRole("button", { name: "Choose artwork" }).click();
	await expect
		.element(page.getByRole("img", { name: "Album artwork" }))
		.toBeVisible();
	await page.getByRole("button", { name: "Choose a folder" }).click();
	await expect.element(page.getByText(FOLDER)).toBeVisible();
	// Exact, because the button offering to check the rip is named after
	// AccurateRip and would otherwise answer to this too.
	await page.getByRole("button", { name: "Rip", exact: true }).click();

	// Assert
	await expect.poll(() => read).toEqual([CHOSEN]);
	await expect
		.element(page.getByRole("img", { name: "Album artwork" }))
		.toHaveAttribute("src", `data:${ARTWORK.mediaType};base64,${ARTWORK.data}`);
	await expect
		.poll(() => ripped)
		.toEqual([
			{
				album: null,
				albumArtist: null,
				artist: null,
				title: null,
				artwork: ARTWORK,
			},
		]);
});
