import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { cdp } from "vitest/browser";
import { render } from "vitest-browser-react";
import App from "@/App";

// What the store plugin hands back for an opened settings file, which nothing
// here looks at beyond passing it back.
const SETTINGS = 1;

// Drawing the app asks which drives hold a disc. These cases are about the
// stylesheet, so the answer is none.
function mockBackend() {
	mockIPC((command) => {
		if (command === "drives") {
			return [];
		}
		// It also reads what it was left set to. Nothing has ever set it, so the
		// settings file is empty.
		if (command === "plugin:store|load") {
			return SETTINGS;
		}
		if (command === "plugin:store|get") {
			return [null, false];
		}
		throw new Error(`the test did not expect ${command}`);
	});
}

afterEach(() => {
	clearMocks();
});

async function emulateOperatingSystemColorScheme(value: "light" | "dark") {
	await cdp().send("Emulation.setEmulatedMedia", {
		features: [{ name: "prefers-color-scheme", value }],
	});
}

// The palette is written in oklch, whose first component is the lightness.
// Comparing lightnesses states "this theme is the dark one" without pinning
// the test cases to the exact colours.
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

// Rendering App is what puts the app in the running state these test cases
// describe. The colours below come from the stylesheet App pulls in rather
// than from anything it renders, which is the point: dropping that import
// leaves the page unthemed and every case here failing.
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
