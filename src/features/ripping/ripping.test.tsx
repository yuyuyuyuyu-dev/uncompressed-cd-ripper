import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";
import { events } from "@/bindings";
import { Ripper } from "./Ripper";
import "@/index.css";

const VERSION = "0.0.0-TEST";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";
const DRIVE_NAME = "MARINA BLUE  CD-RW MB-1";
const SETTINGS = 1;

function mockBackend({
	drives = [DRIVE],
	alreadyThere = [],
	until,
}: {
	drives?: string[];
	alreadyThere?: string[];
	until?: Promise<void>;
}) {
	const ripped: number[] = [];
	const ejected: string[] = [];

	mockIPC(
		(command, payload) => {
			if (command === "plugin:app|version") {
				return VERSION;
			}
			if (command === "plugin:updater|check") {
				return null;
			}
			if (command === "drives") {
				return drives;
			}
			if (command === "drive_name") {
				return DRIVE_NAME;
			}
			if (command === "plugin:store|load") {
				return SETTINGS;
			}
			if (command === "plugin:store|get") {
				return [null, false];
			}
			if (command === "eject_disc") {
				const { drive } = payload as { drive: string };

				ejected.push(drive);

				return null;
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

				const written = {
					file: `${FOLDER}/0${track}.flac`,
					checksums: { v1: 0, v2: 0 },
				};

				return until === undefined ? written : until.then(() => written);
			}
			if (command === "plugin:notification|is_permission_granted") {
				return true;
			}
			if (command === "plugin:notification|notify") {
				return null;
			}
			throw new Error(`the test did not expect ${command}`);
		},
		{ shouldMockEvents: true },
	);

	return { ripped, ejected };
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
	const { ripped } = mockBackend({ alreadyThere: ["01.flac"] });
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

test("should call the eject command when the eject button is clicked", async () => {
	// Arrange
	const { ejected } = mockBackend({});
	await render(<Ripper />);
	await expect.element(page.getByRole("button", { name: DRIVE })).toBeVisible();

	// Act
	await page.getByRole("button", { name: "Eject the disc" }).click();

	// Assert
	await expect.poll(() => ejected).toEqual([DRIVE]);
});

test("should keep the state after another screen is shown", async () => {
	// Arrange
	let letTheRipFinish!: () => void;
	const until = new Promise<void>((resolve) => {
		letTheRipFinish = resolve;
	});
	const { ripped } = mockBackend({ until });
	await render(<App />);
	await page.getByLabelText("Album", { exact: true }).fill("Sea Change");
	await chooseAFolder();
	await page
		.getByRole("button", { name: "Start ripping", exact: true })
		.click();
	await expect.element(page.getByText(/Ripping track 01/)).toBeVisible();

	// Act
	await page.getByRole("button", { name: "Licenses" }).click();
	await page.getByRole("button", { name: "Back" }).click();

	// Assert
	await expect
		.element(page.getByLabelText("Album", { exact: true }))
		.toHaveValue("Sea Change");
	await expect.element(page.getByText(FOLDER)).toBeVisible();
	await expect.element(page.getByText(/Ripping track 01/)).toBeVisible();
	letTheRipFinish();
	await expect.poll(() => ripped).toEqual([1, 2]);
});
