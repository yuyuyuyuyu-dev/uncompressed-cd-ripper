import type { Cover } from "@/bindings";

// An image element is given somewhere to fetch a picture from, and the picture
// has already crossed from the backend. This is the address of one in hand.
export function shown(cover: Cover) {
	return `data:${cover.mediaType};base64,${cover.data}`;
}
