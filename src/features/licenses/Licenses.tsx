import { useTranslation } from "react-i18next";
import "../language/i18n";
import { dependencyLicenses } from "./dependency-licenses";

export function Licenses() {
	const { t } = useTranslation();

	return (
		<section className="flex w-full max-w-xl flex-col gap-4 text-left">
			<h2 className="font-semibold">{t("licenses.heading")}</h2>

			<p className="text-muted-foreground text-sm">{t("licenses.about")}</p>

			<ol className="flex flex-col gap-3">
				{dependencyLicenses.map((dependency) => (
					<li
						key={`${dependency.name} ${dependency.version}`}
						className="rounded-lg border p-3"
					>
						<h3 className="font-medium text-sm">
							{dependency.name} {dependency.version}
						</h3>

						<p className="text-muted-foreground text-xs">
							{dependency.license}
						</p>

						{/* Scrolls rather than stretches, so that one long license does
						    not bury the library after it. */}
						<pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap text-xs">
							{dependency.texts.join("\n\n")}
						</pre>
					</li>
				))}
			</ol>
		</section>
	);
}
