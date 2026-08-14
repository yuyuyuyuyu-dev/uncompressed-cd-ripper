import { Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
	isPermissionGranted,
	requestPermission,
	sendNotification,
} from "@tauri-apps/plugin-notification";
import { useCallback, useEffect, useState } from "react";
import { commands, type Track } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import {
	Progress,
	ProgressLabel,
	ProgressValue,
} from "@/components/ui/progress";
import { expectOk } from "../error-report/backend";

// A CD is addressed in sectors, 75 of which make a second.
const SECTORS_PER_SECOND = 75;

function length(sectors: number) {
	const seconds = Math.round(sectors / SECTORS_PER_SECOND);

	return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

type Reading = {
	track: Track;
	sectors: number;
};

// A disc takes long enough that the user is unlikely to still be watching, so
// the end of it is said through the system rather than only on screen. Asking
// for permission at this point rather than at startup means the request
// arrives when it is obvious what it is for.
async function announceTheEnd(tracks: number) {
	const granted =
		(await isPermissionGranted()) || (await requestPermission()) === "granted";

	if (!granted) {
		return;
	}

	sendNotification({
		title: "Ripping finished",
		body: `${tracks} ${tracks === 1 ? "track is" : "tracks are"} in the folder you chose.`,
	});
}

export function Ripper() {
	const [drives, setDrives] = useState<string[]>([]);
	const [drive, setDrive] = useState<string>();
	const [tracks, setTracks] = useState<Track[]>([]);
	const [destination, setDestination] = useState<string>();
	const [reading, setReading] = useState<Reading>();
	const [overwriting, setOverwriting] = useState<string[]>();

	// Only drives with an audio CD in them are listed, so an empty list is the
	// ordinary state of a machine with nothing loaded rather than a failure.
	const look = useCallback(async () => {
		const found = await commands.drives();

		setDrives(found);
		setDrive(found[0]);
	}, []);

	useEffect(() => {
		look();
	}, [look]);

	useEffect(() => {
		if (drive === undefined) {
			setTracks([]);
			return;
		}

		expectOk(commands.tracks(drive)).then(setTracks);
	}, [drive]);

	const rip = async () => {
		if (drive === undefined || destination === undefined) {
			return;
		}

		try {
			// One track at a time, and counted as they land: the drive reads no
			// faster for being asked for several at once, and a disc that fails
			// halfway leaves the tracks before it already written.
			for (const track of tracks) {
				setReading({ track, sectors: 0 });

				const progress = new Channel<number>();
				progress.onmessage = (sectors) => setReading({ track, sectors });

				await expectOk(
					commands.ripTrack(drive, track.number, destination, progress),
				);
			}

			// Only once every track is written, so that a disc that gave up
			// halfway is not announced as finished.
			await announceTheEnd(tracks.length);
		} finally {
			setReading(undefined);
		}
	};

	// Asking first, because a track already sitting there took as long to read
	// as the one about to replace it.
	const start = async () => {
		if (destination === undefined) {
			return;
		}

		const existing = await commands.alreadyThere(
			destination,
			tracks.map((track) => track.number),
		);

		if (existing.length > 0) {
			setOverwriting(existing);
			return;
		}

		await rip();
	};

	const busy = reading !== undefined;

	return (
		<section className="flex w-full max-w-xl flex-col gap-4 text-left">
			<div className="flex items-center gap-2">
				<h2 className="font-semibold">Disc</h2>
				<Button variant="outline" size="sm" onClick={look} disabled={busy}>
					Look again
				</Button>
			</div>

			{drives.length === 0 ? (
				<p className="text-muted-foreground text-sm">
					No drive with an audio CD in it.
				</p>
			) : (
				<div className="flex flex-wrap gap-2">
					{drives.map((found) => (
						<Button
							key={found}
							variant={found === drive ? "default" : "outline"}
							size="sm"
							onClick={() => setDrive(found)}
							disabled={busy}
						>
							{found}
						</Button>
					))}
				</div>
			)}

			{tracks.length > 0 && (
				<ol className="rounded-lg border">
					{tracks.map((track) => (
						<li
							key={track.number}
							className="flex justify-between border-b px-3 py-1.5 text-sm last:border-b-0"
						>
							<span>{String(track.number).padStart(2, "0")}</span>
							<span className="text-muted-foreground tabular-nums">
								{length(track.sectors)}
							</span>
						</li>
					))}
				</ol>
			)}

			<div className="flex items-center gap-3">
				<Button
					variant="outline"
					onClick={async () => {
						const chosen = await open({ directory: true });

						// Null is the user closing the picker without choosing,
						// which leaves whatever was chosen before in place.
						if (chosen !== null) {
							setDestination(chosen);
						}
					}}
					disabled={busy}
				>
					Choose a folder
				</Button>

				<span className="truncate text-muted-foreground text-sm">
					{destination ?? "No folder chosen"}
				</span>
			</div>

			<div className="flex items-center gap-3">
				<Button
					onClick={start}
					disabled={
						busy ||
						drive === undefined ||
						destination === undefined ||
						tracks.length === 0
					}
				>
					Rip
				</Button>
			</div>

			{reading !== undefined && (
				<Progress
					// Out of the sectors the disc says the track has, which is
					// what the drive is working through.
					value={Math.min(100, (reading.sectors / reading.track.sectors) * 100)}
				>
					<ProgressLabel>
						Ripping track {String(reading.track.number).padStart(2, "0")}
					</ProgressLabel>
					<ProgressValue />
				</Progress>
			)}

			<Dialog
				open={overwriting !== undefined}
				onOpenChange={(open) => !open && setOverwriting(undefined)}
			>
				<DialogContent className="flex max-h-[85vh] flex-col">
					<DialogHeader>
						<DialogTitle>Overwrite what is already there?</DialogTitle>
						<DialogDescription>
							{overwriting?.length === 1
								? "This file is already in that folder and ripping would replace it."
								: `These ${overwriting?.length} files are already in that folder and ripping would replace them.`}
						</DialogDescription>
					</DialogHeader>

					{/* Every one of them, however many there are: a list that stops
					    short leaves the user agreeing to replace files they were
					    never shown. A disc holds up to 99 tracks, so the list
					    scrolls rather than pushing the buttons off screen. */}
					<ul className="min-h-0 overflow-y-auto rounded-lg border">
						{overwriting?.map((name) => (
							<li
								key={name}
								className="border-b px-3 py-1.5 text-sm last:border-b-0"
							>
								{name}
							</li>
						))}
					</ul>

					<DialogFooter>
						<Button variant="outline" onClick={() => setOverwriting(undefined)}>
							Cancel
						</Button>
						<Button
							onClick={async () => {
								setOverwriting(undefined);
								await rip();
							}}
						>
							Overwrite
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</section>
	);
}
