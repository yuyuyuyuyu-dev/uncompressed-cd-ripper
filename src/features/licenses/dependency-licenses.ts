import generated from "./dependency-licenses.json";

export type DependencyLicense = {
	name: string;
	version: string;
	license: string;
	texts: string[];
};

// Written by the generate-licenses script, which the pnpm pre scripts run.
export const dependencyLicenses: DependencyLicense[] = generated;
