import type { Album, Artwork, TrackTags } from "@/bindings";

export type Metadata = {
	album: string;
	albumArtist: string;
	titles: Map<number, string>;
	artists: Map<number, string>;
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

function written(text: string) {
	const trimmed = text.trim();

	return trimmed === "" ? null : trimmed;
}

export function fileTitle(metadata: Metadata, number: number) {
	return written(titleOf(metadata, number));
}

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
