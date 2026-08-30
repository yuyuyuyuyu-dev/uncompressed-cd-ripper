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
	// Keyed by the identifier the toast manager hands back, which is what ties
	// a notification on screen to the report behind it.
	const [caught, setCaught] = useState(new Map<string, Caught>());
	const [environment, setEnvironment] = useState<Environment>();
	const [showingDetailOf, setShowingDetailOf] = useState<string>();
	const { t } = useTranslation();

	useEffect(() => {
		commands.environment().then(setEnvironment);
	}, []);

	// Wrapped so that the listeners below and the boundary around the children
	// are handing errors to one and the same place.
	const catchThrown = useCallback((thrown: unknown, componentStack = "") => {
		const { type, value } = describe(thrown);
		const occurredAt = new Date();

		// The log is what a machine nobody can reach leaves behind, and a
		// failure nobody sent a report about is the one thing worth having in
		// it from this side. Nothing waits on it.
		commands.logError(`${type}: ${value}`).catch(() => {
			// Nothing to do about it, and nothing worth telling the user.
		});

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

		// Asked for here rather than when the report is built, so that a report
		// says what the app was doing when it failed rather than what it went
		// on to do while the notification sat on screen.
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
			// Nothing was thrown here. The browser fires this when a
			// ResizeObserver callback keeps changing the layout it is watching
			// and it gives up delivering the rest of that frame's notifications.
			// The toasts measure themselves that way, so reporting it puts a
			// notification on screen, which moves the layout, which fires it
			// again: one of these arrives as half a dozen.
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

	// Rebuilt on every keystroke so that the text on screen is derived from the
	// same value that the send button hands over. Showing one and sending
	// another is the one way this screen could lie.
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
			{/* Nothing dismisses itself. An error that slid away on a timer is one
			    the user never got the chance to report, which is the whole point
			    of putting it on screen.

			    How many show at once is left at the default, which turned out to
			    be the number the animation stays smooth at. The rest wait behind,
			    and the staircase of their edges still says there are more. */}
			<Toaster timeout={0} />

			<RenderErrorBoundary onError={catchThrown}>
				{children}
			</RenderErrorBoundary>

			<Dialog
				open={showing !== undefined}
				onOpenChange={(open) => !open && setShowingDetailOf(undefined)}
			>
				{/* The report can run long, so the dialog keeps to the window and
				    scrolls rather than pushing the send button off screen. */}
				<DialogContent className="flex max-h-[85vh] flex-col overflow-y-auto sm:max-w-2xl">
					<DialogHeader>
						<DialogTitle>{t("errorReport.title")}</DialogTitle>
						<DialogDescription>{t("errorReport.body")}</DialogDescription>
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
									{/* Wrapped rather than scrolled sideways: a consent screen
									    the user has to drag around is one they will not read. */}
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
											// The one backend failure that must not travel the
											// usual path: reporting that the reporter failed
											// would ask to send a report whose sending is what
											// just failed.
											const result = await commands.sendErrorReport(report);

											if (result.status === "error") {
												update(showingDetailOf, { failedToSend: result.error });
												return;
											}

											dismiss(showingDetailOf);

											// The dialog closing on its own says nothing about
											// whether anything left the machine. Unlike an
											// error, this is a notice rather than something to
											// act on, so it is the one toast that sees itself
											// out.
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
