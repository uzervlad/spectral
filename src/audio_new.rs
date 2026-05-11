use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Async, FixedAsync, PolynomialDegree, Resampler};
use symphonia::core::audio::{AudioBuffer, Signal as _};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

use crate::metronome::samples::MetronomeSamples;
use crate::metronome::{ClickType, check_metronome};
use crate::settings::SettingsManager;
use crate::timing::TimingPoint;

pub struct AudioData {
	samples: Arc<[f32]>,
	mono_samples: Arc<[f32]>,
	sample_rate: u32,
	original_sample_rate: u32,
	duration: f64,
}

impl AudioData {
	pub fn samples(&self) -> &[f32] {
		&self.samples
	}

	pub fn mono_samples(&self) -> &[f32] {
		&self.mono_samples
	}

	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	pub fn original_sample_rate(&self) -> u32 {
		self.original_sample_rate
	}

	pub fn duration(&self) -> f64 {
		self.duration
	}
}

pub struct AudioState {
	audio: Arc<AudioData>,

	playing: bool,
	volume: f32,
	metronome_volume: f32,

	metronome_current: Vec<(usize, ClickType)>,

	position: f64,
	playback_speed: f64,

	resampler: Option<Arc<Mutex<Async<f32>>>>,
	input_position: usize,
	resample_buffer: Vec<f32>,
	resample_buffer_pos: usize,
}

impl AudioState {
	pub fn new(settings: Arc<SettingsManager>) -> Self {
		Self {
			volume: settings.read(|s| s.audio_volume),
			metronome_volume: settings.read(|s| s.metronome_volume),
			..Default::default()
		}
	}
}

impl Default for AudioState {
	fn default() -> Self {
		Self {
			audio: Arc::new(AudioData {
				samples: Arc::new([]),
				mono_samples: Arc::new([]),
				sample_rate: 44100,
				original_sample_rate: 44100,
				duration: 0.,
			}),
			playing: false,
			volume: 0.25,
			metronome_volume: 0.25,
			metronome_current: Vec::new(),
			position: 0.,
			playback_speed: 1.,
			resampler: None,
			input_position: 0,
			resample_buffer: Vec::with_capacity(8192),
			resample_buffer_pos: 0,
		}
	}
}

pub struct AudioSystem {
	state: Arc<RwLock<AudioState>>,
	sample_rate: u32,
	channels: usize,
	_stream: Stream,
}

impl AudioSystem {
	pub fn new(
		settings: Arc<SettingsManager>,
		timing_points: Arc<RwLock<Vec<TimingPoint>>>,
	) -> Self {
		let host = cpal::default_host();

		let device = host
			.default_output_device()
			.expect("no output device available");

		let supported_configs: Vec<_> = device
			.supported_output_configs()
			.expect("failed to get supported configs")
			.filter(|cfg| cfg.sample_format() == SampleFormat::F32)
			.collect();

		// stereo 48kHz > stereo any > mono 48kHz > mono any
		let best_config = supported_configs
			.iter()
			.max_by_key(|cfg| {
				let is_stereo = cfg.channels() == 2;
				let supports_48k = cfg.max_sample_rate() >= 48000;

				match (is_stereo, supports_48k) {
					(true, true) => 3,
					(true, false) => 2,
					(false, true) => 1,
					(false, false) => 0,
				}
			})
			.expect("no output configs available");

		let config = best_config
			.try_with_sample_rate(48000)
			.unwrap_or_else(|| best_config.with_max_sample_rate())
			.config();

		let state = Arc::new(RwLock::new(AudioState::new(settings)));

		let sample_rate = config.sample_rate;
		let channels = config.channels as usize;

		let _stream = device
			.build_output_stream(
				&config,
				create_audio_data_callback(state.clone(), timing_points, sample_rate, channels),
				|_| {},
				None,
			)
			.unwrap();

		_stream.play().expect("failed to play stream");

		Self {
			state,
			sample_rate,
			channels,
			_stream,
		}
	}

