import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";

const VERSION = "0.0.0-TEST";

const SETTINGS = 1;

const AVAILABLE = {
	rid: 1,
	currentVersion: "0.0.1",
	version: "0.0.2",
	date: "2026-09-05 00:00:00.000 +00:00:00",
	body: "",
	rawJson: {},
};

const UPDATE_COMMANDS = [
	"plugin:updater|download_and_install",
	"plugin:process|restart",
];

function mockBackend({ available }: { available: unknown }) {
	const called: string[] = [];

	mockIPC(
		(command) => {
			called.push(command);

			if (command === "plugin:app|version") {
				return VERSION;
			}
			if (command === "plugin:updater|check") {
				return available;
			}
			if (command === "plugin:updater|download_and_install") {
				return null;
			}
			if (command === "plugin:process|restart") {
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

	return called;
}

afterEach(async () => {
	await cleanup();
	clearMocks();
});

test("should show update dialog when a new version is available", async () => {
	// Arrange
	mockBackend({ available: AVAILABLE });

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

test("should call update commands when the update button clicked", async () => {
	// Arrange
	const called = mockBackend({ available: AVAILABLE });

	await render(<App />);

	// Act
	await page.getByRole("button", { name: "Update", exact: true }).click();

	// Assert
	await expect
		.poll(() => called.filter((command) => UPDATE_COMMANDS.includes(command)))
		.toEqual(UPDATE_COMMANDS);
});
