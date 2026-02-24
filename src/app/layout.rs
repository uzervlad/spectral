use egui::text::LayoutJob;
use egui::{Color32, FontId, Pos2, Rect, Sense, TextFormat, Vec2};

use crate::app::SpectralApp;
use crate::app::history::EditHistoryEntry;
use crate::colors::COLOR_TEXT_HIGHLIGHT;
use crate::export::{ExportFormat, export_timing_points};
use crate::spectrogram::colors::Colormap;
use crate::widgets::time::TimeInput;

impl SpectralApp {
	pub fn draw_top_panel(&mut self, ctx: &egui::Context) {
		egui::TopBottomPanel::top("top").show(ctx, |ui| {
			ui.horizontal(|ui| {
				if ui.button("Open audio").clicked() {
					self.request_open_audio();
				}

				ui.separator();

				if ui
					.button(if self.audio_player.is_playing() {
						"Pause"
					} else {
						"Play"
					})
					.clicked()
				{
					self.audio_player.play_pause();
				}

				ui.separator();

				if ui
					.add_enabled(self.history.can_undo(), egui::Button::new("Undo"))
					.on_hover_text(format!(
						"Undo: {}",
						self.history
							.peek_undo()
							.map(|e| e.to_string())
							.unwrap_or_default()
					))
					.clicked() && let Some(entry) = self.history.undo()
				{
					self.undo(entry);
				}

				if ui
					.add_enabled(self.history.can_redo(), egui::Button::new("Redo"))
					.on_hover_text(format!(
						"Redo: {}",
						self.history
							.peek_redo()
							.map(|e| e.to_string())
							.unwrap_or_default()
					))
					.clicked() && let Some(entry) = self.history.redo()
				{
					self.redo(entry);
				}

				ui.separator();

				ui.label("Volume:");

				let mut volume = self.audio_player.get_volume();
				if ui
					.add(
						egui::Slider::new(&mut volume, 0.0..=1.0)
							.show_value(false)
							.fixed_decimals(2),
					)
					.changed()
				{
					self.audio_player.set_volume(volume);
					self.settings.write(move |s| s.audio_volume = volume);
				}

				ui.label(format!("{:.0}%", volume * 100.));

				ui.separator();

				ui.label("Metronome volume:");

				let mut volume = self.audio_player.get_metronome_volume();
				if ui
					.add(
						egui::Slider::new(&mut volume, 0.0..=1.0)
							.show_value(false)
							.fixed_decimals(2),
					)
					.changed()
				{
					self.audio_player.set_metronome_volume(volume);
					self.settings.write(move |s| s.metronome_volume = volume);
				}

				ui.label(format!("{:.0}%", volume * 100.));

				ui.separator();

				ui.menu_button("Export", |ui| {
					ui.set_min_width(200.);

					if ui.button(format!("{}", ExportFormat::Csv)).clicked() {
						export_timing_points(
							self.timing_points.read().unwrap().clone(),
							ExportFormat::Csv,
							self.event_tx.clone(),
						);
					}

					ui.separator();

					ui.heading("Game formats");

					for &fmt in ExportFormat::game_formats() {
						if ui.button(format!("{}", fmt)).clicked() {
							export_timing_points(
								self.timing_points.read().unwrap().clone(),
								fmt,
								self.event_tx.clone(),
							);
							ui.close();
						}
					}
				});
			});
		});
	}

