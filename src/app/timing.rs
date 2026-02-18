use crate::app::SpectralApp;
use crate::timing::SnapDivision;

impl SpectralApp {
	pub fn sort_timing_points(&mut self) {
		self.timing_points
			.write()
			.unwrap()
			.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
	}

	pub fn get_beat_ticks(&self, start: f64, end: f64) -> Vec<(f64, SnapDivision)> {
		let mut ticks = vec![];

		let timing_points = self.timing_points.read().unwrap();

		for (i, tp) in timing_points.iter().enumerate() {
			let ms_per_beat = tp.ms_per_beat();
			let ms_per_tick = ms_per_beat / self.snap_divisor as f64;

			// TODO: stop rendering at low zoom

			let section_end = if i + 1 < timing_points.len() {
				timing_points[i + 1].offset
			} else {
				self.audio_data
					.as_ref()
					.map(|data| data.duration)
					.unwrap_or(f64::MAX)
			};

			let tick_start = start.max(tp.offset);
			let tick_end = end.min(section_end);

			if tick_start >= tick_end {
				continue;
			}

			let beats_from_start = ((tick_start - tp.offset) / ms_per_tick).floor() as i64;
			let first_tick_ms = tp.offset + (beats_from_start as f64 * ms_per_tick);

			let mut tick_ms = first_tick_ms;
			let mut tick_count = beats_from_start;

			while tick_ms <= tick_end {
				if tick_ms >= tick_start && tick_ms < section_end {
					let in_beat = tick_count.rem_euclid(self.snap_divisor);
					let beat_num = tick_count.div_euclid(self.snap_divisor);
					let in_measure = beat_num.rem_euclid(tp.signature.0 as i64);

					let snap = SnapDivision::from_tick(in_beat, self.snap_divisor, in_measure);

					ticks.push((tick_ms, snap));
				}
				tick_count += 1;
				tick_ms = tp.offset + (tick_count as f64 * ms_per_tick);
			}
		}

		ticks
	}
}
