import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import { Licenses } from "./Licenses";

function entryFor(library: RegExp) {
	return page
		.getByRole("listitem")
		.filter({ has: page.getByRole("heading", { name: library }) });
}

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
