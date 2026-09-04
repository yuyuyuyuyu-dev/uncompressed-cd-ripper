import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { ExternalLink } from "lucide-react";
import { useEffect, useRef, useState } from "react";
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
import {
	Progress,
	ProgressLabel,
	ProgressValue,
} from "@/components/ui/progress";
import { describe } from "../error-report/error-report";

const RELEASES =
	"https://github.com/yuyuyuyuyu-dev/uncompressed-cd-ripper/releases";

type Downloading = {
	received: number;
	total?: number;
};

export function SelfUpdate() {
	const [update, setUpdate] = useState<Update>();
	const [downloading, setDownloading] = useState<Downloading>();
	const later = useRef<HTMLButtonElement>(null);
	const { t } = useTranslation();

	useEffect(() => {
		check()
			.then((found) => setUpdate(found ?? undefined))
			.catch((failure) => {
				const { type, value } = describe(failure);

				commands.logError(`${type}: ${value}`).catch(() => {});
			});
	}, []);

	const install = async () => {
		if (update === undefined) {
			return;
		}

		setDownloading({ received: 0 });

		await update.downloadAndInstall((step) => {
			if (step.event === "Started") {
				setDownloading({ received: 0, total: step.data.contentLength });
			}

			if (step.event === "Progress") {
				setDownloading((soFar) =>
					soFar === undefined
						? soFar
						: { ...soFar, received: soFar.received + step.data.chunkLength },
				);
			}
		});

		await relaunch();
	};

	return (
		<Dialog
			open={update !== undefined}
			onOpenChange={(open) => {
				if (!open && downloading === undefined) {
					setUpdate(undefined);
				}
			}}
		>
			<DialogContent
				className="flex max-h-[85vh] flex-col"
				initialFocus={later}
				showCloseButton={downloading === undefined}
			>
				<DialogHeader>
					<DialogTitle>{t("selfUpdate.title")}</DialogTitle>
					<DialogDescription className="whitespace-pre-line">
						{t("selfUpdate.body")}
					</DialogDescription>
				</DialogHeader>

				{downloading !== undefined && (
					<Progress
						value={
							downloading.total === undefined
								? null
								: (downloading.received / downloading.total) * 100
						}
					>
						<ProgressLabel>{t("selfUpdate.downloading")}</ProgressLabel>
						<ProgressValue />
					</Progress>
				)}

				<DialogFooter className="items-center">
					<Button
						variant="ghost"
						size="sm"
						className="mr-auto"
						onClick={() => openUrl(RELEASES)}
					>
						{t("selfUpdate.changes")}
						<ExternalLink />
					</Button>
					<Button
						ref={later}
						variant="outline"
						onClick={() => setUpdate(undefined)}
						disabled={downloading !== undefined}
					>
						{t("selfUpdate.later")}
					</Button>
					<Button onClick={install} disabled={downloading !== undefined}>
						{t("selfUpdate.update")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
