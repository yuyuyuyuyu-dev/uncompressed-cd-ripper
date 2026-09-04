import { type ReactNode, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { type Breadcrumb, commands, type Environment } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Toaster, toast } from "@/components/ui/toast";
import i18next from "../language/i18n";
import { buildErrorReport, describe, newEventId } from "./error-report";
import { RenderErrorBoundary } from "./RenderErrorBoundary";

type Caught = {
	eventId: string;
	thrown: unknown;
	componentStack: string;
	occurredAt: Date;
	comment: string;
	breadcrumbs: Breadcrumb[];
	failedToSend?: string;
};

export function ErrorReporter({ children }: { children?: ReactNode }) {
	const [caught, setCaught] = useState(new Map<string, Caught>());
	const [environment, setEnvironment] = useState<Environment>();
	const [showingDetailOf, setShowingDetailOf] = useState<string>();
	const { t } = useTranslation();

	useEffect(() => {
		commands.environment().then(setEnvironment);
	}, []);

	const catchThrown = useCallback((thrown: unknown, componentStack = "") => {
		const { type, value } = describe(thrown);
		const occurredAt = new Date();

		commands.logError(`${type}: ${value}`).catch(() => {});

		const id = toast.add({
			type: "error",
			title: type,
			description: value,
			actionProps: {
				children: i18next.t("errorReport.details"),
				onClick: () => setShowingDetailOf(id),
			},
		});

		setCaught((caught) =>
			new Map(caught).set(id, {
				eventId: newEventId(),
				thrown,
				componentStack,
				occurredAt,
				comment: "",
				breadcrumbs: [],
			}),
		);

		commands.breadcrumbs().then((breadcrumbs) =>
			setCaught((caught) => {
				const one = caught.get(id);

				return one === undefined
					? caught
					: new Map(caught).set(id, { ...one, breadcrumbs });
			}),
		);
	}, []);

	useEffect(() => {
		const onError = (event: ErrorEvent) => {
			if (!event.error && event.message.startsWith("ResizeObserver loop")) {
				return;
			}

			catchThrown(event.error ?? event.message);
		};
		const onRejection = (event: PromiseRejectionEvent) =>
			catchThrown(event.reason);

		window.addEventListener("error", onError);
		window.addEventListener("unhandledrejection", onRejection);

		return () => {
			window.removeEventListener("error", onError);
			window.removeEventListener("unhandledrejection", onRejection);
		};
	}, [catchThrown]);

	const update = (id: string, change: Partial<Caught>) =>
		setCaught((caught) => {
			const one = caught.get(id);

			return one === undefined
				? caught
				: new Map(caught).set(id, { ...one, ...change });
		});

	const dismiss = (id: string) => {
		toast.close(id);
		setCaught((caught) => {
			const without = new Map(caught);
			without.delete(id);

			return without;
		});
		setShowingDetailOf(undefined);
	};

	const showing =
		showingDetailOf === undefined ? undefined : caught.get(showingDetailOf);

	const report =
		showing === undefined || environment === undefined
			? undefined
			: buildErrorReport({
					eventId: showing.eventId,
					thrown: showing.thrown,
					componentStack: showing.componentStack,
					environment,
					occurredAt: showing.occurredAt,
					comment: showing.comment,
					breadcrumbs: showing.breadcrumbs,
				});

	return (
		<>
			<Toaster timeout={0} />

			<RenderErrorBoundary onError={catchThrown}>
				{children}
			</RenderErrorBoundary>

			<Dialog
				open={showing !== undefined}
				onOpenChange={(open) => !open && setShowingDetailOf(undefined)}
			>
				<DialogContent className="flex max-h-[85vh] flex-col overflow-y-auto sm:max-w-2xl">
					<DialogHeader>
						<DialogTitle>{t("errorReport.title")}</DialogTitle>
						<DialogDescription className="whitespace-pre-line">
							{t("errorReport.body")}
						</DialogDescription>
					</DialogHeader>

					{showing !== undefined &&
						showingDetailOf !== undefined &&
						report !== undefined && (
							<>
								<Textarea
									aria-label={t("errorReport.commentLabel")}
									placeholder={t("errorReport.commentPlaceholder")}
									value={showing.comment}
									onChange={(event) =>
										update(showingDetailOf, {
											comment: event.currentTarget.value,
										})
									}
								/>

								<section
									aria-label={t("errorReport.reportLabel")}
									className="max-h-72 overflow-auto rounded-lg border bg-muted p-3"
								>
									<pre className="whitespace-pre-wrap break-all text-xs">
										{JSON.stringify(report, null, 2)}
									</pre>
								</section>

								<DialogFooter className="items-center gap-3">
									{showing.failedToSend !== undefined && (
										<p className="mr-auto text-destructive text-sm">
											{showing.failedToSend}
										</p>
									)}
									<Button
										onClick={async () => {
											const result = await commands.sendErrorReport(report);

											if (result.status === "error") {
												update(showingDetailOf, { failedToSend: result.error });
												return;
											}

											dismiss(showingDetailOf);

											toast.add({
												type: "success",
												title: t("errorReport.sentTitle"),
												description: t("errorReport.sentBody"),
												timeout: 5000,
											});
										}}
									>
										{t("errorReport.send")}
									</Button>
								</DialogFooter>
							</>
						)}
				</DialogContent>
			</Dialog>
		</>
	);
}
