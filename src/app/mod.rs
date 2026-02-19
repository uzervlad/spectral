use std::env::args;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use egui::{Rect, Ui};

use crate::app::history::{EditHistory, EditHistoryEntry};
use crate::audio::{AudioData, AudioPlayer};
use crate::events::SpectralEvent;
use crate::metronome::{MetronomeState, metronome_thread};
use crate::settings::SettingsManager;
use crate::spectrogram::{CachedSpectrogram, Spectrogram};
use crate::timing::TimingPoint;
use crate::widgets::timeline::Timeline;

mod history;
mod layout;
mod spectrogram;
mod timing;
mod ui;

enum TimingMode {
	Idle,
	SelectedStart { start: f64 },
}

pub struct SpectralApp {
	audio_data: Option<AudioData>,
	audio_player: AudioPlayer,
	audio_loading: bool,
	_metronome: JoinHandle<()>,

	history: EditHistory,
	settings: Arc<SettingsManager>,

	event_rx: Receiver<SpectralEvent>,
	event_tx: Sender<SpectralEvent>,

	spectrogram: Spectrogram,
	cached_spectrogram: Option<CachedSpectrogram>,
	fft_size: usize,
	min_db: f32,
	max_db: f32,

	timeline: Timeline,
	snap_divisor: i64,
	hover_ms: Option<f64>,

	snap_to_tick: bool,
	snap_ms: Option<f64>,

	timing_mode: TimingMode,

	timing_points: Arc<RwLock<Vec<TimingPoint>>>,
	edited_timing_point: Option<TimingPoint>,
}

impl SpectralApp {
	pub fn new() -> Self {
		let (event_tx, event_rx) = mpsc::channel();

		let settings = Arc::new(SettingsManager::new());

		let audio_player = AudioPlayer::new(settings.clone()).expect("penis");
		let timing_points = Arc::new(RwLock::new(vec![
			TimingPoint::new(100., 120.),
			TimingPoint::new(7727., 222.22),
		]));

		let state = MetronomeState::from(&audio_player);
		let sink = audio_player.metronome_sink.clone();
		let _tp = timing_points.clone();
		let _metronome = thread::spawn(move || {
			metronome_thread(state, sink, _tp);
		});

		let mut _self = Self {
			audio_data: None,
			audio_player,
			audio_loading: false,
			_metronome,

			history: EditHistory::default(),
			settings,

			event_rx,
			event_tx,

			spectrogram: Spectrogram::new(2048),
			cached_spectrogram: None,
			fft_size: 2048,
			min_db: -80.,
			max_db: 0.,

			timeline: Timeline::new(),
			snap_divisor: 4,
			hover_ms: None,

			snap_to_tick: false,
			snap_ms: None,

			timing_mode: TimingMode::Idle,

			timing_points,
			edited_timing_point: None,
		};

		if let Some(arg) = args().nth(1)
			&& let Ok(path) = PathBuf::from_str(&arg)
			&& path.exists()
		{
			_self.load_audio(path);
		}

		_self
	}

	fn handle_event(&mut self, event: SpectralEvent) {
		match event {
			SpectralEvent::OpenAudio { path } => {
				self.load_audio(path);
			},
			SpectralEvent::LoadAudio { data } => {
				self.audio_loading = false;

				match data {
					Ok(data) => {
						let _ = self.audio_player.load(&data);

						self.audio_data = Some(data);
						self.cached_spectrogram = None;
						self.timing_points.write().unwrap().clear();
						self.timeline.reset();
					},
					Err(e) => {
						eprintln!("fuck {}", e);
					},
				}
			},
		}
	}

	fn load_audio(&mut self, path: PathBuf) {
		self.audio_player.pause();
		self.audio_loading = true;

		let tx = self.event_tx.clone();
		thread::spawn(move || {
			let data = AudioData::load_from_file(path);
			let _ = tx.send(SpectralEvent::LoadAudio { data });
		});
	}

	fn request_open_audio(&self) {
		let tx = self.event_tx.clone();
		thread::spawn(move || {
			if let Some(path) = rfd::FileDialog::new()
				.add_filter("Audio", &["mp3", "ogg", "flac", "wav"])
				.pick_file()
			{
				let _ = tx.send(SpectralEvent::OpenAudio { path });
			}
		});
	}

