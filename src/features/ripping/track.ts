// A CD is addressed in sectors, 75 of which make a second.
const SECTORS_PER_SECOND = 75;

export function length(sectors: number) {
	const seconds = Math.round(sectors / SECTORS_PER_SECOND);

	return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
