import { Component, type ErrorInfo, type ReactNode } from "react";

type Props = {
	onError: (thrown: unknown, componentStack: string) => void;
	children: ReactNode;
};

// The only thing React offers for an error thrown while rendering, and it has
// to be a class: there is no hook for this.
export class RenderErrorBoundary extends Component<Props, { failed: boolean }> {
	state = { failed: false };

	static getDerivedStateFromError() {
		return { failed: true };
	}

	componentDidCatch(thrown: unknown, info: ErrorInfo) {
		// Deferred out of the commit. A failure on the very first render happens
		// before whatever shows the notification has finished mounting, and
		// reporting straight away hands it to something not yet listening.
		queueMicrotask(() => this.props.onError(thrown, info.componentStack ?? ""));
	}

	render() {
		if (!this.state.failed) {
			return this.props.children;
		}

		// Something has to stand where the interface was. React takes the whole
		// tree down otherwise, and an empty window gives the user nothing to
		// report from.
		return (
			<main className="flex flex-col items-center gap-4 pt-[10vh] text-center">
				<h1 className="text-2xl font-semibold">Something went wrong</h1>
				<p className="text-muted-foreground text-sm">
					The window could not be drawn. Restart the application to carry on.
				</p>
			</main>
		);
	}
}
