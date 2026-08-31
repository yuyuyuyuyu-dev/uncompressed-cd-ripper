import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import { events } from "@/bindings";
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
function mockBackend({
	drives = [DRIVE],
	alreadyThere = [],
}: {
	drives?: string[];
	alreadyThere?: string[];
}) {
	const ripped: number[] = [];

	mockIPC(
		(command, payload) => {
			if (command === "drives") {
				return drives;
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
			throw new Error(`the test did not expect ${command}`);
		},
		{ shouldMockEvents: true },
	);

	return ripped;
}

async function chooseAFolder() {
	await page.getByRole("button", { name: "Choose where to save" }).click();
	await expect.element(page.getByText(FOLDER)).toBeVisible();
}

afterEach(async () => {
	await cleanup();
	clearMocks();
});

test("should ask whether to overwrite when the destination already holds a file for a track", async () => {
	// Arrange
	mockBackend({ alreadyThere: ["01.flac"] });
	await render(<Ripper />);
	await chooseAFolder();

	// Act
	await page
		.getByRole("button", { name: "Start ripping", exact: true })
		.click();

	// Assert
	await expect
		.element(page.getByRole("dialog", { name: "Overwrite?" }))
		.toBeVisible();
});

test("should not start ripping when the overwrite dialog is cancelled", async () => {
	// Arrange
	const ripped = mockBackend({ alreadyThere: ["01.flac"] });
	await render(<Ripper />);
	await chooseAFolder();
	await page
		.getByRole("button", { name: "Start ripping", exact: true })
		.click();

	// Act
	await page.getByRole("button", { name: "Cancel" }).click();

	// Assert
	expect(ripped).toEqual([]);
});

test("should show a disc on screen automatically when it is put into the drive", async () => {
	// Arrange
	mockBackend({ drives: [] });
	await render(<Ripper />);
	await expect
		.element(page.getByText("No drive with an audio CD in it."))
		.toBeVisible();

	// Act
	await events.drivesChanged.emit([DRIVE]);

	// Assert
	await expect.element(page.getByRole("button", { name: DRIVE })).toBeVisible();
});

test("should remove a disc from the screen automatically when it is taken out of the drive", async () => {
	// Arrange
	mockBackend({ drives: [DRIVE] });
	await render(<Ripper />);
	await expect.element(page.getByRole("button", { name: DRIVE })).toBeVisible();

	// Act
	await events.drivesChanged.emit([]);

	// Assert
	await expect
		.element(page.getByText("No drive with an audio CD in it."))
		.toBeVisible();
});
