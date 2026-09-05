import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./components/ui/button";
import "./features/language/i18n";
import { Licenses } from "./features/licenses/Licenses";
import { Ripper } from "./features/ripping/Ripper";
import { SelfUpdate } from "./features/self-update/SelfUpdate";
import "./index.css";

function App() {
	const [screen, setScreen] = useState<"ripping" | "licenses">("ripping");
	const [version, setVersion] = useState<string>();
	const { t } = useTranslation();

	useEffect(() => {
		getVersion().then(setVersion);
	}, []);

	return (
		<div className="flex min-h-screen flex-col">
			<header className="sticky top-0 z-10 flex items-center gap-2 border-b bg-background px-3 py-2">
				<div className="flex flex-1 justify-start">
					{screen === "licenses" && (
						<Button
							variant="outline"
							size="sm"
							onClick={() => setScreen("ripping")}
						>
							{t("app.back")}
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
							{t("app.licenses")}
						</Button>
					)}
				</div>
			</header>

			<main className="flex flex-1 flex-col items-center gap-4 pt-[10vh] text-center">
				{screen === "ripping" ? <Ripper /> : <Licenses />}
			</main>

			<footer className="px-3 py-[5vh] text-center text-muted-foreground text-xs">
				{version}
			</footer>

			<SelfUpdate />
		</div>
	);
}

export default App;