	pub fn load_audio_data(&mut self, data: Arc<AudioData>) {
		let input_sample_rate = data.original_sample_rate();
		let output_sample_rate = self.sample_rate;
		let channels = self.channels;

		let f_ratio = output_sample_rate as f64 / input_sample_rate as f64;
		let resampler = if (f_ratio - 1.0).abs() > 0.001 {
			Some(Arc::new(Mutex::new(
				Async::<f32>::new_poly(
					f_ratio,
					1.1,
					PolynomialDegree::Septic,
					1024,
					channels,
					FixedAsync::Output,
				)
				.unwrap(),
			)))
		} else {
			None
		};

		let mut state = self.state.write().unwrap();
		state.audio = data;
		state.resampler = resampler;

		state.input_position = 0;
		state.position = 0.0;

		state.resample_buffer.clear();
		state.resample_buffer_pos = 0;
	}

	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	pub fn is_playing(&self) -> bool {
		self.state.read().unwrap().playing
	}

	pub fn play(&self) {
		self.state.write().unwrap().playing = true;
	}

	pub fn pause(&self) {
		let mut state = self.state.write().unwrap();

		state.playing = false;
		state.metronome_current.clear();
	}

	pub fn toggle_playback(&self) {
		let mut state = self.state.write().unwrap();
		state.playing = !state.playing;
	}

	pub fn seek_to(&self, ms: f64) {
		let mut state = self.state.write().unwrap();

		state.resample_buffer.clear();
		state.resample_buffer_pos = 0;

		state.position =
			(ms / 1000.) * self.channels as f64 * state.audio.original_sample_rate() as f64;
		state.input_position = (state.position as usize / self.channels) * self.channels;

		state.metronome_current.clear();
	}

	pub fn get_volume(&self) -> f32 {
		self.state.read().unwrap().volume
	}

	pub fn set_volume(&self, volume: f32) {
		self.state.write().unwrap().volume = volume;
	}

	pub fn get_metronome_volume(&self) -> f32 {
		self.state.read().unwrap().metronome_volume
	}

	pub fn set_metronome_volume(&self, volume: f32) {
		self.state.write().unwrap().metronome_volume = volume;
	}

	pub fn get_position_ms(&self) -> f64 {
		let state = self.state.read().unwrap();
		(state.position / self.channels as f64 / state.audio.original_sample_rate() as f64) * 1000.
	}

	pub fn get_playback_speed(&self) -> f64 {
		self.state.read().unwrap().playback_speed
	}

	pub fn set_playback_speed(&self, speed: f64) {
		let mut state = self.state.write().unwrap();

		state.playback_speed = speed;

		let output_sr = self.sample_rate as f64;
		let input_sr = state.audio.original_sample_rate() as f64;
		let base_ratio = output_sr / input_sr;
		let effective_ratio = base_ratio / speed;

		if (effective_ratio - 1.0).abs() > 0.001 {
			state.resampler = Some(Arc::new(Mutex::new(
				Async::<f32>::new_poly(
					effective_ratio,
					1.1,
					PolynomialDegree::Septic,
					1024,
					self.channels,
					FixedAsync::Output,
				)
				.unwrap(),
			)));
		} else {
			state.resampler = None;
		}

		state.resample_buffer.clear();
		state.resample_buffer_pos = 0;

		state.input_position = state.position as usize;
	}
}

