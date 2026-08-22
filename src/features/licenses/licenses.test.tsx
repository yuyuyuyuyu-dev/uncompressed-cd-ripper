import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import App from "@/App";
import { Licenses } from "./Licenses";

function entryFor(library: RegExp) {
	return page
		.getByRole("listitem")
		.filter({ has: page.getByRole("heading", { name: library }) });
}

// Drawing the whole app asks which drives hold a disc. The case below is about
// moving between screens, so the answer is none.
function mockBackend() {
	mockIPC((command) => {
		if (command === "drives") {
			return [];
		}
		throw new Error(`the test did not expect ${command}`);
	});
}

afterEach(() => {
	clearMocks();
});

// One library from each side of the app: React draws the window, libcdio
// reads the disc.
test("should show dependency licenses", async () => {
	// Arrange & Act
	await render(<Licenses />);

	// Assert
	await expect.element(entryFor(/^react \d/)).toHaveTextContent("MIT License");
	await expect
		.element(entryFor(/^libcdio-sys \d/))
		.toHaveTextContent("GNU GENERAL PUBLIC LICENSE");
});

test("should go back from the licenses screen", async () => {
	// Arrange
	mockBackend();
	await render(<App />);
	await page.getByRole("button", { name: "Licenses" }).click();

	// Act
	await page.getByRole("button", { name: "Back" }).click();

	// Assert
	await expect
		.element(page.getByRole("heading", { name: "Disc" }))
		.toBeVisible();
});
