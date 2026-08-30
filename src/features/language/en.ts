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
		scan: "Scan for discs",
		noDrive: "No drive with an audio CD in it.",
		chooseFolder: "Choose a folder",
		noFolder: "No folder chosen",
		rip: "Rip",
		progress: "Ripping track {{number}} · read {{read}} (max {{max}})",
		agreement:
			"The track is saved when {{required}} reads of it match. {{remaining}} more needed.",
		overwrite: {
			title: "Overwrite what is already there?",
			body_one:
				"This file is already in that folder and ripping would replace it.",
			body_other:
				"These {{count}} files are already in that folder and ripping would replace them.",
			confirm: "Overwrite",
		},
	},
	metadata: {
		heading: "Metadata",
		lookUp: "Look this disc up",
		lookingUp: "Looking it up…",
		waiting: "Waiting for an answer about this disc.",
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
			title: "Look this disc up?",
			body: "Finding the album and the track titles means sending a fingerprint of this disc, worked out from where its tracks begin, to MusicBrainz. The album artwork is then asked for from the Artwork Art Archive, which is sent the identifier of whichever release matched and serves the picture from the Internet Archive. The address your machine reaches the internet from goes with both, as it does with any request. Nothing else about you is sent, and nothing is sent at all unless you say so.",
			confirm: "Look it up",
		},
	},
	verification: {
		label:
			"Check the ripped tracks against other people's submissions (AccurateRip)",
		unlisted:
			"AccurateRip has never been told about this drive, so tracks read from it cannot be checked against anybody else's rips.",
		on: "This drive's read offset is set. Every track read from it is put right by that much.",
		ask: {
			title: "Check this rip against other people's submissions?",
			body: "AccurateRip is a list of what other people's drives made of the same discs: one submission for every rip anybody has ever sent in. Turning this on does two things. Every drive reads a little ahead of or behind where the disc says, so AccurateRip's list of the drives it knows about is downloaded and searched on this machine to find how far out this one is; nothing about your drive is sent. Then, once a disc has been read, a fingerprint of it, worked out from where its tracks begin, is sent to AccurateRip, and everything AccurateRip holds about that disc comes back. Nothing of the rip itself goes out: what came off the disc is held against what came back here, on this machine. The address your machine reaches the internet from goes with both requests, as it does with any request. None of your music is sent, and nothing is sent while this is off.",
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
		body: "Nothing leaves this machine unless you send it. Below is the report exactly as it would be sent.",
		commentLabel: "What were you doing?",
		commentPlaceholder: "What were you doing when this happened?",
		reportLabel: "The error report",
		send: "Send",
		sentTitle: "Report sent",
		sentBody: "Thank you.",
	},
};
