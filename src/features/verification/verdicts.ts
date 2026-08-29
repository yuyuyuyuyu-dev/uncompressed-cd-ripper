import type { Verdict } from "@/bindings";

// How many of the submissions AccurateRip holds for a track came out the same
// way as this one, for a column of numbers. Submissions rather than people or
// rips: what the database counts is what was sent to it, and it says nothing
// about how many people or how many readings that came from.
//
// Nothing to show where nobody has ever submitted the track, which is not the
// same as none of them matching: one is no answer and the other is an answer
// nobody wants.
export function matching(verdict: Verdict | undefined) {
	if (verdict === undefined || verdict.outcome === "unknown") {
		return "—";
	}

	return verdict.outcome === "different" ? "0" : String(verdict.others);
}
