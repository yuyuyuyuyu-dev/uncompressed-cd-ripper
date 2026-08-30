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

// What tells one pressing from another when the record is the same: two of
// them can agree on the title and the artist and differ only here.
function pressing(album: Album) {
	return [album.released, album.country].filter(Boolean).join(" · ");
}

// Only where the lookup has something to say. The fields below say the rest:
// what is in them is what will be written, whether it was answered or typed.
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
	// What AccurateRip said about each track once they were ripped, in the
	// order they play, and nothing until they have been. It belongs in this
	// table rather than in one of its own: it is one more thing to know about
	// a track, beside its title and how long it is.
	//
	// The column stands there from the start, holding nothing. A column that
	// appears once the reading is done moves every other column as it arrives,
	// and says nothing beforehand about what is coming.
	verdicts: Verdict[] | undefined;
	// Taking what to change it into rather than what to change it to, because
	// the artwork lands after the fields it arrives beside are already there to
	// be typed in.
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
	// A different disc is a different question. None of this survives one being
	// put in, which is arranged by the drive being this component's key rather
	// than by anything here.
	const [asking, setAsking] = useState(false);
	const [matches, setMatches] = useState<Album[]>();
	const [chosen, setChosen] = useState<string>();
	const [looking, setLooking] = useState(false);
	const [fetchingArtwork, setFetchingArtwork] = useState(false);
	const { t } = useTranslation();

	// Which request the artwork on its way belongs to. Clicking through the
	// matches starts one for each, and an earlier answer arriving late would
	// otherwise settle on the album that was clicked after it.
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

	// Artwork chosen by hand settles it: the request it stops waiting for is one
	// whose answer would otherwise land on top of it.
	const takeChosen = (artwork: Artwork) => {
		asked.current += 1;
		setFetchingArtwork(false);
		onChange((current) => withArtwork(current, artwork));
	};

	// Not waited for: the album and the track titles are there to be read and
	// corrected while the artwork is still coming.
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

			// Choosing between one thing is not a choice.
			if (found.length === 1) {
				take(found[0]);
			}
		} finally {
			setLooking(false);
		}
	};

	const said = status(looking, matches);

	// An answer on its way overwrites every one of these, so typing into them
	// now is typing into something that is about to be taken away.
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
					{/* The dialog closes as the lookup starts, so the eye is on the
					    button that has just gone rather than on this one. Something
					    turning is what carries across the gap: a button that only goes
					    grey is a still picture, and a still picture is missed. */}
					{looking && <LoaderCircle className="animate-spin" />}
					{looking ? t("metadata.lookingUp") : t("metadata.lookUp")}
				</Button>
			</div>

			{said !== undefined && (
				<p className="text-muted-foreground text-sm">{t(said)}</p>
			)}

			{/* Every one of them, and what tells them apart. Picking for the user
			    would mean writing somebody else's track titles into their files.
			    It stays on screen after one is picked, because the wrong one is
			    only obvious once its track titles are against the disc. */}
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
									{/* Faded rather than a colour of its own, because the one
									    that is picked is a dark button and a muted colour goes
									    unreadable on it. */}
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

			{/* A field with nothing in it says nothing about itself, and there are
			    two of them on every row. The heading above the column is what
			    names them, which is why this is a table rather than a list. */}
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
						{/* Both halves are spelled out: what leaves the machine, and who
						    receives it. Agreeing to send something unnamed to somebody
						    unnamed is not agreeing. */}
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
