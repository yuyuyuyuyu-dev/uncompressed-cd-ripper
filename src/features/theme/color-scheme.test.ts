import { expect, test } from "vitest";
import { cdp } from "vitest/browser";

import "@/index.css";

async function emulateOperatingSystemColorScheme(value: "light" | "dark") {
	await cdp().send("Emulation.setEmulatedMedia", {
		features: [{ name: "prefers-color-scheme", value }],
	});
}

// The palette is written in oklch, whose first component is the lightness.
// Comparing lightnesses states "this theme is the dark one" without pinning
// the test to the exact colours.
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
	await emulateOperatingSystemColorScheme("light");

	// Assert
	const { background, text } = pageLightness();
	expect(background).toBeGreaterThan(text);
});

test("should use the dark theme when the operating system prefers a dark color scheme", async () => {
	// Arrange
	await emulateOperatingSystemColorScheme("dark");

	// Assert
	const { background, text } = pageLightness();
	expect(background).toBeLessThan(text);
});

test("should follow the operating system switching to a dark color scheme while running", async () => {
	// Arrange
	await emulateOperatingSystemColorScheme("light");
	const before = pageLightness().background;

	// Act
	await emulateOperatingSystemColorScheme("dark");

	// Assert
	expect(pageLightness().background).toBeLessThan(before);
});
