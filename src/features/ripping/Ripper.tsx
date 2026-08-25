import { Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
	isPermissionGranted,
	requestPermission,
	sendNotification,
} from "@tauri-apps/plugin-notification";
import { useCallback, useEffect, useState } from "react";
import {
	AGREEMENTS_REQUIRED,
	commands,
	READS_ALLOWED,
	type Track,
	type TrackProgress,
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
import {
	Progress,
	ProgressLabel,
	ProgressValue,
} from "@/components/ui/progress";
import { expectOk } from "../error-report/backend";
import { DiscMetadata } from "../metadata/DiscMetadata";
import { fileTitle, NOTHING, tagsFor } from "../metadata/metadata";

type Reading = {
	track: Track;
} & TrackProgress;

// A disc takes long enough that nobody is still watching. Permission is asked
// for here rather than at startup, when it is obvious what it is for.
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
	const [metadata, setMetadata] = useState(NOTHING);

	// Only drives holding a disc are listed, so an empty list is a machine with
	// nothing loaded rather than a failure. Which one is selected when several
	// hold a disc is undecided: it is whichever the library listed first.
	const look = useCallback(async () => {
		const found = await commands.drives();

		setDrives(found);
		setDrive(found[0]);
	}, []);

	useEffect(() => {
		look();
	}, [look]);

	useEffect(() => {
		// What the last disc was called says nothing about this one.
		setMetadata(NOTHING);

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
			// One at a time: the drive is no faster for being asked for several,
			// and a disc that fails halfway keeps the tracks before it.
			for (const track of tracks) {
				setReading({ track, read: 1, sectors: 0, matched: 0 });

				const progress = new Channel<TrackProgress>();
				progress.onmessage = ({ read, sectors, matched }) =>
					setReading({ track, read, sectors, matched });

				await expectOk(
					commands.ripTrack(
						drive,
						track.number,
						destination,
						tagsFor(metadata, track.number),
						progress,
					),
				);
			}

			// After every track, so a disc that gave up is not called finished.
			await announceTheEnd(tracks.length);
		} finally {
			setReading(undefined);
		}
	};

	// A track already sitting there took as long to read as its replacement.
	const start = async () => {
		if (destination === undefined) {
			return;
		}

		const existing = await commands.alreadyThere(
			destination,
			tracks.map((track) => ({
				number: track.number,
				title: fileTitle(metadata, track.number),
			})),
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
				{/* Until something notices a disc arriving, this is how one put in
				    after the window opened gets found. */}
				<Button variant="outline" size="sm" onClick={look} disabled={busy}>
					Scan for discs
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

			<DiscMetadata
				key={drive}
				drive={drive}
				tracks={tracks}
				metadata={metadata}
				onChange={setMetadata}
				disabled={busy}
			/>

			<div className="flex items-center gap-3">
				<Button
					variant="outline"
					onClick={async () => {
						const chosen = await open({ directory: true });

						// Null is closing the picker, which keeps the last choice.
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
				<div className="flex flex-col gap-1.5">
					<Progress
						// Out of what the disc says the track holds.
						value={Math.min(
							100,
							(reading.sectors / reading.track.sectors) * 100,
						)}
					>
						<ProgressLabel>
							Ripping track {String(reading.track.number).padStart(2, "0")} ·
							read {reading.read} (max {READS_ALLOWED})
						</ProgressLabel>
						<ProgressValue />
					</Progress>

					{/* Otherwise the same track goes by several times with nothing on
					    screen to say why. */}
					<p className="text-muted-foreground text-sm">
						The track is kept once {AGREEMENTS_REQUIRED} reads of it match.
						{/* One read has matched nothing, so the count stays away until
						    there is a match to report. */}
						{reading.matched > 1 && ` ${reading.matched} so far.`}
					</p>
				</div>
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

					{/* All of them: agreeing to replace files you were never shown is
					    not agreeing. Up to 99, hence the scrolling. */}
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
