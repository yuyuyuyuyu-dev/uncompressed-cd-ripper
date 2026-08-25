import type { Album, Artwork, TrackTags } from "@/bindings";

// What will be written, as it is being typed. Not an Album, which is one
// answer a server gave about the disc: an answer is somewhere to start from,
// and what ends up in the files is whatever is on screen when the rip does.
//
// The album has one artist and so does every track, because a compilation is
// a disc where those are not the same.
export type Metadata = {
	album: string;
	albumArtist: string;
	titles: Map<number, string>;
	artists: Map<number, string>;
	// One for the disc rather than one per track, and nothing where none was
	// found or none was asked for.
	artwork: Artwork | null;
};

export const NOTHING: Metadata = {
	album: "",
	albumArtist: "",
	titles: new Map(),
	artists: new Map(),
	artwork: null,
};

export function fromAlbum(album: Album): Metadata {
	return {
		album: album.title,
		albumArtist: album.artist,
		titles: new Map(album.tracks.map((track) => [track.number, track.title])),
		artists: new Map(album.tracks.map((track) => [track.number, track.artist])),
		// The artwork is asked for after the album is, from somewhere else, so
		// this is what there is until that answer arrives.
		artwork: null,
	};
}

export function withArtwork(
	metadata: Metadata,
	artwork: Artwork | null,
): Metadata {
	return { ...metadata, artwork };
}

export function titleOf(metadata: Metadata, number: number) {
	return metadata.titles.get(number) ?? "";
}

export function artistOf(metadata: Metadata, number: number) {
	return metadata.artists.get(number) ?? "";
}

export function withAlbum(metadata: Metadata, album: string): Metadata {
	return { ...metadata, album };
}

export function withAlbumArtist(
	metadata: Metadata,
	albumArtist: string,
): Metadata {
	return { ...metadata, albumArtist };
}

export function withTitle(
	metadata: Metadata,
	number: number,
	title: string,
): Metadata {
	return { ...metadata, titles: new Map(metadata.titles).set(number, title) };
}

export function withArtist(
	metadata: Metadata,
	number: number,
	artist: string,
): Metadata {
	return {
		...metadata,
		artists: new Map(metadata.artists).set(number, artist),
	};
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
	const tags = {
		album: written(metadata.album),
		albumArtist: written(metadata.albumArtist),
		artist: written(artistOf(metadata, number)),
		title: fileTitle(metadata, number),
		artwork: metadata.artwork,
	};

	return Object.values(tags).every((field) => field === null) ? null : tags;
}
