import { LoaderCircle } from "lucide-react";
import {
	type Dispatch,
	type SetStateAction,
	useId,
	useRef,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
	type Album,
	type Artwork,
	commands,
	type Track,
	type Verdict,
} from "@/bindings";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { AlbumArtwork } from "../artwork/AlbumArtwork";
import { ChooseArtwork } from "../artwork/ChooseArtwork";
import { expectOk } from "../error-report/backend";
import "../language/i18n";
import { length } from "../ripping/track";
import { matching } from "../verification/verdicts";
import {
	artistOf,
	fromAlbum,
	type Metadata,
	titleOf,
	withAlbum,
	withAlbumArtist,
	withArtist,
	withArtwork,
	withTitle,
} from "./metadata";

function pressing(album: Album) {
	return [album.released, album.country].filter(Boolean).join(" · ");
}

function status(looking: boolean, matches: Album[] | undefined) {
	if (looking || matches === undefined) {
		return undefined;
	}

	if (matches.length === 0) {
		return "metadata.unknown" as const;
	}

	return matches.length > 1 ? ("metadata.choose" as const) : undefined;
}

type Props = {
	drive: string | undefined;
	tracks: Track[];
	metadata: Metadata;
	verdicts: Verdict[] | undefined;
	onChange: Dispatch<SetStateAction<Metadata>>;
	disabled: boolean;
};

