import { LoaderCircle } from "lucide-react";
import { useId, useState } from "react";
import { type Album, commands } from "@/bindings";
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
import { expectOk } from "../error-report/backend";
import { fromAlbum, type Metadata, withAlbum, withArtist } from "./metadata";

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
	metadata: Metadata;
	onChange: (metadata: Metadata) => void;
	disabled: boolean;
};

export function DiscMetadata({ drive, metadata, onChange, disabled }: Props) {
	// A different disc is a different question. None of this survives one being
	// put in, which is arranged by the drive being this component's key rather
	// than by anything here.
	const [asking, setAsking] = useState(false);
	const [matches, setMatches] = useState<Album[]>();
	const [chosen, setChosen] = useState<string>();
	const [looking, setLooking] = useState(false);

	const albumField = useId();
	const artistField = useId();

	const take = (album: Album) => {
		setChosen(album.id);
		onChange(fromAlbum(album));
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

	return (
		<div className="flex flex-col gap-3">
			<div className="flex items-center gap-2">
				<h2 className="font-semibold">Metadata</h2>
				<Button
					variant="outline"
					size="sm"
					onClick={() => setAsking(true)}
					disabled={disabled || looking || drive === undefined}
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
								disabled={disabled}
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

			{/* The column the labels sit in is as wide as the one holding the track
			    numbers, so that every field on the screen starts in one place. */}
			<div className="grid grid-cols-[3rem_1fr] items-center gap-x-3 gap-y-2">
				<label className="text-sm" htmlFor={albumField}>
					Album
				</label>
				<Input
					id={albumField}
					value={metadata.album}
					onChange={(event) =>
						onChange(withAlbum(metadata, event.target.value))
					}
					disabled={disabled}
				/>

				<label className="text-sm" htmlFor={artistField}>
					Artist
				</label>
				<Input
					id={artistField}
					value={metadata.artist}
					onChange={(event) =>
						onChange(withArtist(metadata, event.target.value))
					}
					disabled={disabled}
				/>
			</div>

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
							MusicBrainz. The address your machine reaches the internet from
							goes with it, as it does with any request. Nothing else about you
							is sent, and nothing is sent at all unless you say so.
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
