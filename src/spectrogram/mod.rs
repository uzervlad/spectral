use std::f32::consts::PI;

use egui::TextureHandle;
use rustfft::{FftPlanner, num_complex::Complex};

use crate::audio::AudioData;

pub struct Spectrogram {
	pub fft_size: usize,
	window: Vec<f32>,
	planner: FftPlanner<f32>,
}

impl Spectrogram {
	pub fn new(fft_size: usize) -> Self {
		let window = (0..fft_size)
			.map(|i| 0.5 * (1. - (2. * PI * i as f32 / (fft_size - 1) as f32).cos()))
			.collect();

		Self {
			fft_size,
			window,
			planner: FftPlanner::new(),
		}
	}

	pub fn compute_column(
		&mut self,
		data: &AudioData,
		center_sample: isize,
		min_db: f32,
		max_db: f32
	) -> Vec<f32> {
		let fft = self.planner.plan_fft_forward(self.fft_size);
		let half = (self.fft_size / 2) as isize;

		let mut buffer: Vec<_> = (0..self.fft_size)
			.map(|i| {
				let idx = center_sample - half + i as isize;
				let sample = if idx >= 0 && (idx as usize) < data.mono_samples.len() {
					data.mono_samples[idx as usize]
				} else {
					0.
				};
				Complex::new(sample * self.window[i], 0.)
			})
			.collect();

		fft.process(&mut buffer);

		buffer[..self.fft_size / 2]
			.iter()
			.map(|c| {
				let mag = c.norm() * 2. / self.fft_size as f32;
				let db = 20. * mag.max(1e-10).log10();
				((db - min_db) / (max_db - min_db)).clamp(0., 1.)
			})
			.collect()
	}

	pub fn compute_range(
		&mut self,
		data: &AudioData,
		start_time: f64,
		end_time: f64,
		columns: usize,
		min_db: f32,
		max_db: f32,
	) -> Vec<Vec<f32>> {
		let start_sample = (start_time / 1000. * data.sample_rate as f64) as isize;
		let end_sample = (end_time / 1000. * data.sample_rate as f64) as isize;
		let samples_per_column = (end_sample - start_sample) as f64 / columns as f64;

		(0..columns)
			.map(|i| {
				let sample = start_sample + (i as f64 * samples_per_column) as isize;
				self.compute_column(data, sample, min_db, max_db)
			})
			.collect()
	}
}

pub struct CachedSpectrogram {
	pub texture: TextureHandle,
	start_time: f64,
	end_time: f64,
	// fft_size: usize,
	width: usize,
}

impl CachedSpectrogram {
	pub fn new(texture: TextureHandle, start_time: f64, end_time: f64, width: usize) -> Self {
		Self {
			texture,
			start_time,
			end_time,
			width
		}
	}

	pub fn is_valid(
		&self,
		start_time: f64,
		end_time: f64,
		// fft_size: usize,
		width: usize
	) -> bool {
		(self.start_time - start_time).abs() < 0.001
			&& (self.end_time - end_time).abs() < 0.001
			// && self.fft_size == fft_size
			&& self.width == width
	}
}