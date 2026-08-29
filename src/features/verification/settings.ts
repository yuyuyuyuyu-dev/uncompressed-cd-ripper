import { load } from "@tauri-apps/plugin-store";

// Where what the app is set to is kept between runs. Tauri settles where this
// file goes on each platform, which is the part that would otherwise have to
// be got right three times.
const SETTINGS = "settings.json";

// A read offset is filed under what the drive calls itself rather than under
// the device path it answered to: the path is whatever the operating system
// handed out this time, and the offset belongs to the drive.
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
	// Written out now rather than whenever the store next feels like it: what
	// this is for is surviving the window being closed.
	await settings.save();
}

// Whether a rip is held against other people's once it is done. One answer for
// the app rather than one per drive: it is a decision about what leaves the
// machine, and that does not change with which drive the disc went into.
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
