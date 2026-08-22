import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import App from "@/App";
import type { Album } from "@/bindings";

const DRIVE = "/dev/disk4";

// Two pressings of one record, which is what a disc matching more than once
// usually means. They agree on everything a sleeve shows, so the year and the
// country are all there is to tell them apart.
const BRITISH: Album = {
	id: "8f468b26-4d5f-4c2d-9e5d-3f1c2b7a9e01",
	title: "Sea Change",
	artist: "Marina Blue",
	released: "1998-03-02",
	country: "GB",
	tracks: [
		{ number: 1, title: "Harbour Lights" },
		{ number: 2, title: "Low Tide" },
	],
};

const JAPANESE: Album = {
	...BRITISH,
	id: "1c9d7e52-8b3a-4f6e-9d02-5a7b1c3e8f90",
	released: "1998-04-22",
	country: "JP",
	tracks: [
		{ number: 1, title: "Harbour Lights" },
		{ number: 2, title: "Low Tide (Alternate Take)" },
	],
};

// The drive, the disc and the server are all across the IPC. What comes back
// is collected so that a test can say nothing was asked.
function mockBackend({ matches }: { matches: Album[] }) {
	const askedAbout: string[] = [];

	mockIPC((command, payload) => {
		if (command === "drives") {
			return [DRIVE];
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
		throw new Error(`the test did not expect ${command}`);
	});

	return askedAbout;
}

afterEach(() => {
	clearMocks();
});

test("should show the metadata fetched for the disc", async () => {
	// Arrange
	mockBackend({ matches: [BRITISH] });
	await render(<App />);

	// Act
	await page.getByRole("button", { name: "Look it up" }).click();

	// Assert
	await expect.element(page.getByText("Sea Change")).toBeVisible();
	await expect.element(page.getByText("Marina Blue")).toBeVisible();
	await expect.element(page.getByText("Harbour Lights")).toBeVisible();
	await expect.element(page.getByText("Low Tide")).toBeVisible();
});

test("should show every set of metadata found for the disc and let one of them be chosen", async () => {
	// Arrange
	mockBackend({ matches: [BRITISH, JAPANESE] });
	await render(<App />);

	// Act
	await page.getByRole("button", { name: "Look it up" }).click();

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
		.element(page.getByText("Low Tide (Alternate Take)"))
		.toBeVisible();
});

test("should ask before sending anything about the disc to a server", async () => {
	// Arrange
	const askedAbout = mockBackend({ matches: [BRITISH] });

	// Act
	await render(<App />);

	// Assert
	await expect
		.element(page.getByRole("dialog", { name: "Look this disc up?" }))
		.toBeVisible();
	expect(askedAbout).toEqual([]);
});

test("should send nothing about the disc when the lookup is cancelled", async () => {
	// Arrange
	const askedAbout = mockBackend({ matches: [BRITISH] });
	await render(<App />);

	// Act
	await page.getByRole("button", { name: "Cancel" }).click();
	await expect
		.element(page.getByRole("dialog", { name: "Look this disc up?" }))
		.not.toBeInTheDocument();

	// Assert
	expect(askedAbout).toEqual([]);
});
