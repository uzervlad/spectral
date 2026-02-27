use std::f32::consts::PI;
use std::sync::Arc;

use egui::TextureHandle;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator as _};
use rustfft::{Fft, FftPlanner};
use rustfft::num_complex::Complex;

use crate::audio::AudioData;

pub mod colors;

pub struct Spectrogram {
	pub fft_size: usize,
	window: Vec<f32>,
	_planner: FftPlanner<f32>,
	fft: Arc<dyn Fft<f32>>
}

impl Spectrogram {
	pub fn new(fft_size: usize) -> Self {
		let window = (0..fft_size)
			.map(|i| 0.5 * (1. - (2. * PI * i as f32 / (fft_size - 1) as f32).cos()))
			.collect();

		let mut _planner = FftPlanner::new();
		let fft = _planner.plan_fft_forward(fft_size);

		Self {
			fft_size,
			window,
			_planner,
			fft,
		}
	}

	pub fn compute_column(
		&self,
		data: &AudioData,
		center_sample: isize,
		min_db: f32,
		max_db: f32,
	) -> Vec<f32> {
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

		self.fft.process(&mut buffer);

		buffer[..self.fft_size / 2]
			.par_iter()
			.map(|c| {
				let mag = c.norm() * 2. / self.fft_size as f32;
				let db = 20. * mag.max(1e-10).log10();
				((db - min_db) / (max_db - min_db)).clamp(0., 1.)
			})
			.collect()
	}

	pub fn compute_range(
		&self,
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
			.into_par_iter()
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
	fft_size: usize,
	min_db: f32,
	max_db: f32,
	pps: f64,
}

impl CachedSpectrogram {
	pub fn new(
		texture: TextureHandle,
		start_time: f64,
		end_time: f64,
		fft_size: usize,
		min_db: f32,
		max_db: f32,
		pps: f64,
	) -> Self {
		Self {
			texture,
			start_time,
			end_time,
			min_db,
			max_db,
			fft_size,
			pps,
		}
	}

	pub fn uv(&self, vis_start: f64, vis_end: f64) -> (f64, f64) {
		let len = self.end_time - self.start_time;

		(
			(vis_start - self.start_time) / len,
			(vis_end - self.start_time) / len,
		)
	}

	pub fn is_valid(
		&self,
		vis_start: f64,
		vis_end: f64,
		fft_size: usize,
		min_db: f32,
		max_db: f32,
		pps: f64,
	) -> bool {
		self.start_time <= vis_start
			&& self.end_time >= vis_end
			&& self.fft_size == fft_size
			&& (self.min_db - min_db).abs() < 0.1
			&& (self.max_db - max_db).abs() < 0.1
			&& self.pps == pps
	}
}
