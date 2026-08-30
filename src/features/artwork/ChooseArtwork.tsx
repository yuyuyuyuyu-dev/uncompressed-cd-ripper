import { useTranslation } from "react-i18next";
import type { Artwork } from "@/bindings";
import { Button } from "@/components/ui/button";
import "../language/i18n";
import { chosen } from "./artwork";

type Props = {
	onChoose: (artwork: Artwork) => void;
	disabled: boolean;
};

export function ChooseArtwork({ onChoose, disabled }: Props) {
	const { t } = useTranslation();

	const choose = async () => {
		const artwork = await chosen();

		if (artwork !== null) {
			onChoose(artwork);
		}
	};

	return (
		<Button variant="outline" size="sm" onClick={choose} disabled={disabled}>
			{t("artwork.choose")}
		</Button>
	);
}
