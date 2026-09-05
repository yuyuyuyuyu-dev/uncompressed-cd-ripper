import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";

const VERSION = "0.0.0-TEST";

const SETTINGS = 1;

function entryFor(library: RegExp) {
	return page
		.getByRole("listitem")
		.filter({ has: page.getByRole("heading", { name: library }) });
}

function mockBackend() {
	mockIPC(
		(command) => {
			if (command === "plugin:app|version") {
				return VERSION;
			}
			if (command === "plugin:updater|check") {
				return null;
			}
			if (command === "drives") {
				return [];
			}
			if (command === "plugin:store|load") {
				return SETTINGS;
			}
			if (command === "plugin:store|get") {
				return [null, false];
			}
			throw new Error(`the test did not expect ${command}`);
		},
		{ shouldMockEvents: true },
	);
}

afterEach(async () => {
	await cleanup();
	clearMocks();
});

test("should show dependency licenses and go back", async () => {
	// Arrange
	mockBackend();
	await render(<App />);

	// Act
	await page.getByRole("button", { name: "Licenses" }).click();

	// Assert
	await expect.element(entryFor(/^react \d/)).toHaveTextContent("MIT License");
	await expect
		.element(entryFor(/^libcdio-sys \d/))
		.toHaveTextContent("GNU GENERAL PUBLIC LICENSE");

	// Act
	await page.getByRole("button", { name: "Back" }).click();

	// Assert
	await expect
		.element(page.getByRole("heading", { name: "Disc" }))
		.toBeVisible();
});
