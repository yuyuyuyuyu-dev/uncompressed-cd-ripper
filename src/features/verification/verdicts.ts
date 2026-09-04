import type { Verdict } from "@/bindings";

export function matching(verdict: Verdict | undefined) {
	if (verdict === undefined || verdict.outcome === "unknown") {
		return "—";
	}

	return verdict.outcome === "different" ? "0" : String(verdict.others);
}
