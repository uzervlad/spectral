use std::{path::Path, sync::mpsc::{self, Receiver, Sender}, thread};

use egui::{Color32, ColorImage, FontId, Pos2, Rect, Sense, Stroke, TextFormat, TextureHandle, Ui, Vec2, text::LayoutJob};

use crate::{audio::{AudioData, AudioPlayer}, events::SpectralEvent, export::{ExportFormat, export_timing_points}, spectrogram::{CachedSpectrogram, Spectrogram}, timing::{SnapDivision, TimingPoint}, util::{format_time, magma_colormap}, widgets::{time::TimeInput, timeline::Timeline}};

enum TimingMode {
	Idle,
	SelectedStart { start: f64 },
}

pub struct SpectralApp {
	audio_data: Option<AudioData>,
	audio_player: AudioPlayer,

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

	timing_points: Vec<TimingPoint>,
}

impl SpectralApp {
	pub fn new() -> Self {
		let (event_tx, event_rx) = mpsc::channel();

		Self {
			audio_data: None,
			audio_player: AudioPlayer::new().expect("penis"),

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

			timing_points: vec![
				TimingPoint::new(100., 120.),
				TimingPoint::new(7727., 222.22),
			],
		}
	}

	fn handle_event(&mut self, event: SpectralEvent) {
		match event {
			SpectralEvent::OpenAudio { path } => {
				self.load_audio(path);
			}
		}
	}

