import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorReporter } from "./features/error-report/ErrorReporter";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		{/* Outside whatever fails, so that a window which cannot be drawn still
		    has something left to report it from. */}
		<ErrorReporter>
			<App />
		</ErrorReporter>
	</React.StrictMode>,
);