	fn handle_timeline_input(&mut self, ui: &mut Ui, rect: Rect, response: &egui::Response) {
		if self.audio_loading {
			return;
		}

		let duration = self
			.audio_data
			.as_ref()
			.map(|data| data.duration)
			.unwrap_or(0.);

		let mouse_pos = ui.input(|i| i.pointer.hover_pos());

		if response.hovered() {
			let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
			let zoom_delta = ui.input(|i| i.zoom_delta());

			if zoom_delta != 1. {
				if let Some(pos) = mouse_pos {
					let focus = self.timeline.x_to_ms(pos.x, rect);
					self.timeline
						.zoom(zoom_delta as f64, focus, duration, rect.width());
				}
			} else if scroll_delta.y.abs() > 0. {
				let scroll_speed = 1.0;
				self.timeline.scroll(
					-scroll_delta.y as f64 * scroll_speed,
					duration,
					rect.width(),
				);
			}

			self.hover_ms = mouse_pos.map(|p| self.timeline.x_to_ms(p.x, rect));
		} else {
			self.hover_ms = None;
		}

		if response.dragged_by(egui::PointerButton::Middle) {
			let delta = response.drag_delta();
			self.timeline
				.scroll(-delta.x as f64, duration, rect.width());
		}

		if response.clicked() {
			let ms = if self.snap_to_tick {
				self.snap_ms
			} else {
				self.hover_ms
			};

			if let Some(click_ms) = ms {
				match self.timing_mode {
					TimingMode::Idle => {
						self.timing_mode = TimingMode::SelectedStart { start: click_ms };
					},
					TimingMode::SelectedStart { start } => {
						let bpm = {
							let delta = (click_ms - start).abs();
							if delta > 0. { 60000. / delta } else { 120. }
						};
						let bpm = (bpm * 100.).round() / 100.;

						let offset = start.min(click_ms).round();
						let tp = TimingPoint::new(offset, bpm);
						self.history.push(EditHistoryEntry::CreateTimingPoint(tp));
						self.timing_points.write().unwrap().push(tp);
						self.sort_timing_points();

						self.timing_mode = TimingMode::Idle;
					},
				}
			}
		}

		if response.secondary_clicked() {
			let ms = if self.snap_to_tick {
				self.snap_ms
			} else {
				self.hover_ms
			};

			if let Some(click_ms) = ms {
				self.audio_player.seek_to(click_ms);
			}
		}
	}
}

impl eframe::App for SpectralApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		while let Ok(event) = self.event_rx.try_recv() {
			self.handle_event(event);
		}

		if let Some(dropped_file) = ctx.input(|i| i.raw.dropped_files.first().cloned())
			&& let Some(path) = dropped_file.path
			&& path
				.extension()
				.map(|e| e.to_str().unwrap())
				.map(|e| ["mp3", "ogg", "wav", "flac"].contains(&e))
				.unwrap_or(false)
		{
			self.handle_event(SpectralEvent::OpenAudio { path });
		}

		if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
			self.audio_player.play_pause();
		}

		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
			self.timing_mode = TimingMode::Idle;
		}

		if ctx.input_mut(|i| {
			i.consume_shortcut(&egui::KeyboardShortcut::new(
				egui::Modifiers::CTRL,
				egui::Key::Z,
			))
		}) && let Some(entry) = self.history.undo()
		{
			self.undo(entry);
		}

		if ctx.input_mut(|i| {
			i.consume_shortcut(&egui::KeyboardShortcut::new(
				egui::Modifiers::CTRL,
				egui::Key::Y,
			))
		}) && let Some(entry) = self.history.redo()
		{
			self.redo(entry);
		}

		if self.audio_player.is_playing() {
			ctx.request_repaint();
		}

		self.snap_to_tick = ctx.input(|i| i.modifiers.shift_only());

		self.draw_top_panel(ctx);
		self.draw_timing_points_panel(ctx);
		self.draw_main_contents(ctx);
	}
}
