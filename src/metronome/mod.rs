use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use rodio::Sink;
use rodio::buffer::SamplesBuffer;

use crate::audio::AudioPlayer;
use crate::metronome::samples::MetronomeSamples;
use crate::timing::TimingPoint;

mod samples;

enum ClickType {
	Downbeat,
	Beat,
}

fn check_metronome(
	previous: f64,
	current: f64,
	timing_points: &[TimingPoint],
) -> Option<ClickType> {
	if timing_points.is_empty() {
		return None;
	}

	let tp_idx = timing_points
		.iter()
		.rposition(|tp| tp.offset <= current)
		.unwrap_or(0);

	let tp = &timing_points[tp_idx];
	let ms_per_beat = tp.ms_per_beat();

	let current_beat = ((current - tp.offset) / ms_per_beat).floor() as i64;
	let previous_beat = ((previous - tp.offset) / ms_per_beat).floor() as i64;

	if current_beat > previous_beat && current >= tp.offset {
		let ticks_per_measure = tp.signature.0 as i64;

		let is_downbeat = current_beat % ticks_per_measure == 0;

		Some(if is_downbeat {
			ClickType::Downbeat
		} else {
			ClickType::Beat
		})
	} else {
		None
	}
}

pub fn metronome_thread(
	state: MetronomeState,
	sink: Arc<Sink>,
	timing_points: Arc<RwLock<Vec<TimingPoint>>>,
) {
	let samples = MetronomeSamples::load().expect("Failed to load metronome samples");

	let mut playhead_ms = state.get_position_ms();

	loop {
		let previous_ms = playhead_ms;

		if state.is_playing() {
			playhead_ms = state.get_position_ms();

			if let Some(click) =
				check_metronome(previous_ms, playhead_ms, &timing_points.read().unwrap())
			{
				let (samples, sample_rate, channels) = samples.get_sample(click);

				let source = SamplesBuffer::new(channels, sample_rate, samples.as_ref().clone());
				sink.append(source);
			}
		}

		thread::sleep(Duration::from_millis(3));
	}
}

pub struct MetronomeState {
	playing: Arc<AtomicBool>,
	sample_rate: Arc<AtomicU32>,
	channels: Arc<AtomicU16>,
	position: Arc<AtomicUsize>,
}

impl From<&AudioPlayer> for MetronomeState {
	fn from(value: &AudioPlayer) -> Self {
		Self {
			playing: value.playing.clone(),
			sample_rate: value.sample_rate.clone(),
			channels: value.channels.clone(),
			position: value.position.clone(),
		}
	}
}

impl MetronomeState {
	fn get_position_ms(&self) -> f64 {
		let pos = self.position.load(Ordering::SeqCst);
		let frame = pos / self.channels.load(Ordering::SeqCst) as usize;
		(frame as f64 / self.sample_rate.load(Ordering::SeqCst) as f64) * 1000.
	}

	fn is_playing(&self) -> bool {
		self.playing.load(Ordering::SeqCst)
	}
}