export function DiscMetadata({
	drive,
	tracks,
	metadata,
	verdicts,
	onChange,
	disabled,
}: Props) {
	const [asking, setAsking] = useState(false);
	const [matches, setMatches] = useState<Album[]>();
	const [chosen, setChosen] = useState<string>();
	const [looking, setLooking] = useState(false);
	const [fetchingArtwork, setFetchingArtwork] = useState(false);
	const { t } = useTranslation();

	const asked = useRef(0);

	const albumField = useId();
	const albumArtistField = useId();

	const takeArtwork = async (release: string) => {
		const mine = asked.current + 1;
		asked.current = mine;

		setFetchingArtwork(true);

		try {
			const artwork = await expectOk(commands.lookUpArtwork(release));

			if (asked.current === mine) {
				onChange((current) => withArtwork(current, artwork));
			}
		} finally {
			if (asked.current === mine) {
				setFetchingArtwork(false);
			}
		}
	};

	const takeChosen = (artwork: Artwork) => {
		asked.current += 1;
		setFetchingArtwork(false);
		onChange((current) => withArtwork(current, artwork));
	};

	const take = (album: Album) => {
		setChosen(album.id);
		onChange(fromAlbum(album));
		takeArtwork(album.id);
	};

	const look = async () => {
		if (drive === undefined) {
			return;
		}

		setAsking(false);
		setLooking(true);

		try {
			const found = await expectOk(commands.lookUpDisc(drive));

			setMatches(found);

			if (found.length === 1) {
				take(found[0]);
			}
		} finally {
			setLooking(false);
		}
	};

	const said = status(looking, matches);

	const frozen = disabled || looking;

	return (
		<div className="flex flex-col gap-3">
			<div className="flex items-center gap-2">
				<h2 className="font-semibold">{t("metadata.heading")}</h2>
				<Button
					variant="outline"
					size="sm"
					onClick={() => setAsking(true)}
					disabled={frozen || drive === undefined}
				>
					{looking && <LoaderCircle className="animate-spin" />}
					{looking ? t("metadata.lookingUp") : t("metadata.lookUp")}
				</Button>
			</div>

			{said !== undefined && (
				<p className="text-muted-foreground text-sm">{t(said)}</p>
			)}

			{matches !== undefined && matches.length > 1 && (
				<ul className="flex flex-col gap-1">
					{matches.map((match) => (
						<li key={match.id}>
							<Button
								variant={match.id === chosen ? "default" : "outline"}
								size="sm"
								className="h-auto w-full justify-start py-1.5 text-left"
								onClick={() => take(match)}
								disabled={frozen}
							>
								<span className="truncate">
									{match.title} — {match.artist}
									{pressing(match) !== "" && (
										<span className="opacity-70"> ({pressing(match)})</span>
									)}
								</span>
							</Button>
						</li>
					))}
				</ul>
			)}

			<div className="flex items-start gap-3">
				<div className="flex shrink-0 flex-col items-center gap-2">
					<AlbumArtwork
						artwork={metadata.artwork}
						looking={looking || fetchingArtwork}
					/>
					<ChooseArtwork onChoose={takeChosen} disabled={frozen} />
				</div>

				<div className="grid flex-1 grid-cols-[auto_1fr] items-center gap-x-3 gap-y-2">
					<label className="text-sm" htmlFor={albumField}>
						{t("metadata.album")}
					</label>
					<Input
						id={albumField}
						value={metadata.album}
						onChange={(event) =>
							onChange(withAlbum(metadata, event.target.value))
						}
						disabled={frozen}
					/>

					<label className="text-sm" htmlFor={albumArtistField}>
						{t("metadata.albumArtist")}
					</label>
					<Input
						id={albumArtistField}
						value={metadata.albumArtist}
						onChange={(event) =>
							onChange(withAlbumArtist(metadata, event.target.value))
						}
						disabled={frozen}
					/>
				</div>
			</div>

			{tracks.length > 0 && (
				<table className="w-full">
					<thead>
						<tr className="border-b text-muted-foreground text-sm">
							<th className="w-8 pb-1.5 text-left font-medium">#</th>
							<th className="w-[44%] px-2 pb-1.5 text-left font-medium">
								{t("metadata.title")}
							</th>
							<th className="px-2 pb-1.5 text-left font-medium">
								{t("metadata.artist")}
							</th>
							<th className="pb-1.5 text-right font-medium">
								{t("metadata.length")}
							</th>
							<th className="pb-1.5 pl-2 text-right font-medium">
								{t("metadata.matching")}
							</th>
						</tr>
					</thead>
					<tbody>
						{tracks.map((track, index) => (
							<tr key={track.number}>
								<td className="py-1.5 text-sm tabular-nums">
									{String(track.number).padStart(2, "0")}
								</td>
								<td className="px-2 py-1.5">
									<Input
										aria-label={t("metadata.trackTitle", {
											number: track.number,
										})}
										value={titleOf(metadata, track.number)}
										onChange={(event) =>
											onChange(
												withTitle(metadata, track.number, event.target.value),
											)
										}
										disabled={frozen}
									/>
								</td>
								<td className="px-2 py-1.5">
									<Input
										aria-label={t("metadata.trackArtist", {
											number: track.number,
										})}
										value={artistOf(metadata, track.number)}
										onChange={(event) =>
											onChange(
												withArtist(metadata, track.number, event.target.value),
											)
										}
										disabled={frozen}
									/>
								</td>
								<td className="py-1.5 text-right text-muted-foreground text-sm tabular-nums">
									{length(track.sectors)}
								</td>
								<td className="py-1.5 pl-2 text-right text-muted-foreground text-sm tabular-nums">
									{matching(verdicts?.[index])}
								</td>
							</tr>
						))}
					</tbody>
				</table>
			)}

			<Dialog open={asking} onOpenChange={(open) => !open && setAsking(false)}>
				<DialogContent className="flex max-h-[85vh] flex-col">
					<DialogHeader className="min-h-0">
						<DialogTitle>{t("metadata.ask.title")}</DialogTitle>
						<DialogDescription className="-mr-4 min-h-0 overflow-y-auto pr-4 whitespace-pre-line">
							{t("metadata.ask.body")}
						</DialogDescription>
					</DialogHeader>

					<DialogFooter>
						<Button variant="outline" onClick={() => setAsking(false)}>
							{t("cancel")}
						</Button>
						<Button onClick={look}>{t("metadata.ask.confirm")}</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
