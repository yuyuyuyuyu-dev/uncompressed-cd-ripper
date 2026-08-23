import { ImageOff, LoaderCircle } from "lucide-react";
import type { Cover } from "@/bindings";
import { shown } from "./artwork";

type Props = {
	cover: Cover | null;
	looking: boolean;
};

function inside(cover: Cover | null, looking: boolean) {
	if (looking) {
		return (
			<LoaderCircle className="size-5 animate-spin text-muted-foreground" />
		);
	}

	if (cover === null) {
		return <ImageOff className="size-5 text-muted-foreground" />;
	}

	// Whole rather than filling the square: a sleeve is not always square, and
	// the part cropped off is somebody's cover.
	return (
		<img
			src={shown(cover)}
			alt="Cover art"
			className="size-full object-contain"
		/>
	);
}

// The same size whatever is in it, so that the fields beside it do not move as
// a sleeve arrives.
export function CoverArt({ cover, looking }: Props) {
	return (
		<div className="flex size-24 shrink-0 items-center justify-center overflow-hidden rounded-lg border bg-muted">
			{inside(cover, looking)}
		</div>
	);
}
