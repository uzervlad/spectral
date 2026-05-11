// use crate::audio::AudioPlayer;
// use crate::metronome::samples::MetronomeSamples;
use crate::timing::TimingPoint;

pub mod samples;

#[derive(Debug, Clone, Copy)]
pub enum ClickType {
	Downbeat,
	Beat,
}

pub fn check_metronome(
	previous: f64,
	current: f64,
	timing_points: &[TimingPoint],
) -> Vec<(f64, ClickType)> {
	if timing_points.is_empty() {
		return vec![];
	}

	let tp_idx = timing_points
		.iter()
		.rposition(|tp| tp.offset <= current)
		.unwrap_or(0);

	let tp = &timing_points[tp_idx];
	let ms_per_beat = tp.ms_per_beat();

	let current_beat = ((current - tp.offset) / ms_per_beat).floor() as i64;
	let previous_beat = ((previous - tp.offset) / ms_per_beat).floor() as i64;

	let mut clicks = Vec::new();

	for beat in previous_beat + 1..=current_beat {
		let beat_time = tp.offset + (beat as f64 * ms_per_beat);
		if beat_time >= previous && beat_time < current {
			let ticks_per_measure = tp.signature.0 as i64;
			let is_downbeat = beat % ticks_per_measure == 0;

			clicks.push((
				beat_time,
				if is_downbeat {
					ClickType::Downbeat
				} else {
					ClickType::Beat
				},
			));
		}
	}

	clicks
}
