import { open } from "@tauri-apps/plugin-dialog";
import { type Cover, commands } from "@/bindings";
import { expectOk } from "../error-report/backend";

// An image element is given somewhere to fetch a picture from, and the picture
// has already crossed from the backend. This is the address of one in hand.
export function shown(cover: Cover) {
	return `data:${cover.mediaType};base64,${cover.data}`;
}

// The two kinds a picture block is written with here. A file of any other kind
// is turned down once it is read, so the picker does not offer one.
const PICTURES = { name: "Pictures", extensions: ["png", "jpg", "jpeg"] };

// Null is closing the picker, which keeps the artwork that is already there.
export async function chosen(): Promise<Cover | null> {
	const path = await open({ filters: [PICTURES] });

	if (path === null) {
		return null;
	}

	return await expectOk(commands.readArtwork(path));
}
