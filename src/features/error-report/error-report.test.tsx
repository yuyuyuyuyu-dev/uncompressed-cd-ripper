import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import { commands, type Environment, type ErrorReport } from "@/bindings";
import { expectOk } from "./backend";
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

// Base UI puts the toasts in a region it labels, and gives each one the dialog
// role, so this is what "a notification" looks like from the outside.
function notifications() {
	return page.getByLabelText("Notifications").getByRole("dialog");
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
	await expect.element(notifications().first()).toBeVisible();
});

test("should reach the detail screen from the notification", async () => {
	// Arrange
	mockBackend();
	await render(<ErrorReporter />);
	throwInTheApp("the drive stopped responding");
	await expect.element(notifications().first()).toBeVisible();

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
	await expect.element(notifications().first()).toBeVisible();
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

test("should keep a notification for every error rather than only the newest", async () => {
	// Arrange
	mockBackend();
	await render(<ErrorReporter />);

	// Act
	throwInTheApp("track 3 could not be read");
	throwInTheApp("track 7 could not be read");

	// Assert
	await expect.poll(() => notifications().all()).toHaveLength(2);
});

test("should keep a notification until it is dismissed", async () => {
	// Arrange
	mockBackend();
	await render(<ErrorReporter />);
	throwInTheApp("track 3 could not be read");
	throwInTheApp("track 7 could not be read");
	await expect.poll(() => notifications().all()).toHaveLength(2);

	// Act
	// The stack collapses until it is hovered, and a toast behind the front one
	// takes no clicks while it is folded away.
	await notifications().first().hover();
	await page.getByRole("button", { name: "Close toast" }).first().click();

	// Assert
	await expect.poll(() => notifications().all()).toHaveLength(1);
});

test("should keep the notifications from taking over the window", async () => {
	// Arrange
	mockBackend();
	await render(<ErrorReporter />);

	// Act
	for (let track = 1; track <= 12; track += 1) {
		throwInTheApp(`track ${track} could not be read`);
	}
	await expect.poll(() => notifications().all()).toHaveLength(12);

	// Assert
	const covered =
		page.getByLabelText("Notifications").element().getBoundingClientRect()
			.height / window.innerHeight;
	expect(covered).toBeLessThan(0.5);
});

test("should raise the failure a backend command reports", async () => {
	// Arrange
	mockIPC((command) => {
		if (command === "environment") {
			return environment;
		}
		throw "the drive is not ready";
	});
	const report = buildErrorReport({
		eventId: "fc6d8c0c43fc4630ad850ee518f1b9d0",
		thrown: new TypeError("anything"),
		environment,
		occurredAt: new Date(),
		comment: "",
	});

	// Act
	const raised = expectOk(commands.sendErrorReport(report));

	// Assert
	await expect(raised).rejects.toThrow("the drive is not ready");
});

test("should build the error report from nothing but the error message, the error type, the stack trace, the time it occurred, the application version, the operating system name and version, the architecture and what the user wrote", () => {
	// Arrange
	const thrown = new TypeError("cannot read properties of undefined");
	thrown.stack = "TypeError: cannot read properties of undefined\n    at rip";

	// Act
	const report = buildErrorReport({
		eventId: "fc6d8c0c43fc4630ad850ee518f1b9d0",
		thrown,
		environment,
		occurredAt: new Date("2026-08-13T09:00:00.000Z"),
		comment: "it stopped on the third track",
	});

	// Assert
	// Spelling the whole report out is what states the "nothing but": a field
	// nobody agreed to would have to appear here to pass.
	expect(report).toEqual({
		event_id: "fc6d8c0c43fc4630ad850ee518f1b9d0",
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
