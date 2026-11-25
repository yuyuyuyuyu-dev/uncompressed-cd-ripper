export class BackendError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BackendError";
  }
}

export async function expectOk<T>(call: Promise<T>): Promise<T> {
  try {
    return await call;
  } catch (error) {
    throw error instanceof Error ? error : new BackendError(String(error));
  }
}
