import { useState } from "react";
import { Button } from "./components/ui/button";
import { Licenses } from "./features/licenses/Licenses";
import { Ripper } from "./features/ripping/Ripper";
import "./index.css";

function App() {
	const [screen, setScreen] = useState<"ripping" | "licenses">("ripping");

	return (
		<main className="flex flex-col items-center gap-4 pt-[10vh] text-center">
			<h1 className="text-2xl font-semibold">Uncompressed CD Ripper</h1>

			{screen === "ripping" ? (
				<>
					<Ripper />

					<Button
						variant="link"
						size="sm"
						onClick={() => setScreen("licenses")}
					>
						Licenses
					</Button>
				</>
			) : (
				<>
					{/* Above the list, which is far too long to scroll back up. */}
					<div className="flex w-full max-w-xl">
						<Button
							variant="outline"
							size="sm"
							onClick={() => setScreen("ripping")}
						>
							Back
						</Button>
					</div>

					<Licenses />
				</>
			)}
		</main>
	);
}

export default App;