pub fn load_audio_from_bytes(bytes: &'static [u8]) -> AudioData {
	let cursor = Cursor::new(bytes);
	let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

	let mut hint = Hint::new();
	hint.with_extension("wav");

	let mut format = symphonia::default::get_probe()
		.format(&hint, mss, &Default::default(), &Default::default())
		.unwrap();

	let track = format.format.default_track().unwrap().clone();

	let mut decoder = symphonia::default::get_codecs()
		.make(&track.codec_params, &Default::default())
		.unwrap();

	let track_id = track.id;

	let channels = track.codec_params.channels.unwrap().count();
	let sample_rate = track.codec_params.sample_rate.unwrap();

	let mut samples: Vec<f32> = vec![];

	while let Ok(packet) = format.format.next_packet() {
		while !format.format.metadata().is_latest() {
			format.format.metadata().pop();
		}

		if packet.track_id() != track_id {
			continue;
		}

		match decoder.decode(&packet) {
			Ok(decoded) => {
				let mut buf = AudioBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
				decoded.convert(&mut buf);
				for frame in 0..buf.frames() {
					for ch in 0..channels {
						samples.push(buf.chan(ch)[frame]);
					}
				}
			},
			Err(symphonia::core::errors::Error::DecodeError(_)) => {
				continue;
			},
			Err(_) => {
				panic!("erm");
			},
		}
	}

	AudioData {
		samples: samples.into(),
		mono_samples: Arc::new([]),
		sample_rate,
		original_sample_rate: sample_rate,
		duration: 0.,
	}
}

pub fn load_audio_from_path(path: PathBuf) -> AudioData {
	let file = File::open(&path).unwrap();

	let mss = MediaSourceStream::new(Box::new(file), Default::default());

	let mut hint = Hint::new();
	hint.with_extension(path.extension().unwrap().to_str().unwrap());

	let mut format = symphonia::default::get_probe()
		.format(&hint, mss, &Default::default(), &Default::default())
		.unwrap();

	let track = format.format.default_track().unwrap().clone();

	let mut decoder = symphonia::default::get_codecs()
		.make(&track.codec_params, &Default::default())
		.unwrap();

	let track_id = track.id;

	let channels = track.codec_params.channels.unwrap().count();
	let sample_rate = track.codec_params.sample_rate.unwrap();

	let mut samples: Vec<f32> = vec![];

	while let Ok(packet) = format.format.next_packet() {
		while !format.format.metadata().is_latest() {
			format.format.metadata().pop();
		}

		if packet.track_id() != track_id {
			continue;
		}

		match decoder.decode(&packet) {
			Ok(decoded) => {
				let mut buf = AudioBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
				decoded.convert(&mut buf);
				for frame in 0..buf.frames() {
					for ch in 0..channels {
						samples.push(buf.chan(ch)[frame]);
					}
				}
			},
			Err(symphonia::core::errors::Error::DecodeError(_)) => {
				continue;
			},
			Err(_) => {
				panic!("erm");
			},
		}
	}

	let mono_samples = match channels {
		1 => samples.clone(),
		_ => samples
			.chunks(channels as usize)
			.map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
			.collect(),
	};

	let duration = (mono_samples.len() as f64 / sample_rate as f64) * 1000.;

	AudioData {
		samples: samples.into(),
		mono_samples: mono_samples.into(),
		sample_rate,
		original_sample_rate: sample_rate,
		duration,
	}
}

/*
	WARNING: Most of the code below has been vibecoded
	I can barely understand it, but it does its job
	so I'm keeping it for now
*/

fn samples_to_ms(position: f64, channels: usize, sample_rate: u32) -> f64 {
	position / channels as f64 / sample_rate as f64 * 1000.
}

