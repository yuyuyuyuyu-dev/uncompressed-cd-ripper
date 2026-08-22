import { useState } from "react";
import { Button } from "./components/ui/button";
import { Licenses } from "./features/licenses/Licenses";
import { Ripper } from "./features/ripping/Ripper";
import "./index.css";

function App() {
	const [screen, setScreen] = useState<"ripping" | "licenses">("ripping");

	return (
		<>
			{/* Stuck to the top, because the license screen runs to thousands of
			    lines and the way out of it has to stay in reach from all of them. */}
			<header className="sticky top-0 z-10 flex items-center gap-2 border-b bg-background px-3 py-2">
				<div className="flex flex-1 justify-start">
					{screen === "licenses" && (
						<Button
							variant="outline"
							size="sm"
							onClick={() => setScreen("ripping")}
						>
							Back
						</Button>
					)}
				</div>

				<h1 className="font-semibold text-sm">Uncompressed CD Ripper</h1>

				<div className="flex flex-1 justify-end">
					{screen === "ripping" && (
						<Button
							variant="ghost"
							size="sm"
							onClick={() => setScreen("licenses")}
						>
							Licenses
						</Button>
					)}
				</div>
			</header>

			<main className="flex flex-col items-center gap-4 py-[10vh] text-center">
				{screen === "ripping" ? <Ripper /> : <Licenses />}
			</main>
		</>
	);
}

export default App;
