import { useState } from "react";
import { commands } from "@/bindings";
import { Button } from "@/components/ui/button";
import { expectOk } from "./features/error-report/backend";
import "./index.css";

function App() {
	const [broken, setBroken] = useState(false);

	if (broken) {
		throw new TypeError("the track listing could not be drawn");
	}

	return (
		<>
			<main className="flex flex-col items-center gap-4 pt-[10vh] text-center">
				<h1 className="text-2xl font-semibold">Uncompressed CD Ripper</h1>

				{/* Standing in for the ripping that does not exist yet, so that the
			    error reporting can be walked through by hand. */}
				<Button
					variant="destructive"
					onClick={() => {
						throw new TypeError(
							"the drive stopped responding while reading track 3",
						);
					}}
				>
					Throw an error
				</Button>

				<Button
					variant="destructive"
					onClick={async () => {
						await expectOk(commands.failDeliberately());
					}}
				>
					Fail in the backend
				</Button>

				<Button variant="destructive" onClick={() => setBroken(true)}>
					Break the interface
				</Button>
			</main>
		</>
	);
}

export default App;
