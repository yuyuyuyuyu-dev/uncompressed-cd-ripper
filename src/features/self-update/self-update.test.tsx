import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";

const VERSION = "0.0.1";

const SETTINGS = 1;

function mockBackend({ available }: { available: unknown }) {
	mockIPC(
		(command) => {
			if (command === "plugin:app|version") {
				return VERSION;
			}
			if (command === "plugin:updater|check") {
				return available;
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

test("should show update dialog when a new version is available", async () => {
	// Arrange
	mockBackend({
		available: {
			rid: 1,
			currentVersion: "0.0.1",
			version: "0.0.2",
			date: "2026-09-05 00:00:00.000 +00:00:00",
			body: "",
			rawJson: {},
		},
	});

	// Act
	await render(<App />);

	// Assert
	await expect
		.element(page.getByRole("heading", { name: "A new version is available." }))
		.toBeVisible();
	await expect
		.element(page.getByText("Restart the app to update?"))
		.toBeVisible();
	await expect
		.element(page.getByRole("button", { name: "Update", exact: true }))
		.toBeVisible();
});
