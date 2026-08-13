export class BackendError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "BackendError";
	}
}

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: string };

// A command that failed comes back as a value rather than a rejection, so it
// reads like one that worked unless the caller remembers to look. Turning it
// into a throw puts failures from the backend on the same path as everything
// else the app throws, which is the path the reporter already watches.
export async function expectOk<T>(call: Promise<Result<T>>): Promise<T> {
	const result = await call;

	if (result.status === "error") {
		throw new BackendError(result.error);
	}

	return result.data;
}
