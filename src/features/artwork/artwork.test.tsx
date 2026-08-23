import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import App from "@/App";
import type { Album, Cover } from "@/bindings";

const DRIVE = "/dev/disk4";

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
const SLEEVE: Cover = {
	mediaType: "image/png",
	data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC",
};

// The drive, the disc and the archive are all across the IPC. Every command
// that crosses it is collected, and every release the archive was asked about
// with it, so that the test can say the sleeve was asked for and asked for
// under the release the disc turned out to be.
function mockBackend() {
	const called: string[] = [];
	const askedFor: string[] = [];

	mockIPC((command, payload) => {
		called.push(command);

		if (command === "drives") {
			return [DRIVE];
		}
		if (command === "tracks") {
			return [{ number: 1, sectors: 7500 }];
		}
		if (command === "look_up_disc") {
			return [ALBUM];
		}
		if (command === "look_up_artwork") {
			askedFor.push((payload as { release: string }).release);

			return SLEEVE;
		}
		throw new Error(`the test did not expect ${command}`);
	});

	return { called, askedFor };
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
		.element(page.getByRole("img", { name: "Cover art" }))
		.toHaveAttribute("src", `data:${SLEEVE.mediaType};base64,${SLEEVE.data}`);
});
