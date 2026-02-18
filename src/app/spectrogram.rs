use egui::{Color32, ColorImage, TextureHandle};

use crate::{app::SpectralApp, spectrogram::{CachedSpectrogram, Spectrogram}, util::magma_colormap};

impl SpectralApp {
	pub fn generate_spectrogram(
		&mut self,
		ctx: &egui::Context,
		width: usize,
		height: usize,
	) -> Option<TextureHandle> {
		let audio = self.audio_data.as_ref()?;

		let (vis_start, vis_end) = self.timeline.visible_range(width as _);

		if let Some(cached) = &self.cached_spectrogram {
			if cached.is_valid(
				vis_start,
				vis_end,
				self.fft_size,
				self.min_db,
				self.max_db,
				width,
			) {
				return Some(cached.texture.clone());
			}
		}

		if self.fft_size != self.spectrogram.fft_size {
			self.spectrogram = Spectrogram::new(self.fft_size);
		}

		let columns = self.spectrogram.compute_range(
			audio,
			vis_start,
			vis_end,
			width,
			self.min_db,
			self.max_db,
		);

		let freq_bins = self.spectrogram.fft_size / 2;

		let mut image = ColorImage::filled([width, height], Color32::BLACK);

		for (x, column) in columns.iter().enumerate() {
			for y in 0..height {
				let norm_y = (height - 1 - y) as f32 / height as f32;
				let bin_float = norm_y * (freq_bins - 1) as f32;
				let bin_lo = bin_float.floor() as usize;
				let bin_hi = (bin_lo + 1).min(column.len() - 1);
				let frac = bin_float - bin_lo as f32;

				let value = column[bin_lo] * (1. - frac) + column[bin_hi] * frac;
				let color = magma_colormap(value);

				image[(x, y)] = color;
			}
		}

		let texture = ctx.load_texture("spectrogram", image, egui::TextureOptions::LINEAR);

		self.cached_spectrogram = Some(CachedSpectrogram::new(
			texture,
			vis_start,
			vis_end,
			self.fft_size,
			self.min_db,
			self.max_db,
			width,
		));

		Some(self.cached_spectrogram.as_ref().unwrap().texture.clone())
	}
}