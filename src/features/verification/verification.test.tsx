import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";
import type { Verdict } from "@/bindings";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";

const DRIVE_NAME = "MARINA BLUE  CD-RW MB-1";
const READ_OFFSET = 6;

const SETTINGS = 1;

function mockBackend({
	tracks,
	confidences = [],
}: {
	tracks: number[];
	confidences?: number[];
}) {
	const called: string[] = [];
	const ripped: number[] = [];

	mockIPC(
		(command, payload) => {
			called.push(command);

			if (command === "drives") {
				return [DRIVE];
			}
			if (command === "tracks") {
				return tracks.map((number) => ({ number, sectors: 7500 }));
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
			if (command === "plugin:store|set" || command === "plugin:store|save") {
				return null;
			}
			if (command === "read_offset") {
				return READ_OFFSET;
			}
			if (command === "plugin:dialog|open") {
				return FOLDER;
			}
			if (command === "already_there") {
				return [];
			}
			if (command === "rip_track") {
				ripped.push((payload as { track: number }).track);

				return { file: "", checksums: { v1: 0, v2: 0 } };
			}
			if (command === "check_rip") {
				return confidences.map(
					(others): Verdict => ({ outcome: "matched", others }),
				);
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

	return { called, ripped };
}

async function turnTheCheckOn() {
	await page.getByRole("switch").click();
	await page.getByRole("button", { name: "Turn it on" }).click();
	await expect.element(page.getByRole("switch")).toBeChecked();
}

async function chooseAFolderAndRip() {
	await page.getByRole("button", { name: "Choose where to save" }).click();
	await page
		.getByRole("button", { name: "Start ripping", exact: true })
		.click();
}

afterEach(async () => {
	await cleanup();
	clearMocks();
});

test("should show the AccurateRip confidence for each ripped track", async () => {
	// Arrange
	const confidences = [68, 7, 1];

	mockBackend({ tracks: [1, 2, 3], confidences });
	await render(<App />);
	await turnTheCheckOn();

	// Act
	await chooseAFolderAndRip();

	// Assert
	for (const [index, confidence] of confidences.entries()) {
		await expect
			.element(
				page
					.getByRole("row")
					.filter({ hasText: `0${index + 1}` })
					.getByText(String(confidence), { exact: true }),
			)
			.toBeVisible();
	}
});

test("should send no disc id to AccurateRip when the check is off", async () => {
	// Arrange
	const tracks = [1, 2, 3];
	const { called, ripped } = mockBackend({ tracks });

	await render(<App />);
	await expect.element(page.getByRole("switch")).not.toBeChecked();

	// Act
	await chooseAFolderAndRip();

	// Assert
	await expect.poll(() => ripped).toEqual(tracks);
	await expect
		.element(page.getByRole("button", { name: "Start ripping", exact: true }))
		.toBeEnabled();

	expect(called).not.toContain("check_rip");
});
