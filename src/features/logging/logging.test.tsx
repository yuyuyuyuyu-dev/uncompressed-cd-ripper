import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-react";
import App from "@/App";
import type { Breadcrumb, Environment, Happening } from "@/bindings";
import { ErrorReporter } from "../error-report/ErrorReporter";
// The report is read off the screen it is shown on, which has no layout
// without the app's stylesheet.
import "@/index.css";

const DRIVE = "/dev/disk4";
const FOLDER = "/Users/someone/Music";
const TRACKS = [1, 2, 3];

const environment: Environment = {
	release: "uncompressed-cd-ripper@0.1.0",
	osName: "Mac OS",
	osVersion: "26.6.1",
	architecture: "aarch64",
};

// The trail is kept on the other side of the IPC, which is the sort of thing
// the conventions allow standing in for. What a happening comes to in words is
// the backend's business; this files each one as it arrives and hands the lot
// back, so that a case can say what the window put there and in what order.
function mockBackend() {
	const trail: Breadcrumb[] = [];

	mockIPC((command, payload) => {
		if (command === "record") {
			const { happening } = payload as { happening: Happening };

			trail.push({
				timestamp: new Date(
					Date.UTC(2026, 7, 29, 9, 0, trail.length),
				).toISOString(),
				category: "window",
				message: JSON.stringify(happening),
			});

			return null;
		}
		if (command === "trail") {
			return [...trail];
		}
		if (command === "environment") {
			return environment;
		}
		if (command === "drives") {
			return [DRIVE];
		}
		if (command === "tracks") {
			return TRACKS.map((number) => ({ number, sectors: 7500 }));
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
		if (command === "drive_name") {
			return "MARINA BLUE  CD-RW MB-1";
		}
		if (command === "plugin:dialog|open") {
			return FOLDER;
		}
		if (command === "already_there") {
			return [];
		}
		if (command === "rip_track") {
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

afterEach(() => {
	clearMocks();
});

test("should show what the app was doing before the error in the report", async () => {
	// Arrange
	// The app is used the way anybody uses it, and only then does something
	// fail: a drive, a folder, a rip.
	const trail = mockBackend();

	await render(
		<ErrorReporter>
			<App />
		</ErrorReporter>,
	);

	await page.getByRole("button", { name: DRIVE }).click();
	await page.getByRole("button", { name: "Choose a folder" }).click();
	await page.getByRole("button", { name: "Rip", exact: true }).click();
	// The button comes back to life once the rip has run all the way out.
	await expect
		.element(page.getByRole("button", { name: "Rip", exact: true }))
		.toBeEnabled();

	const before = [...trail];

	expect(before.map((crumb) => JSON.parse(crumb.message))).toEqual([
		{ happening: "driveChosen" },
		{ happening: "folderChosen" },
		{ happening: "ripRequested", tracks: TRACKS.length },
	]);

	// Act
	window.dispatchEvent(
		new ErrorEvent("error", {
			error: new TypeError("the drive stopped responding"),
		}),
	);

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
