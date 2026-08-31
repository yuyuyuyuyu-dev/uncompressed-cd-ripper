import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";
import type { Verdict } from "@/bindings";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";

// What the drive answers with when asked what it is, and what AccurateRip
// lists it as reading ahead by. A drive AccurateRip has never been told about
// could not be checked at all.
const DRIVE_NAME = "MARINA BLUE  CD-RW MB-1";
const READ_OFFSET = 6;

// What the store plugin hands back for an opened settings file, which nothing
// here looks at beyond passing it back.
const SETTINGS = 1;

// The drive, the disc, the folder and AccurateRip are all across the IPC.
// Every command that crosses it is collected, and with them every track that
// was ripped, so that a case can say what was asked for and what was not. What
// is on the disc and what AccurateRip says about it are the case's to lay out.
// The settings file starts empty, as it does on a machine the app has not been
// set up on.
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

// Turned on the way a person turns it on, which is also what looks the drive's
// read offset up: nothing is held against anybody else's rip until that is
// known.
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
	// Three tracks, each agreed with by a different number of submissions, so
	// that a confidence landing on the wrong row shows up.
	const confidences = [68, 7, 1];

	mockBackend({ tracks: [1, 2, 3], confidences });
	await render(<App />);
	await turnTheCheckOn();

	// Act
	await chooseAFolderAndRip();

	// Assert
	// Each row against the number that came back for it, so that a table which
	// showed the right numbers in the wrong order would fail here.
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
	// Nothing is laid out for AccurateRip to answer with, because nothing here
	// should get as far as asking it.
	const tracks = [1, 2, 3];
	const { called, ripped } = mockBackend({ tracks });

	await render(<App />);
	await expect.element(page.getByRole("switch")).not.toBeChecked();

	// Act
	await chooseAFolderAndRip();

	// Assert
	// Every track was read, and the button that started the rip has come back
	// to life, which it only does once the rip has run all the way out. What is
	// being stated is about a whole rip rather than about one that never got
	// as far as asking.
	await expect.poll(() => ripped).toEqual(tracks);
	await expect
		.element(page.getByRole("button", { name: "Start ripping", exact: true }))
		.toBeEnabled();

	// The disc's identifier only ever leaves in the address this command
	// fetches, so a rip that never sends it is a rip that never runs it.
	expect(called).not.toContain("check_rip");
});
