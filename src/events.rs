use std::path::PathBuf;

use eyre::Result;

use crate::audio::AudioData;

pub enum SpectralEvent {
	OpenAudio { path: PathBuf },
	LoadAudio { data: Result<AudioData> },
	Export { error: Option<String> },
}