fn create_audio_data_callback(
	state: Arc<RwLock<AudioState>>,
	timing_points: Arc<RwLock<Vec<TimingPoint>>>,
	sample_rate: u32,
	channels: usize,
) -> impl FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static {
	let metronome_samples = MetronomeSamples::load(sample_rate).unwrap();

	move |data: &mut [f32], _| {
		let mut state = state.write().unwrap();

		let original_sample_rate = state.audio.original_sample_rate();
		let speed = state.playback_speed;
		let source_samples_per_output_sample =
			speed * original_sample_rate as f64 / sample_rate as f64;

		let start_ms = samples_to_ms(state.position, channels, original_sample_rate);
		let end_ms = start_ms
			+ (data.len() as f64 / channels as f64 / original_sample_rate as f64) * speed * 1000.;

		let new_clicks = check_metronome(start_ms, end_ms, &timing_points.read().unwrap());

		for (time, click) in new_clicks {
			let click_samples: Arc<[f32]> = metronome_samples.get_sample(click);

			let start_sample =
				((time - start_ms) / 1000.) / speed * sample_rate as f64 * channels as f64;

			for i in 0..data.len() {
				if (i as f64) >= start_sample && i < click_samples.len() {
					data[i] += click_samples[i] * state.metronome_volume;
				}
			}

			state
				.metronome_current
				.push((data.len() - start_sample as usize, click));
		}

		let metronome_volume = state.metronome_volume;

		for (sample_pos, click_type) in state.metronome_current.iter_mut() {
			let click_samples = match click_type {
				ClickType::Downbeat => &metronome_samples.downbeat,
				ClickType::Beat => &metronome_samples.beat,
			};

			for sample in data.iter_mut() {
				if *sample_pos < click_samples.len() {
					*sample += click_samples[*sample_pos] * metronome_volume;
					*sample_pos += 1;
				}
			}
		}

		state.metronome_current.retain(|(pos, e)| {
			let click_len = match e {
				ClickType::Downbeat => metronome_samples.downbeat.len(),
				ClickType::Beat => metronome_samples.beat.len(),
			};
			*pos < click_len
		});

		if !state.playing {
			return;
		}

		let volume = state.volume;
		let has_resampler = state.resampler.is_some();

		if !has_resampler {
			let samples_len = state.audio.samples().len();
			let mut stop_playing = false;

			for sample in data.iter_mut() {
				let pos = state.position as usize;
				if pos < samples_len {
					let sample_val = state.audio.samples()[pos];
					*sample += sample_val * volume;
					state.position += speed;
				} else {
					stop_playing = true;
				}
			}

			if stop_playing {
				state.playing = false;
			}
		} else {
			if state.resample_buffer_pos >= state.resample_buffer.len() {
				state.resample_buffer.clear();
				state.resample_buffer_pos = 0;

				let resampler_arc = state.resampler.as_ref().unwrap().clone();
				let nbr_input_frames = resampler_arc.lock().unwrap().input_frames_next();

				let (input_slice_copy, nbr_input_frames_actual) = {
					let samples = state.audio.samples();
					let input_end =
						(state.input_position + nbr_input_frames * channels).min(samples.len());
					let nbr_input_frames_actual = (input_end - state.input_position) / channels;

					if nbr_input_frames_actual > 0 {
						(
							samples[state.input_position..input_end].to_vec(),
							nbr_input_frames_actual,
						)
					} else {
						(vec![], nbr_input_frames_actual)
					}
				};

				if nbr_input_frames_actual > 0 {
					let input_adapter =
						InterleavedSlice::new(&input_slice_copy, channels, nbr_input_frames_actual)
							.unwrap();

					let f_ratio = sample_rate as f64 / original_sample_rate as f64;
					let effective_ratio = f_ratio / speed;
					let est_output_frames =
						(nbr_input_frames_actual as f64 * effective_ratio * 1.5) as usize;
					state
						.resample_buffer
						.resize(est_output_frames * channels, 0.0);

					let mut output_adapter = InterleavedSlice::new_mut(
						&mut state.resample_buffer,
						channels,
						est_output_frames,
					)
					.unwrap();

					let mut resampler = resampler_arc.lock().unwrap();
					match resampler.process_into_buffer(&input_adapter, &mut output_adapter, None) {
						Ok((nbr_in_frames, nbr_out_frames)) => {
							state.input_position += nbr_in_frames * channels;
							state.resample_buffer.truncate(nbr_out_frames * channels);
						},
						Err(e) => {
							// TODO: Replace with a proper UI error?
							eprintln!("Resampling error occurred: {}", e);
							state.resample_buffer.clear();
							state.playing = false;
						},
					}
				} else {
					state.playing = false;
				}
			}

			for sample in data.iter_mut() {
				if state.resample_buffer_pos < state.resample_buffer.len() {
					*sample += state.resample_buffer[state.resample_buffer_pos] * volume;
					state.resample_buffer_pos += 1;
					state.position += source_samples_per_output_sample;
				} else {
					state.playing = false;
				}
			}
		}
	}
}
