import { LoaderCircle } from "lucide-react";
import { useEffect, useId, useState } from "react";
import { commands } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { expectOk } from "../error-report/backend";
import {
	saveChecking,
	savedChecking,
	savedOffset,
	saveOffset,
} from "./settings";

type Props = {
	drive: string | undefined;
	// How far along the drive reads, once AccurateRip has been asked about it,
	// and nothing until then. Held above rather than here, because ripping
	// needs it too.
	offset: number | undefined;
	onOffset: (offset: number | undefined) => void;
	checking: boolean;
	onChecking: (checking: boolean) => void;
	disabled: boolean;
};

export function Verification({
	drive,
	offset,
	onOffset,
	checking,
	onChecking,
	disabled,
}: Props) {
	const [asking, setAsking] = useState(false);
	const [looking, setLooking] = useState(false);
	// Told apart from never having asked, because only one of the two leaves
	// the user with nothing they can do.
	const [unlisted, setUnlisted] = useState(false);
	const label = useId();

	// What the app was left set to. Read once rather than per drive: this is a
	// decision about what leaves the machine, and the answer to it does not
	// change with which drive the disc went into.
	useEffect(() => {
		savedChecking().then(onChecking);
	}, [onChecking]);

	// What was found for this drive the last time the app ran. A read offset
	// belongs to the drive, so this follows whichever one is chosen.
	useEffect(() => {
		setUnlisted(false);

		if (drive === undefined) {
			return;
		}

		let wanted = true;

		(async () => {
			const name = await expectOk(commands.driveName(drive));
			const frames = await savedOffset(name);

			// A different drive was picked while this was on its way, and what it
			// found belongs to the drive that has gone.
			if (wanted && frames !== undefined) {
				onOffset(frames);
			}
		})();

		return () => {
			wanted = false;
		};
	}, [drive, onOffset]);

	// Both halves of what was agreed to, in the order they happen. The drive's
	// read offset comes first because nothing can be compared without it, and
	// it is looked up here rather than somewhere else on the screen: it is
	// what turning this on needs, not a thing anybody would go and do.
	//
	// The dialog stays open while this runs, with the button that started it
	// turning: what is being waited for is a list coming over the network, and
	// a dialog that shut first would leave the wait with nothing to show for
	// itself.
	const turnOn = async () => {
		if (drive === undefined) {
			return;
		}

		setLooking(true);

		try {
			const name = await expectOk(commands.driveName(drive));
			const frames = await expectOk(commands.readOffset(drive));

			setUnlisted(frames === null);

			// Left off rather than on and useless. A rip read without the offset
			// matches nothing, and saying it was checked would be saying it
			// failed.
			if (frames === null) {
				return;
			}

			await saveOffset(name, frames);
			onOffset(frames);
			onChecking(true);
			await saveChecking(true);
		} finally {
			setLooking(false);
			setAsking(false);
		}
	};

	const turnOff = () => {
		onChecking(false);
		saveChecking(false);
	};

	// On only where there is also something to compare with. The app can be
	// left switched on and then have a drive AccurateRip knows nothing about
	// put in front of it.
	const on = checking && offset !== undefined;

	const said = () => {
		if (unlisted) {
			return "AccurateRip has never been told about this drive, so tracks read from it cannot be checked against anybody else's rips.";
		}

		if (on) {
			return "This drive's read offset is set. Every track read from it is put right by that much.";
		}

		return undefined;
	};

	return (
		<div className="flex flex-col gap-1.5">
			<div className="flex items-center gap-2">
				<Switch
					id={label}
					checked={on}
					onCheckedChange={(wanted) => (wanted ? setAsking(true) : turnOff())}
					disabled={disabled || looking || drive === undefined}
				/>
				{/* The same word the column of numbers is headed with, so that
				    what this switch turns on and what turns up in that column are
				    plainly the same thing. */}
				<label className="text-sm" htmlFor={label}>
					Check the ripped tracks against other people's submissions
					(AccurateRip)
				</label>
			</div>

			{said() !== undefined && (
				<p className="text-muted-foreground text-sm">{said()}</p>
			)}

			<Dialog
				open={asking}
				// Not while the list is on its way. Closing it would leave the
				// request running with nothing on screen saying so.
				onOpenChange={(open) => !open && !looking && setAsking(false)}
			>
				<DialogContent>
					<DialogHeader>
						{/* Padded clear of the button that closes the dialog, which sits
						    over the top right corner. */}
						<DialogTitle className="pr-8">
							Check this rip against other people's submissions?
						</DialogTitle>
						{/* Both halves are spelled out: what leaves the machine, and who
						    receives it. Agreeing to send something unnamed to somebody
						    unnamed is not agreeing. */}
						<DialogDescription>
							AccurateRip is a list of what other people's drives made of the
							same discs: one submission for every rip anybody has ever sent in.
							Turning this on does two things. Every drive reads a little ahead
							of or behind where the disc says, so AccurateRip's list of the
							drives it knows about is downloaded and searched on this machine
							to find how far out this one is; nothing about your drive is sent.
							Then, once a disc has been read, a fingerprint of it, worked out
							from where its tracks begin, is sent to AccurateRip, and
							everything AccurateRip holds about that disc comes back. Nothing
							of the rip itself goes out: what came off the disc is held against
							what came back here, on this machine. The address your machine
							reaches the internet from goes with both requests, as it does with
							any request. None of your music is sent, and nothing is sent while
							this is off.
						</DialogDescription>
					</DialogHeader>

					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => setAsking(false)}
							disabled={looking}
						>
							Cancel
						</Button>
						<Button onClick={turnOn} disabled={looking}>
							{looking && <LoaderCircle className="animate-spin" />}
							Turn it on
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
