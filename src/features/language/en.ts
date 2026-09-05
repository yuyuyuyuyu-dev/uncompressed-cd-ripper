export const en = {
	cancel: "Cancel",
	app: {
		back: "Back",
		licenses: "Licenses",
	},
	ripping: {
		notification: {
			title: "Ripping finished",
			body_one: "{{count}} track is in the folder you chose.",
			body_other: "{{count}} tracks are in the folder you chose.",
		},
		heading: "Disc",
		eject: "Eject the disc",
		noDrive: "No drive with an audio CD in it.",
		chooseFolder: "Choose where to save",
		noFolder: "No folder chosen",
		rip: "Start ripping",
		progress: "Ripping track {{number}} · read {{read}} (max {{max}})",
		agreement:
			"The track is saved once {{required}} of its reads have matched. {{remaining}} more matches needed.",
		overwrite: {
			title: "Overwrite?",
			body_one:
				"This file is already in that folder and ripping would replace it.",
			body_other:
				"These {{count}} files are already where you chose to save, so ripping would overwrite them.",
			confirm: "Overwrite",
		},
	},
	metadata: {
		heading: "Metadata",
		lookUp: "Fetch CD details",
		lookingUp: "Fetching...",
		unknown:
			"This disc is not in the database. Its metadata can still be typed in.",
		choose: "Choose which of these the disc is.",
		album: "Album",
		albumArtist: "Album artist",
		title: "Title",
		artist: "Artist",
		length: "Length",
		matching: "Matching submissions",
		trackTitle: "Title of track {{number}}",
		trackArtist: "Artist of track {{number}}",
		ask: {
			title: "Fetch this CD's details?",
			body: "Fetching the album name, the track titles and the rest means sending part of this CD's information to MusicBrainz.\nSearching for the album artwork also sends the CD's identifier to the Cover Art Archive.\nThe picture itself is fetched from the Internet Archive.\nFetching anything from a server sends your IP address to that server.\nThat happens with any request over the internet, not only with this one.\nNo other data is sent.\nAnd if you cancel, nothing is sent at all.",
			confirm: "Fetch",
		},
	},
	verification: {
		label:
			"Check the ripped data against results other people submitted (AccurateRip)",
		unlisted:
			"AccurateRip's server holds nothing about this CD drive, so this could not be turned on.",
		ask: {
			title: "Turn AccurateRip on?",
			body: "AccurateRip checks whether the data you ripped is correct by comparing it with what other people got.\nA checksum — a value that always comes out the same when it is worked out from the same data — is worked out from the ripped data, and volunteers submit theirs to the server, where they build up.\nUsing this needs two things: your CD drive's read offset corrected, and part of the ripped CD's information sent to the server.\nCorrecting the read offset downloads AccurateRip's whole list of drive read offsets and matches it locally, so nothing about your CD drive is sent to the server.\nFetching the rips other people submitted does need part of the CD's information sent to the server.\nWhat is sent is which pressing the CD is, not the ripped data itself.\nThe rips are compared locally as well.\nFetching anything from a server sends your IP address to that server.\nThat happens with any request over the internet, not only with this one.\nNo other data is sent.\nAnd if you cancel, nothing is sent at all.",
			confirm: "Turn it on",
		},
	},
	artwork: {
		alt: "Album artwork",
		choose: "Choose artwork",
		images: "Images",
	},
	licenses: {
		heading: "Licenses",
		about: "This app uses the libraries below.",
	},
	errorReport: {
		details: "Details",
		title: "Send this error report?",
		body: "No data is sent unless you press the send button.\nPress it and the text below is sent exactly as it stands.",
		commentLabel: "Comment",
		commentPlaceholder: "Comment",
		reportLabel: "The error report",
		send: "Send",
		sentTitle: "Report sent",
		sentBody: "Thank you.",
	},
	selfUpdate: {
		title: "A new version is available.",
		body: "Restart the app to update?",
		changes: "See what changed",
		downloading: "Downloading the update",
		later: "Later",
		update: "Update",
	},
};
