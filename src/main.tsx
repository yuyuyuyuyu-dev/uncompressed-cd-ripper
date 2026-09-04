import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorReporter } from "./features/error-report/ErrorReporter";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<ErrorReporter>
			<App />
		</ErrorReporter>
	</React.StrictMode>,
);
