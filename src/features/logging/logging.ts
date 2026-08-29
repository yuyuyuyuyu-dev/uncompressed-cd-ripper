import { commands, type Happening } from "@/bindings";

// The window's way into the trail the backend keeps. Nothing waits on it, and
// nothing is owed if it never arrives: an entry is only ever read back by a
// report about some later failure, and an app that stopped to complain it
// could not keep its own diary would be worse than one without one.
export function record(happening: Happening) {
	commands.record(happening).catch(() => {
		// Nothing to do about it, and nothing worth telling the user.
	});
}
