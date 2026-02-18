use egui::Rect;

pub struct Timeline {
	pub offset: f64,
	pub pixels_per_second: f64,
}

impl Timeline {
	pub fn new() -> Self {
		Self {
			offset: 0.,
			pixels_per_second: 100.,
		}
	}

	pub fn reset(&mut self) {
		self.offset = 0.;
		self.pixels_per_second = 100.;
	}

	pub fn pixels_per_ms(&self) -> f64 {
		self.pixels_per_second / 1000.
	}

	pub fn ms_to_x(&self, ms: f64, rect: Rect) -> f32 {
		rect.left() + ((ms - self.offset) * self.pixels_per_ms()) as f32
	}

	pub fn x_to_ms(&self, x: f32, rect: Rect) -> f64 {
		self.offset + (x - rect.left()) as f64 / self.pixels_per_ms()
	}

	pub fn visible_range(&self, width: f32) -> (f64, f64) {
		let start = self.offset;
		let end = self.offset + width as f64 / self.pixels_per_ms();
		(start, end)
	}

	fn max_scroll(&self, duration: f64, width: f32) -> f64 {
		(duration - width as f64 / self.pixels_per_ms()).max(0.)
	}

	pub fn scroll(&mut self, delta: f64, duration: f64, width: f32) {
		let delta = delta / self.pixels_per_ms();
		self.offset = (self.offset + delta).clamp(0., self.max_scroll(duration, width));
	}

	pub fn scroll_ms(&mut self, delta: f64, duration: f64, width: f32) {
		self.offset = (self.offset + delta).clamp(0., self.max_scroll(duration, width));
	}

	pub fn scroll_to(&mut self, ms: f64, duration: f64, width: f32) {
		let visible_duration = width as f64 / self.pixels_per_ms();
		self.offset = (ms - visible_duration / 2.).clamp(0., self.max_scroll(duration, width));
	}

	pub fn zoom(&mut self, delta: f64, focus: f64, duration: f64, width: f32) {
		let min_pps = 10_f64;
		let max_pps = 2000_f64;

		let current_ln = self.pixels_per_second.ln();
		let min_ln = min_pps.ln();
		let max_ln = max_pps.ln();

		let zoom_speed = 0.12;
		let new_ln = (current_ln * delta.powf(zoom_speed)).clamp(min_ln, max_ln);

		let old_pps = self.pixels_per_second;
		self.pixels_per_second = new_ln.exp();

		let focus_old = (focus - self.offset) * old_pps / 1000.;
		let focus_new = focus_old * 1000. / self.pixels_per_second;
		let max_offset = duration - (width as f64 / self.pixels_per_ms());
		self.offset = (focus - focus_new).min(max_offset).max(0.);
	}
}
