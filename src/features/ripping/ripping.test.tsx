import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import { Ripper } from "./Ripper";
// The dialog has no layout to click through without the app's stylesheet.
import "@/index.css";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";
const DRIVE_NAME = "MARINA BLUE  CD-RW MB-1";
// What the store plugin hands back for an opened settings file, which nothing
// here looks at beyond passing it back.
const SETTINGS = 1;

// The drive, the disc and the filesystem are all across the IPC.
function mockBackend({ alreadyThere }: { alreadyThere: string[] }) {
	const ripped: number[] = [];

	mockIPC((command, payload) => {
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
			return [
				{ number: 1, sectors: 7500 },
				{ number: 2, sectors: 7500 },
			];
		}
		if (command === "plugin:dialog|open") {
			return FOLDER;
		}
		if (command === "already_there") {
			return alreadyThere;
		}
		if (command === "rip_track") {
			const { track } = payload as { track: number };

			ripped.push(track);

			return {
				file: `${FOLDER}/0${track}.flac`,
				checksums: { v1: 0, v2: 0 },
			};
		}
		// What the window did, which is filed rather than answered: the trail
		// itself is stated by the logging cases.
		if (command === "record") {
			return null;
		}
		throw new Error(`the test did not expect ${command}`);
	});

	return ripped;
}

async function chooseAFolder() {
	await page.getByRole("button", { name: "Choose a folder" }).click();
	await expect.element(page.getByText(FOLDER)).toBeVisible();
}

afterEach(() => {
	clearMocks();
});

test("should ask whether to overwrite when the destination already holds a file for a track", async () => {
	// Arrange
	mockBackend({ alreadyThere: ["01.flac"] });
	await render(<Ripper />);
	await chooseAFolder();

	// Act
	// Exact, because the button offering to check the rip is named after
	// AccurateRip and would otherwise answer to this too.
	await page.getByRole("button", { name: "Rip", exact: true }).click();

	// Assert
	await expect
		.element(
			page.getByRole("dialog", { name: "Overwrite what is already there?" }),
		)
		.toBeVisible();
});

test("should not start ripping when the overwrite dialog is cancelled", async () => {
	// Arrange
	const ripped = mockBackend({ alreadyThere: ["01.flac"] });
	await render(<Ripper />);
	await chooseAFolder();
	// Exact, because the button offering to check the rip is named after
	// AccurateRip and would otherwise answer to this too.
	await page.getByRole("button", { name: "Rip", exact: true }).click();

	// Act
	await page.getByRole("button", { name: "Cancel" }).click();

	// Assert
	expect(ripped).toEqual([]);
});
