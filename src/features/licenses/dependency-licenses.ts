import generated from "./dependency-licenses.json";

export type DependencyLicense = {
	name: string;
	version: string;
	license: string;
	texts: string[];
};

export const dependencyLicenses: DependencyLicense[] = generated;
