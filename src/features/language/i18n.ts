import i18next from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";
import { en } from "./en";
import { ja } from "./ja";

i18next
	.use(LanguageDetector)
	.use(initReactI18next)
	.init({
		resources: {
			en: { translation: en },
			ja: { translation: ja },
		},
		supportedLngs: ["en", "ja"],
		fallbackLng: "en",
		detection: { order: ["navigator"], caches: [] },
		interpolation: { escapeValue: false },
	});

export default i18next;
