import { LoaderCircle } from "lucide-react";
import {
	type Dispatch,
	type SetStateAction,
	useId,
	useRef,
	useState,
} from "react";
import { type Album, commands, type Track } from "@/bindings";
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
import { CoverArt } from "../artwork/CoverArt";
import { expectOk } from "../error-report/backend";
import { length } from "../ripping/track";
import {
	artistOf,
	fromAlbum,
	type Metadata,
	titleOf,
	withAlbum,
	withAlbumArtist,
	withArtist,
	withCover,
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
	if (looking) {
		return "Waiting for an answer about this disc.";
	}

	if (matches === undefined) {
		return undefined;
	}

	if (matches.length === 0) {
		return "This disc is not in the database. Its metadata can still be typed in.";
	}

	return matches.length > 1 ? "Choose which of these the disc is." : undefined;
}

type Props = {
	drive: string | undefined;
	tracks: Track[];
	metadata: Metadata;
	// Taking what to change it into rather than what to change it to, because
	// the sleeve lands after the fields it arrives beside are already there to
	// be typed in.
	onChange: Dispatch<SetStateAction<Metadata>>;
	disabled: boolean;
};

export function DiscMetadata({
	drive,
	tracks,
	metadata,
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
	const [fetchingCover, setFetchingCover] = useState(false);

	// Which request the sleeve on its way belongs to. Clicking through the
	// matches starts one for each, and an earlier answer arriving late would
	// otherwise settle on the album that was clicked after it.
	const asked = useRef(0);

	const albumField = useId();
	const albumArtistField = useId();

	const takeCover = async (release: string) => {
		const mine = asked.current + 1;
		asked.current = mine;

		setFetchingCover(true);

		try {
			const cover = await expectOk(commands.lookUpArtwork(release));

			if (asked.current === mine) {
				onChange((current) => withCover(current, cover));
			}
		} finally {
			if (asked.current === mine) {
				setFetchingCover(false);
			}
		}
	};

	// Not waited for: the album and the track titles are there to be read and
	// corrected while the sleeve is still coming.
	const take = (album: Album) => {
		setChosen(album.id);
		onChange(fromAlbum(album));
		takeCover(album.id);
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
				<h2 className="font-semibold">Metadata</h2>
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
					{looking ? "Looking it up…" : "Look this disc up"}
				</Button>
			</div>

			{said !== undefined && (
				<p className="text-muted-foreground text-sm">{said}</p>
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
				<CoverArt cover={metadata.cover} looking={looking || fetchingCover} />

				<div className="grid flex-1 grid-cols-[auto_1fr] items-center gap-x-3 gap-y-2">
					<label className="text-sm" htmlFor={albumField}>
						Album
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
						Album artist
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
								Title
							</th>
							<th className="px-2 pb-1.5 text-left font-medium">Artist</th>
							<th className="pb-1.5 text-right font-medium">Length</th>
						</tr>
					</thead>
					<tbody>
						{tracks.map((track) => (
							<tr key={track.number}>
								<td className="py-1.5 text-sm tabular-nums">
									{String(track.number).padStart(2, "0")}
								</td>
								<td className="px-2 py-1.5">
									<Input
										aria-label={`Title of track ${track.number}`}
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
										aria-label={`Artist of track ${track.number}`}
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
							</tr>
						))}
					</tbody>
				</table>
			)}

			<Dialog open={asking} onOpenChange={(open) => !open && setAsking(false)}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Look this disc up?</DialogTitle>
						{/* Both halves are spelled out: what leaves the machine, and who
						    receives it. Agreeing to send something unnamed to somebody
						    unnamed is not agreeing. */}
						<DialogDescription>
							Finding the album and the track titles means sending a fingerprint
							of this disc, worked out from where its tracks begin, to
							MusicBrainz. The sleeve is then asked for from the Cover Art
							Archive, which is sent the identifier of whichever release matched
							and serves the picture from the Internet Archive. The address your
							machine reaches the internet from goes with both, as it does with
							any request. Nothing else about you is sent, and nothing is sent
							at all unless you say so.
						</DialogDescription>
					</DialogHeader>

					<DialogFooter>
						<Button variant="outline" onClick={() => setAsking(false)}>
							Cancel
						</Button>
						<Button onClick={look}>Look it up</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
