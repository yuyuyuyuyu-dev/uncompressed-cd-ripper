import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import type { Environment, ErrorReport } from "@/bindings";
import { ErrorReporter } from "./ErrorReporter";
import { buildErrorReport } from "./error-report";
// The component is only ever on screen inside the app, which is where the
// stylesheet comes from; without it the dialog has no layout to click through.
import "@/index.css";

const environment: Environment = {
	release: "uncompressed-cd-ripper@0.1.0",
	osName: "Mac OS",
	osVersion: "26.6.1",
	architecture: "aarch64",
};

// The backend is a separate process reached over IPC, which is the sort of
// thing the conventions allow standing in for.
function mockBackend() {
	const sent: ErrorReport[] = [];

	mockIPC((command, payload) => {
		if (command === "environment") {
			return environment;
		}
		if (command === "send_error_report") {
			sent.push((payload as { report: ErrorReport }).report);
			return null;
		}
		throw new Error(`the test did not expect ${command}`);
	});

	return sent;
}

function throwInTheApp(message: string) {
	window.dispatchEvent(
		new ErrorEvent("error", { error: new TypeError(message) }),
	);
}

afterEach(() => {
	clearMocks();
});

test("should notify the user when an error occurs", async () => {
	// Arrange
	mockBackend();
	await render(<ErrorReporter />);

	// Act
	throwInTheApp("the drive stopped responding");

	// Assert
	await expect.element(page.getByRole("alert")).toBeVisible();
});

test("should reach the detail screen from the notification", async () => {
	// Arrange
	mockBackend();
	await render(<ErrorReporter />);
	throwInTheApp("the drive stopped responding");
	await expect.element(page.getByRole("alert")).toBeVisible();

	// Act
	await page.getByRole("button", { name: "Details" }).click();

	// Assert
	await expect.element(page.getByLabelText("The error report")).toBeVisible();
});

test("should send the error report from the detail screen", async () => {
	// Arrange
	const sent = mockBackend();
	await render(<ErrorReporter />);
	throwInTheApp("the drive stopped responding");
	await expect.element(page.getByRole("alert")).toBeVisible();
	await page.getByRole("button", { name: "Details" }).click();
	const shown = page.getByLabelText("The error report");
	await expect.element(shown).toBeVisible();
	const onScreen = JSON.parse(shown.element().textContent ?? "");

	// Act
	await page.getByRole("button", { name: "Send" }).click();

	// Assert
	// What arrives is the report that was on screen, rather than merely a
	// report: the whole point of showing it first.
	await expect.poll(() => sent).toEqual([onScreen]);
});

test("should build the error report from nothing but the error message, the error type, the stack trace, the time it occurred, the application version, the operating system name and version, the architecture and what the user wrote", () => {
	// Arrange
	const thrown = new TypeError("cannot read properties of undefined");
	thrown.stack = "TypeError: cannot read properties of undefined\n    at rip";

	// Act
	const report = buildErrorReport({
		thrown,
		environment,
		occurredAt: new Date("2026-08-13T09:00:00.000Z"),
		comment: "it stopped on the third track",
	});

	// Assert
	// Spelling the whole report out is what states the "nothing but": a field
	// nobody agreed to would have to appear here to pass.
	expect(report).toEqual({
		event_id: expect.stringMatching(/^[0-9a-f]{32}$/),
		timestamp: "2026-08-13T09:00:00.000Z",
		platform: "javascript",
		release: "uncompressed-cd-ripper@0.1.0",
		exception: {
			values: [
				{
					type: "TypeError",
					value: "cannot read properties of undefined",
				},
			],
		},
		contexts: {
			os: { name: "Mac OS", version: "26.6.1" },
			device: { arch: "aarch64" },
		},
		tags: { architecture: "aarch64" },
		extra: {
			stacktrace: "TypeError: cannot read properties of undefined\n    at rip",
			comment: "it stopped on the third track",
		},
	});
});
