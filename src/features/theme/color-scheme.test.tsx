import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { cdp } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";

const VERSION = "0.0.0-TEST";

const SETTINGS = 1;

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

async function emulateOperatingSystemColorScheme(value: "light" | "dark") {
	await cdp().send("Emulation.setEmulatedMedia", {
		features: [{ name: "prefers-color-scheme", value }],
	});
}

function lightnessOf(color: string) {
	const lightness = /^oklch\(([\d.]+)/.exec(color)?.[1];
	if (lightness === undefined) {
		throw new Error(`expected an oklch colour but got ${color}`);
	}
	return Number(lightness);
}

function pageLightness() {
	const style = getComputedStyle(document.body);
	return {
		background: lightnessOf(style.backgroundColor),
		text: lightnessOf(style.color),
	};
}

test("should use the light theme when the operating system prefers a light color scheme", async () => {
	// Arrange
	mockBackend();
	await render(<App />);

	// Act
	await emulateOperatingSystemColorScheme("light");

	// Assert
	const { background, text } = pageLightness();
	expect(background).toBeGreaterThan(text);
});

test("should use the dark theme when the operating system prefers a dark color scheme", async () => {
	// Arrange
	mockBackend();
	await render(<App />);

	// Act
	await emulateOperatingSystemColorScheme("dark");

	// Assert
	const { background, text } = pageLightness();
	expect(background).toBeLessThan(text);
});

test("should follow the operating system switching to a dark color scheme while running", async () => {
	// Arrange
	mockBackend();
	await render(<App />);
	await emulateOperatingSystemColorScheme("light");
	const before = pageLightness().background;

	// Act
	await emulateOperatingSystemColorScheme("dark");

	// Assert
	expect(pageLightness().background).toBeLessThan(before);
});
