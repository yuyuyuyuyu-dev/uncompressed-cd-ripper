import { useEffect, useRef, useState } from "react";
import { commands, type Environment } from "@/bindings";
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
import { buildErrorReport, newEventId } from "./error-report";

type Caught = {
	id: number;
	eventId: string;
	thrown: unknown;
	occurredAt: Date;
	comment: string;
	failedToSend?: string;
};

export function ErrorReporter() {
	// Every error keeps its own notification rather than replacing the one
	// before it. A disc whose tracks fail one after another should look like
	// several problems, because that is what it is.
	const [caught, setCaught] = useState<Caught[]>([]);
	const [environment, setEnvironment] = useState<Environment>();
	const [showingDetailOf, setShowingDetailOf] = useState<number>();
	const nextId = useRef(0);

	useEffect(() => {
		commands.environment().then(setEnvironment);
	}, []);

	useEffect(() => {
		const catchThrown = (thrown: unknown) => {
			// Read outside the update rather than inside it. React runs these in
			// a batch, by which time the counter has moved on for every one of
			// them, and they would all end up sharing the last number.
			nextId.current += 1;
			const id = nextId.current;

			setCaught((caught) => [
				...caught,
				{
					id,
					eventId: newEventId(),
					thrown,
					occurredAt: new Date(),
					comment: "",
				},
			]);
		};
		const onError = (event: ErrorEvent) =>
			catchThrown(event.error ?? event.message);
		const onRejection = (event: PromiseRejectionEvent) =>
			catchThrown(event.reason);

		window.addEventListener("error", onError);
		window.addEventListener("unhandledrejection", onRejection);

		return () => {
			window.removeEventListener("error", onError);
			window.removeEventListener("unhandledrejection", onRejection);
		};
	}, []);

	if (environment === undefined) {
		return null;
	}

	const update = (id: number, change: Partial<Caught>) =>
		setCaught((caught) =>
			caught.map((one) => (one.id === id ? { ...one, ...change } : one)),
		);

	const dismiss = (id: number) => {
		setCaught((caught) => caught.filter((one) => one.id !== id));
		setShowingDetailOf(undefined);
	};

	// Rebuilt on every keystroke so that the text on screen is derived from the
	// same value that the send button hands over. Showing one and sending
	// another is the one way this screen could lie.
	const reportFor = (one: Caught) =>
		buildErrorReport({
			eventId: one.eventId,
			thrown: one.thrown,
			environment,
			occurredAt: one.occurredAt,
			comment: one.comment,
		});

	const showing = caught.find((one) => one.id === showingDetailOf);

	return (
		<>
			<div className="fixed inset-x-0 bottom-0 z-40 flex flex-col">
				{caught.map((one) => (
					<div
						key={one.id}
						role="alert"
						// Hidden while its own detail screen is up, where it would
						// otherwise sit on top of the very dialog it opened.
						hidden={showingDetailOf !== undefined}
						className="flex items-center gap-3 border-t bg-card p-4 text-left text-card-foreground"
					>
						<p className="min-w-0 flex-1 truncate text-sm">
							{reportFor(one).exception.values[0].value}
						</p>
						<Button
							variant="outline"
							onClick={() => setShowingDetailOf(one.id)}
						>
							Details
						</Button>
						<Button variant="ghost" onClick={() => dismiss(one.id)}>
							Dismiss
						</Button>
					</div>
				))}
			</div>

			<Dialog
				open={showing !== undefined}
				onOpenChange={(open) => !open && setShowingDetailOf(undefined)}
			>
				{/* The report can run long, so the dialog keeps to the window and
				    scrolls rather than pushing the send button off screen. */}
				<DialogContent className="flex max-h-[85vh] flex-col overflow-y-auto sm:max-w-2xl">
					<DialogHeader>
						<DialogTitle>Send this error report?</DialogTitle>
						<DialogDescription>
							Nothing leaves this machine unless you send it. Below is the
							report exactly as it would be sent.
						</DialogDescription>
					</DialogHeader>

					{showing !== undefined && (
						<>
							<Textarea
								aria-label="What were you doing?"
								placeholder="What were you doing when this happened?"
								value={showing.comment}
								onChange={(event) =>
									update(showing.id, { comment: event.currentTarget.value })
								}
							/>

							<section
								aria-label="The error report"
								className="max-h-72 overflow-auto rounded-lg border bg-muted p-3"
							>
								{/* Wrapped rather than scrolled sideways: a consent screen
								    the user has to drag around is one they will not read. */}
								<pre className="whitespace-pre-wrap break-all text-xs">
									{JSON.stringify(reportFor(showing), null, 2)}
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
										const result = await commands.sendErrorReport(
											reportFor(showing),
										);

										if (result.status === "error") {
											update(showing.id, { failedToSend: result.error });
											return;
										}

										dismiss(showing.id);
									}}
								>
									Send
								</Button>
							</DialogFooter>
						</>
					)}
				</DialogContent>
			</Dialog>
		</>
	);
}