	fn load_audio<P: AsRef<Path>>(&mut self, path: P) {
		self.audio_player.pause();

		match AudioData::load_from_file(path) {
			Ok(data) => {
				let _ = self.audio_player.load(&data);	

				self.audio_data = Some(data);
				self.cached_spectrogram = None;
				self.timing_points.clear();
				self.timeline.reset();
			},
			Err(e) => {
				eprintln!("fuck {}", e);
			}
		}
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

	fn sort_timing_points(&mut self) {
		self.timing_points.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
	}

	fn get_beat_ticks(&self, start: f64, end: f64) -> Vec<(f64, SnapDivision)> {
		let mut ticks = vec![];

		for (i, tp) in self.timing_points.iter().enumerate() {
			let ms_per_beat = tp.ms_per_beat();
			let ms_per_tick = ms_per_beat / self.snap_divisor as f64;

			// TODO: stop rendering at low zoom

			let section_end = if i + 1 < self.timing_points.len() {
				self.timing_points[i + 1].offset
			} else {
				self.audio_data
					.as_ref()
					.map(|data| data.duration)
					.unwrap_or(f64::MAX)
			};

			let tick_start = start.max(tp.offset);
			let tick_end = end.min(section_end);

			if tick_start >= tick_end { continue; }

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

	fn generate_spectrogram(&mut self, ctx: &egui::Context, width: usize, height: usize) -> Option<TextureHandle> {
		let audio = self.audio_data.as_ref()?;

		let (vis_start, vis_end) = self.timeline.visible_range(width as _);

		if let Some(cached) = &self.cached_spectrogram {
			if cached.is_valid(vis_start, vis_end, self.fft_size, self.min_db, self.max_db, width) {
				return Some(cached.texture.clone())
			}
		}

		if self.fft_size != self.spectrogram.fft_size {
			self.spectrogram = Spectrogram::new(self.fft_size);
		}

		let columns = self.spectrogram.compute_range(audio, vis_start, vis_end, width, self.min_db, self.max_db);

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

		self.cached_spectrogram = Some(CachedSpectrogram::new(texture, vis_start, vis_end, self.fft_size, self.min_db, self.max_db, width));

		Some(self.cached_spectrogram.as_ref().unwrap().texture.clone())
	}

	fn draw_ruler(&self, ui: &mut Ui, rect: Rect) {
		let painter = ui.painter_at(rect);
		painter.rect_filled(rect, 0., Color32::from_gray(27));
		painter.line_segment(
			[
				Pos2::new(rect.left(), rect.bottom()),
				Pos2::new(rect.right(), rect.bottom()),
			],
			Stroke::new(1., Color32::from_gray(65)),
		);

		let (vis_start, vis_end) = self.timeline.visible_range(rect.width());

		let vis_duration = vis_end - vis_start;
		let target_ticks = (rect.width() / 100.) as f64;

		let interval = [100., 200., 500., 1000., 2000., 5000., 10000., 15000., 30000., 60000.]
			.iter()
			.find(|&&i| vis_duration / i < target_ticks * 2.0)
			.copied()
			.unwrap_or(60000.);

		let start_tick = (vis_start / interval).floor() as i64;
		let end_tick = (vis_end / interval).ceil() as i64;

		for tick in start_tick..=end_tick {
			let ms = tick as f64 * interval;
			let x = self.timeline.ms_to_x(ms, rect);

			if x >= rect.left() && x <= rect.right() {
				painter.line_segment(
					[
						Pos2::new(x, rect.top()),
						Pos2::new(x, rect.bottom()),
					],
					Stroke::new(1., Color32::from_gray(50)),
				);

				let time_text = format_time(ms);
				ui.painter().text(
					Pos2::new(x + 3., rect.center().y),
					egui::Align2::LEFT_CENTER,
					time_text,
					egui::FontId::proportional(10.),
					Color32::from_gray(100),
				);
			}
		}
	}

	fn draw_frequency_axis(&self, ui: &mut Ui, rect: Rect) {
		let painter = ui.painter_at(rect);
		
		painter.rect_filled(rect, 0., Color32::from_gray(27));

		if let Some(data) = &self.audio_data {
			let max_freq = data.sample_rate as f32 / 2.;

			let freqs = [2000., 4000., 6000., 8000., 10000., 12000., 14000., 16000., 18000., 20000.]
				.into_iter()
				.filter(|&f| f <= max_freq);

			for freq in freqs {
				let y = rect.bottom() - (freq / max_freq) * rect.height();

				if y >= rect.top() && y <= rect.bottom() {
					painter.line_segment(
						[
							Pos2::new(rect.right() - 4.0, y),
							Pos2::new(rect.right(), y),
						],
						Stroke::new(1., Color32::from_gray(100)),
					);
					
					let label = if freq >= 1000. {
						format!("{}k", (freq / 1000.) as u16)
					} else {
						format!("{}", freq as u16)
					};
					
					painter.text(
						Pos2::new(rect.center().x, y),
						egui::Align2::CENTER_CENTER,
						&label,
						egui::FontId::proportional(11.),
						Color32::from_gray(160),
					);
				}
			}
		}
	}

	fn draw_timeline(&mut self, ui: &mut Ui, rect: Rect) {
		self.draw_spectrogram(ui, rect);

		let _painter = ui.painter_at(rect);

		self.draw_beat_ticks(ui, rect);
		self.draw_timing_points(ui, rect);
		self.draw_playhead(ui, rect);
		self.draw_cursor(ui, rect);
	}

	fn draw_spectrogram(&mut self, ui: &mut Ui, rect: Rect) {
		let painter = ui.painter_at(rect);

		painter.rect_filled(rect, 0., Color32::from_gray(15));

		if self.audio_data.is_none() { return }

		let width = (rect.width() as usize).max(100);
		let height = (rect.height() as usize).max(100);

		if let Some(texture) = self.generate_spectrogram(ui.ctx(), width, height) {
			let uv = Rect::from_min_max(Pos2::new(0., 0.), Pos2::new(1., 1.));
			painter.image(texture.id(), rect, uv, Color32::WHITE);
		}
	}

	fn draw_cursor(&mut self, ui: &mut Ui, rect: Rect) {
		let ms = if self.snap_to_tick { self.snap_ms } else { self.hover_ms };

		if let Some(ms) = ms {
			let x = self.timeline.ms_to_x(ms, rect);

			ui.painter_at(rect).line_segment(
				[
					Pos2::new(x, rect.top()),
					Pos2::new(x, rect.bottom()),
				],
				Stroke::new(1., Color32::from_gray(170)),
			);
		}
	}

	fn draw_playhead(&self, ui: &mut Ui, rect: Rect) {
		let x = self.timeline.ms_to_x(self.audio_player.get_position_ms(), rect);

		let color = Color32::from_rgb(102, 255, 204);

		if x >= rect.left() && x <= rect.right() {
			ui.painter_at(rect).line_segment(
				[
					Pos2::new(x, rect.top()),
					Pos2::new(x, rect.bottom()),
				],
				Stroke::new(2., color),
			);

			let tri = vec![
				Pos2::new(x - 8., rect.top()),
				Pos2::new(x + 8., rect.top()),
				Pos2::new(x, rect.top() + 12.),
			];

			ui.painter_at(rect).add(egui::Shape::convex_polygon(tri, color, Stroke::NONE));
		}
	}

	fn draw_timing_points(&self, ui: &mut Ui, rect: Rect) {
		for tp in self.timing_points.iter() {
			let x = self.timeline.ms_to_x(tp.offset, rect);
			if x >= rect.left() && x <= rect.right() {
				ui.painter_at(rect).line_segment(
					[
						Pos2::new(x, rect.top()),
						Pos2::new(x, rect.bottom()),
					],
					Stroke::new(2., Color32::GOLD),
				);

				let tri = vec![
					Pos2::new(x - 8., rect.top()),
					Pos2::new(x + 8., rect.top()),
					Pos2::new(x, rect.top() + 12.),
				];

				ui.painter_at(rect).add(egui::Shape::convex_polygon(tri, Color32::GOLD, Stroke::NONE));
			}
		}
	
		match self.timing_mode {
			TimingMode::SelectedStart { start } => {
				let x = self.timeline.ms_to_x(start, rect);

				ui.painter_at(rect).line_segment(
					[
						Pos2::new(x, rect.top()),
						Pos2::new(x, rect.bottom()),
					],
					Stroke::new(2., Color32::CYAN)
				);

				ui.painter_at(rect).text(
					Pos2::new(x, rect.top() + 5.),
					egui::Align2::CENTER_TOP,
					"START",
					egui::FontId::proportional(9.),
					Color32::CYAN,
				);
			},
			TimingMode::Idle => {}
		}
	}

	fn draw_beat_ticks(&mut self, ui: &mut Ui, rect: Rect) {
		let (start, end) = self.timeline.visible_range(rect.width());
		let ticks = self.get_beat_ticks(start, end);

		let mouse_x = ui.input(|i| i.pointer.hover_pos()).map(|p| p.x);

		if self.snap_to_tick {
			self.snap_ms = None;
		}

		let mut closest_dist = f32::MAX;

		for (tick_ms, snap) in ticks {
			let x = self.timeline.ms_to_x(tick_ms, rect);

			if x >= rect.left() && x <= rect.right() {
				if self.snap_to_tick {
					if let Some(mx) = mouse_x {
						let dist = (x - mx).abs();
						if dist < closest_dist {
							closest_dist = dist;
							self.snap_ms = Some(tick_ms);
						}
					}
				}

				let height = snap.height() * rect.height();

				ui.painter_at(rect).line_segment(
					[
						Pos2::new(x, rect.bottom()),
						Pos2::new(x, rect.bottom() - height),
					],
					Stroke::new(snap.width(), snap.color())
				);
			}
		}
	}

	fn handle_timeline_input(&mut self, ui: &mut Ui, rect: Rect, response: &egui::Response) {
		let duration = self.audio_data.as_ref().map(|data| data.duration).unwrap_or(0.);

		let mouse_pos = ui.input(|i| i.pointer.hover_pos());

		if response.hovered() {
			let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
			let alt_held = ui.input(|i| i.modifiers.alt);

			if scroll_delta.y.abs() > 0. {
				if alt_held {
					if let Some(pos) = mouse_pos {
						let focus = self.timeline.x_to_ms(pos.x, rect);
						self.timeline.zoom(scroll_delta.y as f64, focus, duration, rect.width());
					}
				} else {
					let scroll_speed = 1.0;
					self.timeline.scroll(-scroll_delta.y as f64 * scroll_speed, duration, rect.width());
				}
			}

			self.hover_ms = mouse_pos.map(|p| self.timeline.x_to_ms(p.x, rect));
		} else {
			self.hover_ms = None;
		}

		if response.dragged_by(egui::PointerButton::Middle) {
			let delta = response.drag_delta();
			self.timeline.scroll(-delta.x as f64, duration, rect.width());
		}

		if response.clicked() {
			let ms = if self.snap_to_tick { self.snap_ms } else { self.hover_ms };

			if let Some(click_ms) = ms {
				match self.timing_mode {
					TimingMode::Idle => {
						self.timing_mode = TimingMode::SelectedStart { start: click_ms };
					},
					TimingMode::SelectedStart { start } => {
						let bpm = ({
							let delta = (click_ms - start).abs();
							if delta > 0. { 60000. / delta } else { 120. }
						} * 100.).round() / 100.;

						let offset = start.min(click_ms);
						let tp = TimingPoint::new(offset, bpm);
						self.timing_points.push(tp);
						self.sort_timing_points();

						self.timing_mode = TimingMode::Idle;
					}
				}
			}
		}

		if response.secondary_clicked() {
			let ms = if self.snap_to_tick { self.snap_ms } else { self.hover_ms };

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

		if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
			self.audio_player.play_pause();
		}

		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
			self.timing_mode = TimingMode::Idle;
		}

		if self.audio_player.is_playing() {
			ctx.request_repaint();
		}

		self.snap_to_tick = ctx.input(|i| i.modifiers.shift_only());

		/* Top panel */
		egui::TopBottomPanel::top("top").show(ctx, |ui| {
			ui.horizontal(|ui| {
				if ui.button("Open audio").clicked() {
					self.request_open_audio();
				}

				ui.separator();

				if ui.button(if self.audio_player.is_playing() { "Pause" } else { "Play" }).clicked() {
					self.audio_player.play_pause();
				}

				ui.separator();

				ui.label("Volume:");

				let mut volume = self.audio_player.get_volume();
				if ui.add(
					egui::Slider::new(&mut volume, 0.0..=1.0)
						.show_value(false)
						.fixed_decimals(2)
				).changed() {
					self.audio_player.set_volume(volume);
				}

				ui.label(format!("{:.0}%", volume * 100.));

				ui.separator();

				ui.label("FFT size");

				egui::ComboBox::from_id_salt("fft_size")
					.selected_text(format!("{}", self.fft_size))
					.show_ui(ui, |ui| {
						for &v in [512, 1024, 2048, 4096].iter() {
							ui.selectable_value(&mut self.fft_size, v, format!("{}", v));
						}
					});

				ui.separator();

				ui.label("dB range");

				ui.add(
					egui_double_slider::DoubleSlider::new(
						&mut self.min_db,
						&mut self.max_db,
						-120.0..=0.0
					)
						.width(150.)
						.separation_distance(5.)
				);

				let db_label = ui.add(
					egui::Label::new(format!("{:.1}..{:.1}", self.min_db, self.max_db))
						.sense(egui::Sense::click())
						.selectable(false)
				);

				if db_label.hovered() {
					ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
				}

				if db_label.double_clicked() {
					self.min_db = -80.;
					self.max_db = 0.;
				}

				ui.separator();

				ui.label("Beat Snap Divisor");

				ui.add(
					egui::Slider::new(&mut self.snap_divisor, 1..=16)
						.show_value(false)
						.max_decimals(0)
						.step_by(1.)
				);

				ui.label(format!("1 / {:.0}", self.snap_divisor));

				ui.separator();

				ui.menu_button("Export", |ui| {
					ui.set_min_width(200.);

					for &fmt in ExportFormat::list() {
						if ui.button(format!("{}", fmt)).clicked() {
							export_timing_points(self.timing_points.clone(), fmt);
							ui.close();
						}
					}
				});
			});
		});

		/* Timing points */
		egui::SidePanel::right("timing_points")
			.min_width(300.)
			.show(ctx, |ui| {
				ui.heading("Timing points");
				ui.separator();

				egui::ScrollArea::vertical().show(ui, |ui| {
					let mut timing_point_delete = None;
					let mut resort_timing_points = false;

					for (i, timing_point) in self.timing_points.iter_mut().enumerate() {
						// TODO: selection?
						let frame = egui::Frame::new()
							.fill(Color32::TRANSPARENT)
							.inner_margin(4.);

						frame.show(ui, |ui| {
							ui.vertical(|ui| {
								ui.horizontal(|ui| {
									ui.label(format!("#{}", i+1));

									if ui.small_button("🗑").clicked() {
										timing_point_delete = Some(i);
									}

									ui.label("@");

									let id = timing_point.id();
									resort_timing_points |= TimeInput::ui(ui, &mut timing_point.offset, id);
								});

								ui.horizontal(|ui| {
									ui.label("BPM:");
									ui.add(
										egui::DragValue::new(&mut timing_point.bpm)
											.speed(0.01)
											.range(1.0..=999.0)
											.suffix("BPM")
									);
								});

								ui.horizontal(|ui| {
									ui.label("Signature:");
									let (mut n, mut m) = timing_point.signature;

									ui.add(egui::DragValue::new(&mut n).range(1..=16));
									ui.label("/");
									ui.add(egui::DragValue::new(&mut m).range(1..=16));

									timing_point.signature = (n, m);
								});
							});
						});

						ui.separator();
					}

					if let Some(idx) = timing_point_delete {
						self.timing_points.remove(idx);
					}

					if resort_timing_points {
						self.sort_timing_points();
					}
				});
			});

		/* Main contents */
		egui::CentralPanel::default().show(ctx, |ui| {
			let available = ui.available_rect_before_wrap();

			let ruler_height = 28.;
			let freq_axis_width = 40.;
			let timeline_height = (ui.available_height() - 80.).clamp(150., 600.);

			let ruler_rect = Rect::from_min_size(
				Pos2::new(available.left() + freq_axis_width, available.top()),
				Vec2::new(available.width() - freq_axis_width, ruler_height),
			);
			self.draw_ruler(ui, ruler_rect);

			let freq_rect = Rect::from_min_size(
				Pos2::new(available.left(), available.top() + ruler_height),
				Vec2::new(freq_axis_width, timeline_height - ruler_height),
			);
			self.draw_frequency_axis(ui, freq_rect);

			let timeline_rect = Rect::from_min_max(
				Pos2::new(available.left() + freq_axis_width, available.top() + ruler_height),
				Pos2::new(available.max.x, available.top() + timeline_height),
			);

			let timeline_response = ui.allocate_rect(timeline_rect, Sense::click_and_drag());
			self.handle_timeline_input(ui, timeline_rect, &timeline_response);

			self.draw_timeline(ui, timeline_rect);

			let font = FontId::proportional(12.);
			let highlight = Color32::from_rgb(50, 170, 255);
			let mut job = LayoutJob::default();

			macro_rules! add_text {
				($text:literal) => {
					job.append($text, 0., TextFormat { font_id: font.clone(), ..Default::default() });
				};
				($text:literal, true) => {
					job.append($text, 0., TextFormat {
						font_id: font.clone(),
						color: highlight,
						..Default::default()
					});
				};
				($(($text:literal $(, $hl:tt)?)),+$(,)?) => {
					$(
						add_text!($text $(, $hl)?);
					)+
				};
			}

			add_text!(
				("Scroll", true),
				(" or "),
				("drag with Mouse Wheel", true),
				(" to move the timeline\n"),

				("Alt+Scroll", true),
				(" to zoom in/out\n"),

				("Click", true),
				(" to start a new timing section. "),
				("Click again", true),
				(" to select the next beat.\n"),

				("Right Click", true),
				(" to seek"),
			);

			ui.add(egui::Label::new(job).selectable(false));

			/* Uncomment to show egui settings, including styling */
			// egui::ScrollArea::vertical().show(ui, |ui| {
			// 	ctx.settings_ui(ui);
			// })
		});
	}
}