use std::path::PathBuf;

use crate::audio_new::AudioData;

pub enum SpectralEvent {
	OpenAudio { path: PathBuf },
	LoadAudio { data: AudioData },
	Export { error: Option<String> },
}
