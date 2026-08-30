import { open } from "@tauri-apps/plugin-dialog";
import { type Artwork, commands } from "@/bindings";
import { expectOk } from "../error-report/backend";
import i18next from "../language/i18n";

// An image element is given somewhere to fetch an image from, and this one has
// already crossed from the backend. This is the address of artwork in hand.
export function shown(artwork: Artwork) {
	return `data:${artwork.mediaType};base64,${artwork.data}`;
}

// The two kinds a picture block is written with here. A file of any other kind
// is turned down once it is read, so the picker does not offer one.
const EXTENSIONS = ["png", "jpg", "jpeg"];

// Null is closing the picker, which keeps the artwork that is already there.
export async function chosen(): Promise<Artwork | null> {
	const path = await open({
		filters: [{ name: i18next.t("artwork.images"), extensions: EXTENSIONS }],
	});

	if (path === null) {
		return null;
	}

	return await expectOk(commands.readArtwork(path));
}
