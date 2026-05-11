use std::sync::Arc;

use eyre::Result;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::audio_new::load_audio_from_bytes;
use crate::metronome::ClickType;

const METRONOME_DOWNBEAT: &[u8] = include_bytes!("../assets/metronome-tick-downbeat.wav");
const METRONOME_BEAT: &[u8] = include_bytes!("../assets/metronome-tick.wav");

pub struct MetronomeSamples {
	pub downbeat: Arc<[f32]>,
	pub beat: Arc<[f32]>,
	// pub sample_rate: u32,
	// pub channels: u16,
}

impl MetronomeSamples {
	pub fn load(out_sample_rate: u32) -> Result<Self> {
		let (downbeat, sample_rate) = Self::decode(METRONOME_DOWNBEAT)?;
		let (beat, _) = Self::decode(METRONOME_BEAT)?;

		let downbeat = Self::resample(&downbeat, sample_rate, out_sample_rate);
		let beat = Self::resample(&beat, sample_rate, out_sample_rate);

		Ok(Self {
			downbeat: downbeat.into(),
			beat: beat.into(),
		})
	}

	fn decode(bytes: &'static [u8]) -> Result<(Vec<f32>, u32)> {
		let audio_data = load_audio_from_bytes(bytes);

		Ok((audio_data.samples().to_vec(), audio_data.sample_rate()))
	}

	fn resample(samples: &[f32], original_sample_rate: u32, target_sample_rate: u32) -> Vec<f32> {
		if original_sample_rate == target_sample_rate {
			return samples.to_vec();
		}
		
		let nbr_input_frames = samples.len() / 2;
		let f_ratio = target_sample_rate as f64 / original_sample_rate as f64;
		let mut outdata = vec![0.; 4 * (nbr_input_frames as f64 * f_ratio) as usize];

		let mut resampler = Fft::<f32>::new(
			original_sample_rate as usize,
			target_sample_rate as usize,
			1024,
			2,
			2,
			FixedSync::Both,
		).unwrap();



		let input_adapter = InterleavedSlice::new(samples, 2, nbr_input_frames).unwrap();
		let output_capacity = outdata.len() / 2;
		let mut output_adapter = InterleavedSlice::new_mut(&mut outdata, 2, output_capacity).unwrap();

		let (_, nbr_out) = resampler
			.process_all_into_buffer(&input_adapter, &mut output_adapter, nbr_input_frames, None)
			.unwrap();

		outdata.truncate(nbr_out * 2);

		outdata
	}

	pub fn get_sample(&self, click_type: ClickType) -> Arc<[f32]> {
		match click_type {
			ClickType::Downbeat => self.downbeat.clone(),
			ClickType::Beat => self.beat.clone(),
		}
	}
}
