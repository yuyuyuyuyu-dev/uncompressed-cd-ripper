import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { cleanup, render } from "vitest-browser-react";
import App from "@/App";
import type { Breadcrumb, Environment, ErrorReport } from "@/bindings";
import { ErrorReporter } from "./ErrorReporter";
import { buildErrorReport } from "./error-report";
import "@/index.css";

const environment: Environment = {
	release: "uncompressed-cd-ripper@0.1.0",
	osName: "Mac OS",
	osVersion: "26.6.1",
	architecture: "aarch64",
};

const BREADCRUMBS: Breadcrumb[] = [
	{
		timestamp: "2026-08-29T09:00:00.000Z",
		category: "ripping",
		message: "the rip of track 3 started",
	},
	{
		timestamp: "2026-08-29T09:00:12.500Z",
		category: "ripping",
		message: "track 3 was read again: read 2",
	},
];

function mockBackend() {
	const sent: ErrorReport[] = [];

	mockIPC((command, payload) => {
		if (command === "environment") {
			return environment;
		}
		if (command === "breadcrumbs") {
			return BREADCRUMBS;
		}
		if (command === "log_error") {
			return null;
		}
		if (command === "send_error_report") {
			sent.push((payload as { report: ErrorReport }).report);
			return null;
		}
		throw new Error(`the test did not expect ${command}`);
	});

	return sent;
}

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";
const TRACKS = [1, 2, 3];
const GAVE_UP = "the drive stopped responding";

function mockBackendFailingToRip() {
	const logged: string[] = [];

	mockIPC(
		(command, payload) => {
			if (command === "log_error") {
				logged.push((payload as { error: string }).error);

				return null;
			}
			if (command === "environment") {
				return environment;
			}
			if (command === "breadcrumbs") {
				return BREADCRUMBS;
			}
			if (command === "drives") {
				return [DRIVE];
			}
			if (command === "tracks") {
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
				return [];
			}
			if (command === "rip_track") {
				throw GAVE_UP;
			}
			throw new Error(`the test did not expect ${command}`);
		},
		{ shouldMockEvents: true },
	);

	return logged;
}

function notifications() {
	return page.getByLabelText("Notifications").getByRole("dialog");
}

function throwInTheApp(message: string) {
	window.dispatchEvent(
		new ErrorEvent("error", { error: new TypeError(message) }),
	);
}

afterEach(async () => {
	await cleanup();
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
	await expect
		.poll(() => sent)
		.toEqual([
			{
				event_id: onScreen.event_id,
				timestamp: onScreen.timestamp,
				platform: "javascript",
				release: environment.release,
				exception: {
					values: [
						{ type: "TypeError", value: "the drive stopped responding" },
					],
				},
				breadcrumbs: { values: BREADCRUMBS },
				contexts: {
					os: { name: environment.osName, version: environment.osVersion },
					device: { arch: environment.architecture },
				},
				tags: { architecture: environment.architecture },
				extra: {
					stacktrace: onScreen.extra.stacktrace,
					component_stack: "",
					comment: "",
				},
			},
		]);
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
	await notifications().first().hover();
	await page.getByRole("button", { name: "Close toast" }).first().click();

	// Assert
	await expect.poll(() => notifications().all()).toHaveLength(1);
});

test("should record an error on the TypeScript side as a log", async () => {
	// Arrange
	const logged = mockBackendFailingToRip();

	await render(
		<ErrorReporter>
			<App />
		</ErrorReporter>,
	);

	await page.getByRole("button", { name: "Choose where to save" }).click();

	// Act
	await page
		.getByRole("button", { name: "Start ripping", exact: true })
		.click();

	// Assert
	await expect.poll(() => logged).toEqual([`BackendError: ${GAVE_UP}`]);
});

test("should build the error report from nothing but event ID, timestamp, platform, release version, exception type and value, breadcrumbs, OS name and version, architecture tag, stacktrace, component stack, and user comment", () => {
	// Arrange
	const thrown = new TypeError("cannot read properties of undefined");
	thrown.stack = "TypeError: cannot read properties of undefined\n    at rip";
	const breadcrumbs = [
		{
			timestamp: "2026-08-13T08:59:58.750Z",
			category: "ripping",
			message: "the rip of track 3 started",
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
		breadcrumbs,
	});

	// Assert
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
					message: "the rip of track 3 started",
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