	pub fn draw_timing_points_panel(&mut self, ctx: &egui::Context) {
		egui::SidePanel::right("timing_points")
			.min_width(300.)
			.show(ctx, |ui| {
				ui.heading("Timing points");
				ui.separator();

				ui.horizontal(|ui| {
					ui.label("Beat Snap Divisor");

					ui.add(egui::Slider::new(&mut self.snap_divisor, 1..=16).show_value(false));

					let div_label = ui.add(
						egui::Label::new(format!("1 / {:.0}", self.snap_divisor))
							.sense(egui::Sense::click())
							.selectable(false),
					);

					if div_label.hovered() {
						ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
					}

					if div_label.double_clicked() {
						self.snap_divisor = 4;
					}
				});

				ui.separator();

				egui::ScrollArea::vertical().show(ui, |ui| {
					let mut timing_point_delete = None;
					let mut resort_timing_points = false;

					for (i, timing_point) in
						self.timing_points.write().unwrap().iter_mut().enumerate()
					{
						// TODO: selection?
						let frame = egui::Frame::new()
							.fill(Color32::TRANSPARENT)
							.inner_margin(4.);

						frame.show(ui, |ui| {
							ui.vertical(|ui| {
								ui.horizontal(|ui| {
									ui.label(format!("#{}", i + 1));

									if ui.small_button("🗑").clicked() {
										timing_point_delete = Some(i);
									}

									ui.label("@");

									let id = timing_point.id();
									let mut offset = timing_point.offset;
									let response = TimeInput::ui(ui, &mut offset, id);

									if response.changed() || response.dragged() {
										if self.edited_timing_point.is_none() {
											self.edited_timing_point = Some(*timing_point);
										}
										timing_point.offset = offset;
									}

									if let Some(before) = self.edited_timing_point
										&& (response.drag_stopped() || response.lost_focus())
									{
										resort_timing_points = true;
										if before != *timing_point {
											self.history.push(
												EditHistoryEntry::ModifyTimingPoint {
													before,
													after: *timing_point,
												},
											);
										}
										self.edited_timing_point = None;
									}

									if ui
										.small_button("<")
										.on_hover_text("Move 1 beat earlier")
										.clicked() && timing_point.offset
										> timing_point.ms_per_beat()
									{
										timing_point.offset -= timing_point.ms_per_beat();
									}

									if ui
										.small_button(">")
										.on_hover_text("Move 1 beat later")
										.clicked() && timing_point.offset
										+ timing_point.ms_per_beat() < self
										.audio_data
										.as_ref()
										.map(|d| d.duration)
										.unwrap_or_default()
									{
										timing_point.offset += timing_point.ms_per_beat();
									}
								});

								ui.horizontal(|ui| {
									ui.label("BPM:");

									let mut bpm = timing_point.bpm;
									let response = ui.add(
										egui::DragValue::new(&mut bpm)
											.speed(0.01)
											.range(1.0..=999.0)
											.suffix("BPM"),
									);

									if response.changed() {
										if self.edited_timing_point.is_none() {
											self.edited_timing_point = Some(*timing_point);
										}
										timing_point.bpm = bpm;
									}

									if let Some(before) = self.edited_timing_point
										&& (response.drag_stopped() || response.lost_focus())
									{
										if before != *timing_point {
											self.history.push(
												EditHistoryEntry::ModifyTimingPoint {
													before,
													after: *timing_point,
												},
											);
										}
										self.edited_timing_point = None;
									}
								});

								ui.horizontal(|ui| {
									ui.label("Signature:");
									let (mut n, mut m) = timing_point.signature;

									let n_response =
										ui.add(egui::DragValue::new(&mut n).range(1..=16));
									ui.label("/");
									let _m_response =
										ui.add(egui::DragValue::new(&mut m).range(1..=16));

									if n_response.changed() {
										if self.edited_timing_point.is_none() {
											self.edited_timing_point = Some(*timing_point);
										}
										timing_point.signature = (n, m);
									}

									if let Some(before) = self.edited_timing_point
										&& (n_response.drag_stopped() || n_response.lost_focus())
									{
										if before != *timing_point {
											self.history.push(
												EditHistoryEntry::ModifyTimingPoint {
													before,
													after: *timing_point,
												},
											);
										}
										self.edited_timing_point = None;
									}
								});
							});
						});

						ui.separator();
					}

					if let Some(idx) = timing_point_delete {
						let mut timing_points = self.timing_points.write().unwrap();
						self.history
							.push(EditHistoryEntry::DeleteTimingPoint(timing_points[idx]));
						timing_points.remove(idx);
					}

					if resort_timing_points {
						self.sort_timing_points();
					}
				});
			});
	}

	pub fn draw_main_contents(&mut self, ctx: &egui::Context) {
		egui::CentralPanel::default().show(ctx, |ui| {
			let available = ui.available_rect_before_wrap();

			let ruler_height = 28.;
			let freq_axis_width = 40.;
			let timeline_height = (ui.available_height() - 130.).clamp(150., 600.);
			let scrollbar_height = 20.;

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
				Pos2::new(
					available.left() + freq_axis_width,
					available.top() + ruler_height,
				),
				Pos2::new(available.max.x, available.top() + timeline_height),
			);

			let scrollbar_rect = Rect::from_min_size(
				Pos2::new(available.left() + freq_axis_width, timeline_rect.bottom()),
				Vec2::new(timeline_rect.width(), scrollbar_height),
			);

			let timeline_response = ui.allocate_rect(timeline_rect, Sense::click_and_drag());
			self.handle_timeline_input(ui, timeline_rect, &timeline_response);

			self.draw_timeline(ui, timeline_rect);

			self.draw_scrollbar(ui, scrollbar_rect);

			ui.separator();

			ui.horizontal(|ui| {
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
						-120.0..=0.0,
					)
					.width(150.)
					.separation_distance(5.),
				);

				let db_label = ui.add(
					egui::Label::new(format!("{:.1}..{:.1}", self.min_db, self.max_db))
						.sense(egui::Sense::click())
						.selectable(false),
				);

				if db_label.hovered() {
					ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
				}

				if db_label.double_clicked() {
					self.min_db = -80.;
					self.max_db = 0.;
				}

				ui.separator();

				let mut colormap = self.spectrogram_colormap;

				ui.label("Colormap");

				egui::ComboBox::from_label("")
					.selected_text(format!("{:?}", colormap))
					.show_ui(ui, |ui| {
						for cm in [
							Colormap::Roseus,
							Colormap::Arcus,
							Colormap::Lavendula,
							Colormap::Magma,
						] {
							ui.selectable_value(&mut colormap, cm, format!("{:?}", cm));
						}
					});

				if self.spectrogram_colormap != colormap {
					self.spectrogram_colormap = colormap;
					self.settings.write(move |s| s.colormap = colormap);

					self.cached_spectrogram = None;
					ctx.request_repaint();
				}
			});

			ui.separator();

			let font = FontId::proportional(12.);
			let highlight = COLOR_TEXT_HIGHLIGHT;
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

			#[rustfmt::skip]
			add_text!(
				("Scroll", true),
				(" or "),
				("drag with Mouse Wheel", true),
				(" to move the timeline\n"),

				("Ctrl+Scroll", true),
				(" to zoom in/out\n"),

				("Click", true),
				(" to start a new timing section. "),
				("Click again", true),
				(" to select the next beat, or press "),
				("Escape", true),
				(" to cancel\n"),

				("Hold "),
				("Shift", true),
				(" to lock cursor onto visible ticks\n"),

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
