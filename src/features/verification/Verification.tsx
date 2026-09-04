import { LoaderCircle } from "lucide-react";
import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
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
import "../language/i18n";
import {
	saveChecking,
	savedChecking,
	savedOffset,
	saveOffset,
} from "./settings";

type Props = {
	drive: string | undefined;
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
	const [unlisted, setUnlisted] = useState(false);
	const label = useId();
	const { t } = useTranslation();

	useEffect(() => {
		savedChecking().then(onChecking);
	}, [onChecking]);

	useEffect(() => {
		setUnlisted(false);

		if (drive === undefined) {
			return;
		}

		let wanted = true;

		(async () => {
			const name = await expectOk(commands.driveName(drive));
			const frames = await savedOffset(name);

			if (wanted && frames !== undefined) {
				onOffset(frames);
			}
		})();

		return () => {
			wanted = false;
		};
	}, [drive, onOffset]);

	const turnOn = async () => {
		if (drive === undefined) {
			return;
		}

		setLooking(true);

		try {
			const name = await expectOk(commands.driveName(drive));
			const frames = await expectOk(commands.readOffset(drive));

			setUnlisted(frames === null);

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

	const on = checking && offset !== undefined;

	return (
		<div className="flex flex-col gap-1.5">
			<div className="flex items-center gap-2">
				<Switch
					id={label}
					checked={on}
					onCheckedChange={(wanted) => (wanted ? setAsking(true) : turnOff())}
					disabled={disabled || looking || drive === undefined}
				/>
				<label className="text-sm" htmlFor={label}>
					{t("verification.label")}
				</label>
			</div>

			{unlisted && (
				<p className="text-muted-foreground text-sm">
					{t("verification.unlisted")}
				</p>
			)}

			<Dialog
				open={asking}
				onOpenChange={(open) => !open && !looking && setAsking(false)}
			>
				<DialogContent className="flex max-h-[85vh] flex-col">
					<DialogHeader className="min-h-0">
						<DialogTitle className="pr-8">
							{t("verification.ask.title")}
						</DialogTitle>
						<DialogDescription className="-mr-4 min-h-0 overflow-y-auto pr-4 whitespace-pre-line">
							{t("verification.ask.body")}
						</DialogDescription>
					</DialogHeader>

					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => setAsking(false)}
							disabled={looking}
						>
							{t("cancel")}
						</Button>
						<Button onClick={turnOn} disabled={looking}>
							<LoaderCircle
								className={looking ? "animate-spin" : "invisible"}
							/>
							{t("verification.ask.confirm")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
