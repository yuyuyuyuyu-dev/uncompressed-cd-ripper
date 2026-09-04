import { load } from "@tauri-apps/plugin-store";

const SETTINGS = "settings.json";

function offsetOf(drive: string) {
	return `read-offset:${drive}`;
}

export async function savedOffset(drive: string) {
	const settings = await load(SETTINGS);

	return await settings.get<number>(offsetOf(drive));
}

export async function saveOffset(drive: string, offset: number) {
	const settings = await load(SETTINGS);

	await settings.set(offsetOf(drive), offset);
	await settings.save();
}

const CHECKING = "check-against-accuraterip";

export async function savedChecking() {
	const settings = await load(SETTINGS);

	return (await settings.get<boolean>(CHECKING)) ?? false;
}

export async function saveChecking(checking: boolean) {
	const settings = await load(SETTINGS);

	await settings.set(CHECKING, checking);
	await settings.save();
}
