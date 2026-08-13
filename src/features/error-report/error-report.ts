import type { Environment, ErrorReport } from "@/bindings";

// Sentry wants 32 hexadecimal characters with the dashes taken out. Minted
// once when the error is caught rather than each time the report is built,
// which would otherwise leave the report on screen and the report sent
// carrying different identifiers.
export function newEventId() {
	return crypto.randomUUID().replace(/-/g, "");
}

// Anything at all can be thrown in JavaScript, and a report is still owed for
// whatever arrives.
function describe(thrown: unknown) {
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
	environment,
	occurredAt,
	comment,
}: {
	eventId: string;
	thrown: unknown;
	environment: Environment;
	occurredAt: Date;
	comment: string;
}): ErrorReport {
	const { type, value, stacktrace } = describe(thrown);

	return {
		event_id: eventId,
		timestamp: occurredAt.toISOString(),
		platform: "javascript",
		release: environment.release,
		exception: { values: [{ type, value }] },
		contexts: {
			os: { name: environment.osName, version: environment.osVersion },
			device: { arch: environment.architecture },
		},
		tags: { architecture: environment.architecture },
		extra: { stacktrace, comment },
	};
}
