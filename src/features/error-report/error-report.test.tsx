import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import App from "@/App";
import type { Breadcrumb, Environment, ErrorReport } from "@/bindings";
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

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";
const TRACKS = [1, 2, 3];

// The backend is a separate process reached over IPC, which is the sort of
// thing the conventions allow standing in for.
function mockBackend() {
	const sent: ErrorReport[] = [];

	mockIPC((command, payload) => {
		if (command === "environment") {
			return environment;
		}
		if (command === "trail") {
			return [];
		}
		if (command === "send_error_report") {
			sent.push((payload as { report: ErrorReport }).report);
			return null;
		}
		throw new Error(`the test did not expect ${command}`);
	});

	return sent;
}

// A whole app in front of the reporter, and behind it a backend that keeps a
// trail as the real one does: an entry as each piece of the work arrives,
// worded on that side. Rather than a list this case hands over, so that what
// ends up in the report is what using the app came to.
function mockBackendKeepingATrail() {
	const trail: Breadcrumb[] = [];

	const file = (category: string, message: string) => {
		trail.push({
			timestamp: new Date(
				Date.UTC(2026, 7, 29, 9, 0, trail.length),
			).toISOString(),
			category,
			message,
		});
	};

	mockIPC((command, payload) => {
		if (command === "environment") {
			return environment;
		}
		if (command === "trail") {
			return [...trail];
		}
		if (command === "drives") {
			return [DRIVE];
		}
		if (command === "tracks") {
			file("ripping", `the disc holds ${TRACKS.length} audio tracks`);

			return TRACKS.map((number) => ({ number, sectors: 7500 }));
		}
		if (command === "drive_name") {
			return "MARINA BLUE  CD-RW MB-1";
		}
		if (command === "plugin:store|load") {
			return 1;
		}
		if (command === "plugin:store|get") {
			return [null, false];
		}
		if (command === "plugin:store|set" || command === "plugin:store|save") {
			return null;
		}
		if (command === "plugin:dialog|open") {
			return FOLDER;
		}
		if (command === "already_there") {
			file("ripping", `a rip of ${TRACKS.length} tracks was asked for`);

			return [];
		}
		if (command === "rip_track") {
			file(
				"ripping",
				`track ${(payload as { track: number }).track} was written`,
			);

			return { file: "", checksums: { v1: 0, v2: 0 } };
		}
		if (command === "plugin:notification|is_permission_granted") {
			return true;
		}
		if (command === "plugin:notification|notify") {
			return null;
		}
		throw new Error(`the test did not expect ${command}`);
	});

	return trail;
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

test("should show a success message once the error report has been sent", async () => {
	// Arrange
	mockBackend();
	await render(<ErrorReporter />);
	throwInTheApp("the drive stopped responding");
	await expect.element(notifications().first()).toBeVisible();
	await page.getByRole("button", { name: "Details" }).click();
	await expect.element(page.getByLabelText("The error report")).toBeVisible();

	// Act
	await page.getByRole("button", { name: "Send" }).click();

	// Assert
	await expect.element(page.getByText("Report sent")).toBeVisible();
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

test("should show what the app was doing before the error in the report", async () => {
	// Arrange
	// The app is used the way anybody uses it — a folder, a rip — and only
	// then does something fail.
	const trail = mockBackendKeepingATrail();

	await render(
		<ErrorReporter>
			<App />
		</ErrorReporter>,
	);

	await page.getByRole("button", { name: "Choose a folder" }).click();
	await page.getByRole("button", { name: "Rip", exact: true }).click();
	// The button comes back to life once the rip has run all the way out.
	await expect
		.element(page.getByRole("button", { name: "Rip", exact: true }))
		.toBeEnabled();

	const before = [...trail];

	expect(before.map((crumb) => crumb.message)).toEqual([
		"the disc holds 3 audio tracks",
		"a rip of 3 tracks was asked for",
		"track 1 was written",
		"track 2 was written",
		"track 3 was written",
	]);

	// Act
	throwInTheApp("the drive stopped responding");
	await page.getByRole("button", { name: "Details" }).click();

	// Assert
	// Read off the screen the user is asked to agree to rather than out of the
	// call that sends it: the trail travels only if it is shown first.
	const shown = page.getByLabelText("The error report");

	await expect.element(shown).toBeVisible();
	await expect
		.poll(() => JSON.parse(shown.element().textContent ?? "{}").breadcrumbs)
		.toEqual({ values: before });
});

test("should build the error report from nothing but event ID, timestamp, platform, release version, exception type and value, breadcrumbs, OS name and version, architecture tag, stacktrace, component stack, and user comment", () => {
	// Arrange
	const thrown = new TypeError("cannot read properties of undefined");
	thrown.stack = "TypeError: cannot read properties of undefined\n    at rip";
	const trail = [
		{
			timestamp: "2026-08-13T08:59:58.750Z",
			category: "ripping",
			message: "track 3 was started",
		},
	];

	// Act
	const report = buildErrorReport({
		eventId: "fc6d8c0c43fc4630ad850ee518f1b9d0",
		thrown,
		componentStack: "\n    at TrackListing\n    at App",
		environment,
		occurredAt: new Date("2026-08-13T09:00:00.000Z"),
		comment: "it stopped on the third track",
		trail,
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
		breadcrumbs: {
			values: [
				{
					timestamp: "2026-08-13T08:59:58.750Z",
					category: "ripping",
					message: "track 3 was started",
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
			component_stack: "\n    at TrackListing\n    at App",
			comment: "it stopped on the third track",
		},
	});
});
