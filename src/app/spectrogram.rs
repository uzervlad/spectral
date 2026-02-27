use egui::{ColorImage, TextureHandle};

use crate::app::SpectralApp;
use crate::spectrogram::{CachedSpectrogram, Spectrogram};

impl SpectralApp {
	pub fn generate_spectrogram(
		&mut self,
		ctx: &egui::Context,
		width: usize,
		height: usize,
	) -> Option<(TextureHandle, f64, f64)> {
		let audio = self.audio_data.as_ref()?;

		let (vis_start, vis_end) = self.timeline.visible_range(width as _);

		if let Some(cached) = &self.cached_spectrogram
			&& cached.is_valid(
				vis_start,
				vis_end,
				self.fft_size,
				self.min_db,
				self.max_db,
				self.timeline.pixels_per_second,
			) {
			let (x_from, x_to) = cached.uv(vis_start, vis_end);
			return Some((cached.texture.clone(), x_from, x_to));
		}

		if self.fft_size != self.spectrogram.fft_size {
			self.spectrogram = Spectrogram::new(self.fft_size);
		}

		let vis_len = vis_end - vis_start;
		let spec_start = (vis_start - vis_len / 4.).max(0.);
		let spec_end = (vis_end + vis_len / 4.).min(audio.duration);

		let spec_width = (spec_end - spec_start) / vis_len * width as f64;
 
		let columns = self.spectrogram.compute_range(
			audio,
			spec_start,
			spec_end,
			spec_width as _,
			self.min_db,
			self.max_db,
		);

		let freq_bins = self.spectrogram.fft_size / 2;

		let mut image = ColorImage::filled([spec_width as _, height], Default::default());

		for (x, column) in columns.iter().enumerate() {
			for y in 0..height {
				let norm_y = (height - 1 - y) as f32 / height as f32;
				let bin_float = norm_y * (freq_bins - 1) as f32;
				let bin_lo = bin_float.floor() as usize;
				let bin_hi = (bin_lo + 1).min(column.len() - 1);
				let frac = bin_float - bin_lo as f32;

				let value = column[bin_lo] * (1. - frac) + column[bin_hi] * frac;
				let color = self.spectrogram_colormap.get_color(value);

				image[(x, y)] = color;
			}
		}

		let texture = ctx.load_texture("spectrogram", image, egui::TextureOptions::LINEAR);

		let cached = CachedSpectrogram::new(
			texture,
			spec_start,
			spec_end,
			self.fft_size,
			self.min_db,
			self.max_db,
			self.timeline.pixels_per_second,
		);
		let (x_from, x_to) = cached.uv(vis_start, vis_end);
		
		self.cached_spectrogram = Some(cached);

		Some((self.cached_spectrogram.as_ref().unwrap().texture.clone(), x_from, x_to))
	}
}
