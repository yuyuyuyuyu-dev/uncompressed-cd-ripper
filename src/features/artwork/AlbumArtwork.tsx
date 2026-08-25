import { ImageOff, LoaderCircle } from "lucide-react";
import type { Artwork } from "@/bindings";
import { shown } from "./artwork";

type Props = {
	artwork: Artwork | null;
	looking: boolean;
};

function inside(artwork: Artwork | null, looking: boolean) {
	if (looking) {
		return (
			<LoaderCircle className="size-5 animate-spin text-muted-foreground" />
		);
	}

	if (artwork === null) {
		return <ImageOff className="size-5 text-muted-foreground" />;
	}

	// Whole rather than filling the square: artwork is not always square, and
	// the part cropped off is somebody's work.
	return (
		<img
			src={shown(artwork)}
			alt="Album artwork"
			className="size-full object-contain"
		/>
	);
}

// The same size whatever is in it, so that the fields beside it do not move as
// artwork arrives.
export function AlbumArtwork({ artwork, looking }: Props) {
	return (
		<div className="flex size-24 shrink-0 items-center justify-center overflow-hidden rounded-lg border bg-muted">
			{inside(artwork, looking)}
		</div>
	);
}
