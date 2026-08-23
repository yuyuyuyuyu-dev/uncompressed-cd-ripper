import type { Cover } from "@/bindings";
import { Button } from "@/components/ui/button";
import { chosen } from "./artwork";

type Props = {
	onChoose: (cover: Cover) => void;
	disabled: boolean;
};

export function ChooseCover({ onChoose, disabled }: Props) {
	const choose = async () => {
		const cover = await chosen();

		if (cover !== null) {
			onChoose(cover);
		}
	};

	return (
		<Button variant="outline" size="sm" onClick={choose} disabled={disabled}>
			Choose a picture
		</Button>
	);
}
