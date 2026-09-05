import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";
import type { Album, TrackTags } from "@/bindings";

const VERSION = "0.0.0-TEST";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";
const DRIVE_NAME = "MARINA BLUE  CD-RW MB-1";
const SETTINGS = 1;

const BRITISH: Album = {
	id: "8f468b26-4d5f-4c2d-9e5d-3f1c2b7a9e01",
	title: "Sea Change",
	artist: "Marina Blue",
	released: "1998-03-02",
	country: "GB",
	tracks: [
		{ number: 1, title: "Harbour Lights", artist: "Marina Blue" },
		{ number: 2, title: "Low Tide", artist: "Marina Blue & The Tide" },
	],
};

const JAPANESE: Album = {
	...BRITISH,
	id: "1c9d7e52-8b3a-4f6e-9d02-5a7b1c3e8f90",
	released: "1998-04-22",
	country: "JP",
	tracks: [
		{ number: 1, title: "Harbour Lights", artist: "Marina Blue" },
		{ number: 2, title: "Low Tide (Alternate Take)", artist: "The Tide" },
	],
};

function mockBackend({ matches }: { matches: Album[] }) {
	const askedAbout: string[] = [];
	const ripped: (TrackTags | null)[] = [];

	mockIPC(
		(command, payload) => {
			if (command === "plugin:app|version") {
				return VERSION;
			}
			if (command === "plugin:updater|check") {
				return null;
			}
			if (command === "drives") {
				return [DRIVE];
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
			if (command === "tracks") {
				return [
					{ number: 1, sectors: 7500 },
					{ number: 2, sectors: 9000 },
				];
			}
			if (command === "look_up_disc") {
				askedAbout.push((payload as { drive: string }).drive);
				return matches;
			}
			if (command === "look_up_artwork") {
				return null;
			}
			if (command === "plugin:dialog|open") {
				return FOLDER;
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
			throw new Error(`the test did not expect ${command}`);
		},
		{ shouldMockEvents: true },
	);

	return { askedAbout, ripped };
}

async function askToLookUp() {
	await page.getByRole("button", { name: "Fetch CD details" }).click();
}

afterEach(async () => {
	await cleanup();
	clearMocks();
});

test("should show the metadata fetched for the disc", async () => {
	// Arrange
	mockBackend({ matches: [BRITISH] });
	await render(<App />);

	// Act
	await askToLookUp();
	await page.getByRole("button", { name: "Fetch", exact: true }).click();

	// Assert
	await expect
		.element(page.getByLabelText("Album", { exact: true }))
		.toHaveValue("Sea Change");
	await expect
		.element(page.getByLabelText("Album artist"))
		.toHaveValue("Marina Blue");
	await expect
		.element(page.getByLabelText("Title of track 1"))
		.toHaveValue("Harbour Lights");
	await expect
		.element(page.getByLabelText("Artist of track 1"))
		.toHaveValue("Marina Blue");
	await expect
		.element(page.getByLabelText("Title of track 2"))
		.toHaveValue("Low Tide");
	await expect
		.element(page.getByLabelText("Artist of track 2"))
		.toHaveValue("Marina Blue & The Tide");
});

test("should show every set of metadata found for the disc and let one of them be chosen", async () => {
	// Arrange
	mockBackend({ matches: [BRITISH, JAPANESE] });
	await render(<App />);

	// Act
	await askToLookUp();
	await page.getByRole("button", { name: "Fetch", exact: true }).click();

	// Assert
	await expect
		.element(page.getByRole("button", { name: /1998-03-02 · GB/ }))
		.toBeVisible();
	await expect
		.element(page.getByRole("button", { name: /1998-04-22 · JP/ }))
		.toBeVisible();

	// Act
	await page.getByRole("button", { name: /1998-04-22 · JP/ }).click();

	// Assert
	await expect
		.element(page.getByLabelText("Title of track 2"))
		.toHaveValue("Low Tide (Alternate Take)");
	await expect
		.element(page.getByLabelText("Artist of track 2"))
		.toHaveValue("The Tide");
});

test("should let the metadata be typed in by hand", async () => {
	// Arrange
	const { ripped } = mockBackend({ matches: [] });
	await render(<App />);

	// Act
	await page.getByLabelText("Album", { exact: true }).fill("Sea Change");
	await page.getByLabelText("Album artist").fill("Marina Blue");
	await page.getByLabelText("Title of track 1").fill("Harbour Lights");
	await page.getByLabelText("Artist of track 1").fill("Marina Blue");
	await page.getByLabelText("Title of track 2").fill("Low Tide");
	await page.getByLabelText("Artist of track 2").fill("The Tide");
	await page.getByRole("button", { name: "Choose where to save" }).click();
	await expect.element(page.getByText(FOLDER)).toBeVisible();
	await page
		.getByRole("button", { name: "Start ripping", exact: true })
		.click();

	// Assert
	await expect
		.poll(() => ripped)
		.toEqual([
			{
				album: "Sea Change",
				albumArtist: "Marina Blue",
				artist: "Marina Blue",
				title: "Harbour Lights",
				artwork: null,
			},
			{
				album: "Sea Change",
				albumArtist: "Marina Blue",
				artist: "The Tide",
				title: "Low Tide",
				artwork: null,
			},
		]);
});

test("should ask before sending anything about the disc to a server", async () => {
	// Arrange
	const { askedAbout } = mockBackend({ matches: [BRITISH] });
	await render(<App />);

	// Act
	await askToLookUp();

	// Assert
	await expect
		.element(page.getByRole("dialog", { name: "Fetch this CD's details?" }))
		.toBeVisible();
	expect(askedAbout).toEqual([]);
});

test("should send nothing about the disc when the lookup is cancelled", async () => {
	// Arrange
	const { askedAbout } = mockBackend({ matches: [BRITISH] });
	await render(<App />);
	await askToLookUp();

	// Act
	await page.getByRole("button", { name: "Cancel" }).click();
	await expect
		.element(page.getByRole("dialog", { name: "Fetch this CD's details?" }))
		.not.toBeInTheDocument();

	// Assert
	expect(askedAbout).toEqual([]);
});
