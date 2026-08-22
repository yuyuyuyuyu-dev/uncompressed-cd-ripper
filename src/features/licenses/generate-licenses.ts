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

// Walks out of the file to the directory the package declares itself in.
// Packages carry further package.json files below that one, which name nothing
// and only say how the files beside them are to be read.
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

// Which packages a build pulls in is the question being asked, so the answer
// comes from a build rather than from the manifest. Nothing is written out:
// the bundle is thrown away and only the files behind it are kept. Those
// include what the stylesheet reaches, which no bundled module names: the
// font, and the packages the theme is built from.
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

// cargo-about reads the license out of every crate the app is linked against,
// following what Cargo resolved rather than what Cargo.toml asks for.
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
	// The screen imports this file, so the build below cannot resolve its
	// imports until there is something here to read.
	if (!existsSync(LICENSES)) {
		writeFileSync(LICENSES, "[]");
	}

	const licenses = [...(await bundled()), ...linked()].sort(order);

	writeFileSync(LICENSES, `${JSON.stringify(licenses, null, "\t")}\n`);
}

await main();
