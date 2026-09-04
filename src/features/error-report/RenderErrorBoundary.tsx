import { Component, type ErrorInfo, type ReactNode } from "react";

type Props = {
	onError: (thrown: unknown, componentStack: string) => void;
	children: ReactNode;
};

export class RenderErrorBoundary extends Component<Props, { failed: boolean }> {
	state = { failed: false };

	static getDerivedStateFromError() {
		return { failed: true };
	}

	componentDidCatch(thrown: unknown, info: ErrorInfo) {
		queueMicrotask(() => this.props.onError(thrown, info.componentStack ?? ""));
	}

	render() {
		if (!this.state.failed) {
			return this.props.children;
		}

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
