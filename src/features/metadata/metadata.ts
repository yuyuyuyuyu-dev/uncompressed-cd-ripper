import type { Album, TrackTags } from "@/bindings";

// What will be written, as it is being typed. Not an Album, which is one
// answer a server gave about the disc: an answer is somewhere to start from,
// and what ends up in the files is whatever is on screen when the rip does.
export type Metadata = {
	album: string;
	artist: string;
	titles: Map<number, string>;
};

export const NOTHING: Metadata = { album: "", artist: "", titles: new Map() };

export function fromAlbum(album: Album): Metadata {
	return {
		album: album.title,
		artist: album.artist,
		titles: new Map(album.tracks.map((track) => [track.number, track.title])),
	};
}

export function titleOf(metadata: Metadata, number: number) {
	return metadata.titles.get(number) ?? "";
}

export function withAlbum(metadata: Metadata, album: string): Metadata {
	return { ...metadata, album };
}

export function withArtist(metadata: Metadata, artist: string): Metadata {
	return { ...metadata, artist };
}

export function withTitle(
	metadata: Metadata,
	number: number,
	title: string,
): Metadata {
	return { ...metadata, titles: new Map(metadata.titles).set(number, title) };
}

// Trimmed, because a file name is trimmed whatever is typed, and a tag holding
// the spaces would disagree with the name of the file it sits in.
function written(text: string) {
	const trimmed = text.trim();

	return trimmed === "" ? null : trimmed;
}

// Both the tags and the file name come from here, so a file cannot end up
// named after a title it was not tagged with.
export function fileTitle(metadata: Metadata, number: number) {
	return written(titleOf(metadata, number));
}

// A file is better untagged than tagged wrongly: a field left blank is left
// out, and a disc nothing was said about at all is tagged with nothing.
export function tagsFor(metadata: Metadata, number: number): TrackTags | null {
	const album = written(metadata.album);
	const artist = written(metadata.artist);
	const title = fileTitle(metadata, number);

	if (album === null && artist === null && title === null) {
		return null;
	}

	return { album, artist, title };
}
