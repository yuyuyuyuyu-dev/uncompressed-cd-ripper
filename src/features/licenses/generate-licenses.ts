import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { build, type Rollup } from "vite";
import type { DependencyLicense } from "./dependency-licenses";

const HERE = import.meta.dirname;
const ROOT = join(HERE, "../../..");
const LICENSES = join(HERE, "dependency-licenses.json");

function textsIn(directory: string) {
	return readdirSync(directory)
		.filter((file) => /^(licen[cs]e|copying)/i.test(file))
		.sort()
		.map((file) => readFileSync(join(directory, file), "utf8"));
}

function packageOf(file: string) {
	for (
		let directory = dirname(file);
		directory.includes("node_modules");
		directory = dirname(directory)
	) {
		if (!existsSync(join(directory, "package.json"))) {
			continue;
		}

		const { name, version, license } = JSON.parse(
			readFileSync(join(directory, "package.json"), "utf8"),
		);

		if (typeof name === "string" && typeof version === "string") {
			return {
				name,
				version,
				license: typeof license === "string" ? license : "",
				texts: textsIn(directory),
			};
		}
	}
}

function packagesBehind(files: readonly string[]) {
	const packages = new Map<string, DependencyLicense>();

	for (const file of files.filter((file) => file.includes("node_modules"))) {
		const found = packageOf(file);

		if (found !== undefined) {
			packages.set(`${found.name}@${found.version}`, found);
		}
	}

	return [...packages.values()];
}

async function bundled() {
	let files: readonly string[] = [];

	await build({
		configFile: join(ROOT, "vite.config.ts"),
		logLevel: "warn",
		build: { write: false },
		plugins: [
			{
				name: "collect-licenses",
				generateBundle(this: Rollup.PluginContext) {
					files = this.getWatchFiles();
				},
			},
		],
	});

	return packagesBehind(files);
}

type Crate = { name: string; version: string };

type About = {
	licenses: { text: string; used_by: { crate: Crate }[] }[];
	crates: { package: Crate; license: string | null }[];
};

function linked(): DependencyLicense[] {
	const about: About = JSON.parse(
		execFileSync("cargo", ["about", "generate", "--format", "json"], {
			cwd: join(ROOT, "src-tauri"),
			encoding: "utf8",
			maxBuffer: Number.POSITIVE_INFINITY,
		}),
	);

	const texts = new Map<string, Set<string>>();

	for (const { text, used_by } of about.licenses) {
		for (const { crate } of used_by) {
			const key = `${crate.name}@${crate.version}`;

			texts.set(key, (texts.get(key) ?? new Set()).add(text));
		}
	}

	return about.crates.map(({ package: crate, license }) => ({
		name: crate.name,
		version: crate.version,
		license: license ?? "",
		texts: [...(texts.get(`${crate.name}@${crate.version}`) ?? [])],
	}));
}

function order(a: DependencyLicense, b: DependencyLicense) {
	return a.name.localeCompare(b.name) || a.version.localeCompare(b.version);
}

async function main() {
	if (!existsSync(LICENSES)) {
		writeFileSync(LICENSES, "[]");
	}

	const licenses = [...(await bundled()), ...linked()].sort(order);

	writeFileSync(LICENSES, `${JSON.stringify(licenses, null, "\t")}\n`);
}

await main();
