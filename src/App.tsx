import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import reactLogo from "./assets/react.svg";
import "./index.css";

function App() {
	const [greetMsg, setGreetMsg] = useState("");
	const [name, setName] = useState("");

	async function greet() {
		// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
		setGreetMsg(await invoke("greet", { name }));
	}

	return (
		<main className="flex flex-col items-center gap-4 pt-[10vh] text-center">
			<h1 className="text-2xl font-semibold">Welcome to Tauri + React</h1>

			<div className="flex justify-center">
				<a href="https://vite.dev" target="_blank" rel="noopener">
					<img
						src="/vite.svg"
						className="h-24 p-6 transition duration-700 hover:drop-shadow-[0_0_2em_#747bff]"
						alt="Vite logo"
					/>
				</a>
				<a href="https://tauri.app" target="_blank" rel="noopener">
					<img
						src="/tauri.svg"
						className="h-24 p-6 transition duration-700 hover:drop-shadow-[0_0_2em_#24c8db]"
						alt="Tauri logo"
					/>
				</a>
				<a href="https://react.dev" target="_blank" rel="noopener">
					<img
						src={reactLogo}
						className="h-24 p-6 transition duration-700 hover:drop-shadow-[0_0_2em_#61dafb]"
						alt="React logo"
					/>
				</a>
			</div>
			<p>Click on the Tauri, Vite, and React logos to learn more.</p>

			<form
				className="flex justify-center gap-2"
				onSubmit={(e) => {
					e.preventDefault();
					greet();
				}}
			>
				<Input
					id="greet-input"
					onChange={(e) => setName(e.currentTarget.value)}
					placeholder="Enter a name..."
				/>
				<Button type="submit">Greet</Button>
			</form>
			<p>{greetMsg}</p>
		</main>
	);
}

export default App;
