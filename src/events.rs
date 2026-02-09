use std::path::PathBuf;

pub enum SpectralEvent {
	OpenAudio { path: PathBuf },
}