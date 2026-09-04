export class BackendError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "BackendError";
	}
}

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: string };

export async function expectOk<T>(call: Promise<Result<T>>): Promise<T> {
	const result = await call;

	if (result.status === "error") {
		throw new BackendError(result.error);
	}

	return result.data;
}
