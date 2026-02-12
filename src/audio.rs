use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicUsize, Ordering};
use std::time::Duration;

use eyre::Result;
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
pub struct AudioData {
	pub samples: Arc<Vec<f32>>,
	pub mono_samples: Arc<Vec<f32>>,
	pub sample_rate: u32,
	pub channels: u16,
	pub duration: f64,
}

impl AudioData {
	pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
		let file = File::open(path)?;
		let source = Decoder::new(BufReader::new(file))?;

		let sample_rate = source.sample_rate();
		let channels = source.channels();

		let samples: Vec<_> = source.convert_samples().collect();

		let mono_samples = if channels > 1 {
			samples
				.chunks(channels as usize)
				.map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
				.collect()
		} else {
			samples.clone()
		};

		let duration = (mono_samples.len() as f64 / sample_rate as f64) * 1000.;

		Ok(Self {
			samples: Arc::new(samples),
			mono_samples: Arc::new(mono_samples),
			sample_rate,
			channels,
			duration,
		})
	}

	pub fn ms_to_idx(&self, ms: f64) -> usize {
		let frame = ((ms / 1000.) * self.sample_rate as f64) as usize;
		(frame * self.channels as usize).min(self.samples.len())
	}

	pub fn idx_to_ms(&self, idx: usize) -> f64 {
		let frame = idx / self.channels as usize;
		(frame as f64 / self.sample_rate as f64) * 1000.
	}
}

pub struct SeekableSource {
	samples: Arc<Vec<f32>>,
	sample_rate: u32,
	channels: u16,
	position: Arc<AtomicUsize>,
	playing: Arc<AtomicBool>,
}

impl Iterator for SeekableSource {
	type Item = f32;

	fn next(&mut self) -> Option<Self::Item> {
		if !self.playing.load(Ordering::SeqCst) {
			return Some(0.0);
		}

		let pos = self.position.fetch_add(1, Ordering::SeqCst);
		if pos < self.samples.len() {
			Some(self.samples[pos])
		} else {
			self.playing.store(false, Ordering::SeqCst);
			Some(0.0)
		}
	}
}

impl rodio::Source for SeekableSource {
	fn current_frame_len(&self) -> Option<usize> {
		None
	}

	fn channels(&self) -> u16 {
		self.channels
	}

	fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	fn total_duration(&self) -> Option<std::time::Duration> {
		let total_frames = self.samples.len() / self.channels as usize;
		Some(Duration::from_secs_f64(
			total_frames as f64 / self.sample_rate as f64,
		))
	}
}

pub struct AudioPlayer {
	_stream: OutputStream,
	handle: OutputStreamHandle,
	sink: Option<Sink>,
	pub metronome_sink: Arc<Sink>,

	samples: Option<Arc<Vec<f32>>>,
	pub sample_rate: Arc<AtomicU32>,
	pub channels: Arc<AtomicU16>,
	pub position: Arc<AtomicUsize>,
	pub playing: Arc<AtomicBool>,

	duration: f64,
	volume: f32,
}

impl AudioPlayer {
	pub fn new() -> Result<Self> {
		let (_stream, handle) = OutputStream::try_default()?;

		let metronome_sink = Arc::new(Sink::try_new(&handle)?);
		metronome_sink.set_volume(0.2);

		Ok(Self {
			_stream,
			handle,
			sink: None,
			metronome_sink,

			samples: None,
			sample_rate: Arc::new(AtomicU32::new(41000)),
			channels: Arc::new(AtomicU16::new(1)),
			position: Arc::new(AtomicUsize::new(0)),
			playing: Arc::new(AtomicBool::new(false)),

			duration: 0.,
			volume: 0.4,
		})
	}

	pub fn load(&mut self, audio_data: &AudioData) -> Result<()> {
		if let Some(sink) = self.sink.take() {
			sink.stop();
		}

		self.samples = Some(audio_data.samples.clone());
		self.sample_rate
			.store(audio_data.sample_rate, Ordering::SeqCst);
		self.channels.store(audio_data.channels, Ordering::SeqCst);
		self.duration = audio_data.duration;
		self.position.store(0, Ordering::SeqCst);
		self.playing.store(false, Ordering::SeqCst);

		self.create_sink()?;

		Ok(())
	}

	fn create_sink(&mut self) -> Result<()> {
		if let Some(samples) = &self.samples {
			let source = SeekableSource {
				samples: samples.clone(),
				sample_rate: self.sample_rate.load(Ordering::SeqCst),
				channels: self.channels.load(Ordering::SeqCst),
				position: self.position.clone(),
				playing: self.playing.clone(),
			};

			let sink = Sink::try_new(&self.handle)?;
			sink.set_volume(self.volume);
			sink.append(source);

			self.sink = Some(sink);
		}

		Ok(())
	}

	pub fn play_metronome(&self, samples: Arc<Vec<f32>>, sample_rate: u32, channels: u16) {
		let source =
			SamplesBuffer::new(channels, sample_rate, samples.as_ref().clone()).amplify(0.2);
		self.metronome_sink.append(source);
	}

	pub fn play(&self) {
		self.playing.store(true, Ordering::SeqCst);
	}

	pub fn pause(&self) {
		self.playing.store(false, Ordering::SeqCst);
	}

	pub fn play_pause(&self) {
		if self.playing.load(Ordering::SeqCst) {
			self.pause()
		} else {
			if self.duration == 0. {
				return;
			}

			if self.get_position_ms() >= self.duration - 50. {
				self.seek_to(0.);
			}
			self.play();
		}
	}

	pub fn is_playing(&self) -> bool {
		self.playing.load(Ordering::SeqCst)
	}

	pub fn get_position_ms(&self) -> f64 {
		let pos = self.position.load(Ordering::SeqCst);
		let frame = pos / self.channels.load(Ordering::SeqCst) as usize;
		(frame as f64 / self.sample_rate.load(Ordering::SeqCst) as f64) * 1000.
	}

	pub fn seek_to(&self, ms: f64) {
		let ms = ms.clamp(0., self.duration);
		let frame = ((ms / 1000.) * self.sample_rate.load(Ordering::SeqCst) as f64) as usize;

		let sample_idx = frame * self.channels.load(Ordering::SeqCst) as usize;
		let max_idx = self.samples.as_ref().map(|s| s.len()).unwrap_or(0);

		self.position
			.store(sample_idx.min(max_idx), Ordering::SeqCst);
	}

	pub fn set_volume(&mut self, volume: f32) {
		self.volume = volume.clamp(0., 1.);
		if let Some(sink) = &self.sink {
			sink.set_volume(self.volume);
		}
	}

	pub fn get_volume(&self) -> f32 {
		self.volume
	}

	pub fn set_metronome_volume(&self, volume: f32) {
		self.metronome_sink.set_volume(volume);
	}

	pub fn get_metronome_volume(&self) -> f32 {
		self.metronome_sink.volume()
	}
}
