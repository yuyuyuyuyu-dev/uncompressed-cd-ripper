import { useState } from "react";
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
import { expectOk } from "../error-report/backend";

// What tells one pressing from another when the record is the same: two of
// them can agree on the title and the artist and differ only here.
function pressing(album: Album) {
	return [album.released, album.country].filter(Boolean).join(" · ");
}

type Props = {
	drive: string | undefined;
	chosen: Album | undefined;
	onChosen: (album: Album | undefined) => void;
	disabled: boolean;
};

export function DiscMetadata({ drive, chosen, onChosen, disabled }: Props) {
	// A different disc is a different question. None of this survives one being
	// put in, which is arranged by the drive being this component's key rather
	// than by anything here.
	const [asking, setAsking] = useState(false);
	const [matches, setMatches] = useState<Album[]>();
	const [looking, setLooking] = useState(false);

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
				onChosen(found[0]);
			}
		} finally {
			setLooking(false);
		}
	};

	return (
		<div className="flex flex-col gap-2">
			<div className="flex items-center gap-2">
				<h2 className="font-semibold">Metadata</h2>
				<Button
					variant="outline"
					size="sm"
					onClick={() => setAsking(true)}
					disabled={disabled || looking || drive === undefined}
				>
					Look this disc up
				</Button>
			</div>

			{chosen !== undefined ? (
				<p className="flex gap-1 text-sm">
					<span className="font-medium">{chosen.title}</span>
					<span className="text-muted-foreground">—</span>
					<span className="text-muted-foreground">{chosen.artist}</span>
				</p>
			) : (
				<p className="text-muted-foreground text-sm">
					{looking
						? "Looking this disc up…"
						: matches?.length === 0
							? "This disc is not in the database."
							: matches === undefined
								? "This disc has not been looked up."
								: "Choose which of these the disc is."}
				</p>
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
								variant={match.id === chosen?.id ? "default" : "outline"}
								size="sm"
								className="h-auto w-full justify-start py-1.5 text-left"
								onClick={() => onChosen(match)}
							>
								<span className="truncate">
									{match.title} — {match.artist}
									{pressing(match) !== "" && (
										<span className="text-muted-foreground">
											{" "}
											({pressing(match)})
										</span>
									)}
								</span>
							</Button>
						</li>
					))}
				</ul>
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
