import { Ripper } from "./features/ripping/Ripper";
import "./index.css";

function App() {
	return (
		<main className="flex flex-col items-center gap-4 pt-[10vh] text-center">
			<h1 className="text-2xl font-semibold">Uncompressed CD Ripper</h1>

			<Ripper />
		</main>
	);
}

export default App;
