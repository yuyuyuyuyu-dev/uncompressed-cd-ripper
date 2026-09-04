import { ImageOff, LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Artwork } from "@/bindings";
import "../language/i18n";
import { shown } from "./artwork";

type Props = {
	artwork: Artwork | null;
	looking: boolean;
};

function Inside({ artwork, looking }: Props) {
	const { t } = useTranslation();

	if (looking) {
		return (
			<LoaderCircle className="size-5 animate-spin text-muted-foreground" />
		);
	}

	if (artwork === null) {
		return <ImageOff className="size-5 text-muted-foreground" />;
	}

	return (
		<img
			src={shown(artwork)}
			alt={t("artwork.alt")}
			className="size-full object-contain"
		/>
	);
}

export function AlbumArtwork({ artwork, looking }: Props) {
	return (
		<div className="flex size-24 shrink-0 items-center justify-center overflow-hidden rounded-lg border bg-muted">
			<Inside artwork={artwork} looking={looking} />
		</div>
	);
}
