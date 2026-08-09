import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, test } from "vitest";
import App from "./App";

afterEach(clearMocks);

test("should show the greeting the backend returns for the submitted name", async () => {
	// Arrange
	mockIPC((command, payload) =>
		command === "greet"
			? `Hello, ${(payload as { name: string }).name}!`
			: undefined,
	);
	const user = userEvent.setup();
	render(<App />);

	// Act
	await user.type(screen.getByRole("textbox"), "yu");
	await user.click(screen.getByRole("button", { name: "Greet" }));

	// Assert
	expect(await screen.findByText("Hello, yu!")).toBeInTheDocument();
});

test("should send the submitted name to the greet command", async () => {
	// Arrange
	const calls: { command: string; payload: unknown }[] = [];
	mockIPC((command, payload) => {
		calls.push({ command, payload });
		return "";
	});
	const user = userEvent.setup();
	render(<App />);

	// Act
	await user.type(screen.getByRole("textbox"), "yu");
	await user.click(screen.getByRole("button", { name: "Greet" }));

	// Assert
	expect(calls).toEqual([{ command: "greet", payload: { name: "yu" } }]);
});
