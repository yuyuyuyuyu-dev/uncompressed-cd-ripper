import { open } from "@tauri-apps/plugin-dialog";
import { type Artwork, commands } from "@/bindings";
import { expectOk } from "../error-report/backend";
import i18next from "../language/i18n";

export function shown(artwork: Artwork) {
	return `data:${artwork.mediaType};base64,${artwork.data}`;
}

const EXTENSIONS = ["png", "jpg", "jpeg"];

export async function chosen(): Promise<Artwork | null> {
	const path = await open({
		filters: [{ name: i18next.t("artwork.images"), extensions: EXTENSIONS }],
	});

	if (path === null) {
		return null;
	}

	return await expectOk(commands.readArtwork(path));
}
