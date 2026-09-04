import { Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
	isPermissionGranted,
	requestPermission,
	sendNotification,
} from "@tauri-apps/plugin-notification";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	AGREEMENTS_REQUIRED,
	type Checksums,
	commands,
	events,
	READS_ALLOWED,
	type Track,
	type TrackProgress,
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
import {
	Progress,
	ProgressLabel,
	ProgressValue,
} from "@/components/ui/progress";
import { expectOk } from "../error-report/backend";
import i18next from "../language/i18n";
import { DiscMetadata } from "../metadata/DiscMetadata";
import { fileTitle, NOTHING, tagsFor } from "../metadata/metadata";
import { Verification } from "../verification/Verification";

type Reading = {
	track: Track;
} & TrackProgress;

async function announceTheEnd(tracks: number) {
	const granted =
		(await isPermissionGranted()) || (await requestPermission()) === "granted";

	if (!granted) {
		return;
	}

	sendNotification({
		title: i18next.t("ripping.notification.title"),
		body: i18next.t("ripping.notification.body", { count: tracks }),
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
	const [offset, setOffset] = useState<number>();
	const [checking, setChecking] = useState(false);
	const [verdicts, setVerdicts] = useState<Verdict[]>();
	const { t } = useTranslation();

	const look = useCallback(async () => {
		const found = await commands.drives();

		setDrives(found);
		setDrive(found[0]);
	}, []);

	useEffect(() => {
		look();
	}, [look]);

	useEffect(() => {
		const listening = events.drivesChanged.listen(({ payload }) => {
			setDrives(payload);
			setDrive((chosen) =>
				chosen !== undefined && payload.includes(chosen) ? chosen : payload[0],
			);
		});

		return () => {
			listening.then((stop) => stop());
		};
	}, []);

	useEffect(() => {
		setMetadata(NOTHING);
		setOffset(undefined);
		setVerdicts(undefined);

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

		setVerdicts(undefined);

		try {
			const checksums: Checksums[] = [];

			for (const track of tracks) {
				setReading({ track, read: 1, sectors: 0, matched: 0 });

				const progress = new Channel<TrackProgress>();
				progress.onmessage = ({ read, sectors, matched }) =>
					setReading({ track, read, sectors, matched });

				const ripped = await expectOk(
					commands.ripTrack(
						drive,
						track.number,
						destination,
						tagsFor(metadata, track.number),
						offset ?? 0,
						progress,
					),
				);

				checksums.push(ripped.checksums);
			}

			await announceTheEnd(tracks.length);

			if (checking && offset !== undefined) {
				setVerdicts(await expectOk(commands.checkRip(drive, checksums)));
			}
		} finally {
			setReading(undefined);
		}
	};

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
				<h2 className="font-semibold">{t("ripping.heading")}</h2>
				<Button variant="outline" size="sm" onClick={look} disabled={busy}>
					{t("ripping.scan")}
				</Button>
			</div>

			{drives.length === 0 ? (
				<p className="text-muted-foreground text-sm">{t("ripping.noDrive")}</p>
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
				verdicts={verdicts}
				onChange={setMetadata}
				disabled={busy}
			/>

			<div className="flex items-center gap-3">
				<Button
					variant="outline"
					onClick={async () => {
						const chosen = await open({ directory: true });

						if (chosen !== null) {
							setDestination(chosen);
						}
					}}
					disabled={busy}
				>
					{t("ripping.chooseFolder")}
				</Button>

				<span className="truncate text-muted-foreground text-sm">
					{destination ?? t("ripping.noFolder")}
				</span>
			</div>

			<Verification
				drive={drive}
				offset={offset}
				onOffset={setOffset}
				checking={checking}
				onChecking={setChecking}
				disabled={busy}
			/>

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
					{t("ripping.rip")}
				</Button>
			</div>

			{reading !== undefined && (
				<div className="flex flex-col gap-1.5">
					<Progress
						value={Math.min(
							100,
							(reading.sectors / reading.track.sectors) * 100,
						)}
					>
						<ProgressLabel>
							{t("ripping.progress", {
								number: String(reading.track.number).padStart(2, "0"),
								read: reading.read,
								max: READS_ALLOWED,
							})}
						</ProgressLabel>
						<ProgressValue />
					</Progress>

					<p className="text-muted-foreground text-sm">
						{t("ripping.agreement", {
							required: AGREEMENTS_REQUIRED,
							remaining: AGREEMENTS_REQUIRED - reading.matched,
						})}
					</p>
				</div>
			)}

			<Dialog
				open={overwriting !== undefined}
				onOpenChange={(open) => !open && setOverwriting(undefined)}
			>
				<DialogContent className="flex max-h-[85vh] flex-col">
					<DialogHeader>
						<DialogTitle>{t("ripping.overwrite.title")}</DialogTitle>
						<DialogDescription className="whitespace-pre-line">
							{t("ripping.overwrite.body", {
								count: overwriting?.length ?? 0,
							})}
						</DialogDescription>
					</DialogHeader>

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
							{t("cancel")}
						</Button>
						<Button
							onClick={async () => {
								setOverwriting(undefined);
								await rip();
							}}
						>
							{t("ripping.overwrite.confirm")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</section>
	);
}
