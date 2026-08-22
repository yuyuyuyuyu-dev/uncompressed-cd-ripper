import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import { Ripper } from "./Ripper";
// The dialog has no layout to click through without the app's stylesheet.
import "@/index.css";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";

// The drive, the disc and the filesystem are all across the IPC.
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

// A disc in the drive is now asked about before anything is sent away, and the
// question stands in front of the screen until it is answered. These cases are
// about ripping, which the answer does not change.
async function dismissTheLookup() {
	await page.getByRole("button", { name: "Cancel" }).click();
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
	await dismissTheLookup();
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
	await dismissTheLookup();
	await chooseAFolder();
	await page.getByRole("button", { name: "Rip" }).click();

	// Act
	await page.getByRole("button", { name: "Cancel" }).click();

	// Assert
	expect(ripped).toEqual([]);
});
