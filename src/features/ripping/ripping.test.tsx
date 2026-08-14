import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import { Ripper } from "./Ripper";
// The dialog below is only ever on screen inside the app, which is where the
// stylesheet comes from; without it there is no layout to click through.
import "@/index.css";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";

// The drive, the disc and the filesystem are all on the other side of the IPC,
// which is the sort of thing the conventions allow standing in for.
function mockBackend({ alreadyThere }: { alreadyThere: string[] }) {
	const ripped: number[] = [];

	mockIPC((command, payload) => {
		if (command === "drives") {
			return [DRIVE];
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
			ripped.push((payload as { track: number }).track);
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
	await page.getByRole("button", { name: "Rip" }).click();

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
	await page.getByRole("button", { name: "Rip" }).click();

	// Act
	await page.getByRole("button", { name: "Cancel" }).click();

	// Assert
	expect(ripped).toEqual([]);
});
