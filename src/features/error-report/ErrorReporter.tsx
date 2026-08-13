import { useEffect, useState } from "react";
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
import { buildErrorReport } from "./error-report";

type Caught = {
	thrown: unknown;
	occurredAt: Date;
};

export function ErrorReporter() {
	const [caught, setCaught] = useState<Caught>();
	const [environment, setEnvironment] = useState<Environment>();
	const [comment, setComment] = useState("");
	const [showingDetail, setShowingDetail] = useState(false);

	useEffect(() => {
		commands.environment().then(setEnvironment);
	}, []);

	useEffect(() => {
		const catchThrown = (thrown: unknown) => {
			setCaught({ thrown, occurredAt: new Date() });
			setComment("");
			setShowingDetail(false);
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

	if (caught === undefined || environment === undefined) {
		return null;
	}

	// Rebuilt on every keystroke so that the text on screen is derived from the
	// same value that the send button hands over. Showing one and sending
	// another is the one way this screen could lie.
	const report = buildErrorReport({
		thrown: caught.thrown,
		environment,
		occurredAt: caught.occurredAt,
		comment,
	});

	const dismiss = () => {
		setCaught(undefined);
		setShowingDetail(false);
	};

	return (
		<>
			{/* Hidden while the detail screen is up, where it would otherwise sit
			    on top of the very dialog it opened. */}
			<div
				role="alert"
				hidden={showingDetail}
				className="fixed inset-x-0 bottom-0 z-40 flex items-center gap-3 border-t bg-card p-4 text-card-foreground"
			>
				<p className="min-w-0 flex-1 truncate text-sm">
					{report.exception.values[0].value}
				</p>
				<Button variant="outline" onClick={() => setShowingDetail(true)}>
					Details
				</Button>
				<Button variant="ghost" onClick={dismiss}>
					Dismiss
				</Button>
			</div>

			<Dialog open={showingDetail} onOpenChange={setShowingDetail}>
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

					<Textarea
						aria-label="What were you doing?"
						placeholder="What were you doing when this happened?"
						value={comment}
						onChange={(event) => setComment(event.currentTarget.value)}
					/>

					<section
						aria-label="The error report"
						className="max-h-72 overflow-auto rounded-lg border bg-muted p-3"
					>
						{/* Wrapped rather than scrolled sideways: a consent screen the
						    user has to drag around is one they will not read. */}
						<pre className="whitespace-pre-wrap break-all text-xs">
							{JSON.stringify(report, null, 2)}
						</pre>
					</section>

					<DialogFooter>
						<Button
							onClick={async () => {
								await commands.sendErrorReport(report);
								dismiss();
							}}
						>
							Send
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</>
	);
}
