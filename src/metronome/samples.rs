use std::{io::{BufReader, Cursor}, sync::Arc};

use eyre::Result;
use rodio::{Decoder, Source as _};

use crate::metronome::ClickType;

const METRONOME_DOWNBEAT: &'static [u8] = include_bytes!("../assets/metronome-tick-downbeat.wav");
const METRONOME_BEAT: &'static [u8] = include_bytes!("../assets/metronome-tick.wav");

pub struct MetronomeSamples {
	pub downbeat: Arc<Vec<f32>>,
	pub beat: Arc<Vec<f32>>,
	pub sample_rate: u32,
	pub channels: u16,
}

impl MetronomeSamples {
	pub fn load() -> Result<Self> {
		let (downbeat, sample_rate, channels) = Self::decode(METRONOME_DOWNBEAT)?;
		let (beat, _, _) = Self::decode(METRONOME_BEAT)?;

		Ok(Self {
			downbeat: Arc::new(downbeat),
			beat: Arc::new(beat),
			sample_rate,
			channels
		})
	}

	fn decode(bytes: &'static [u8]) -> Result<(Vec<f32>, u32, u16)> {
		let cursor = Cursor::new(bytes);
		let source = Decoder::new(BufReader::new(cursor))?;
		let sample_rate = source.sample_rate();
		let channels = source.channels();
		let samples: Vec<f32> = source.convert_samples().collect();
		Ok((samples, sample_rate, channels))
	}

	pub fn get_sample(&self, click_type: ClickType) -> (Arc<Vec<f32>>, u32, u16) {
		let samples = match click_type {
			ClickType::Downbeat => self.downbeat.clone(),
			ClickType::Beat => self.beat.clone(),
		};
		(samples, self.sample_rate, self.channels)
	}
}