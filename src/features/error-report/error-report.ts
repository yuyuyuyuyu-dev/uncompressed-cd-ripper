import type { Breadcrumb, Environment, ErrorReport } from "@/bindings";

export function newEventId() {
	return crypto.randomUUID().replace(/-/g, "");
}

export function describe(thrown: unknown) {
	if (thrown instanceof Error) {
		return {
			type: thrown.name,
			value: thrown.message,
			stacktrace: thrown.stack ?? "",
		};
	}

	return { type: "UnknownError", value: String(thrown), stacktrace: "" };
}

export function buildErrorReport({
	eventId,
	thrown,
	componentStack,
	environment,
	occurredAt,
	comment,
	breadcrumbs,
}: {
	eventId: string;
	thrown: unknown;
	componentStack: string;
	environment: Environment;
	occurredAt: Date;
	comment: string;
	breadcrumbs: Breadcrumb[];
}): ErrorReport {
	const { type, value, stacktrace } = describe(thrown);

	return {
		event_id: eventId,
		timestamp: occurredAt.toISOString(),
		platform: "javascript",
		release: environment.release,
		exception: { values: [{ type, value }] },
		breadcrumbs: { values: breadcrumbs },
		contexts: {
			os: { name: environment.osName, version: environment.osVersion },
			device: { arch: environment.architecture },
		},
		tags: { architecture: environment.architecture },
		extra: { stacktrace, component_stack: componentStack, comment },
	};
}
