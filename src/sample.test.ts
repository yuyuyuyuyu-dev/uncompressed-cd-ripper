import { expect, test } from "vitest";

// Vitest exits 1 when it finds no test file at all, so this placeholder keeps
// CI meaningful until the first real test case replaces it.
test("should be replaced by a real test case", () => {
	expect("Hello").toBe("Hello");
});
