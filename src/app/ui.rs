use egui::{Color32, Pos2, Rect, Stroke, Ui};

use crate::{app::{SpectralApp, TimingMode}, util::format_time};

impl SpectralApp {
	pub fn draw_ruler(&self, ui: &mut Ui, rect: Rect) {
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

		let interval = [
			100., 200., 500., 1000., 2000., 5000., 10000., 15000., 30000., 60000.,
		]
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
					[Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
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

	pub fn draw_frequency_axis(&self, ui: &mut Ui, rect: Rect) {
		let painter = ui.painter_at(rect);

		painter.rect_filled(rect, 0., Color32::from_gray(27));

		if let Some(data) = &self.audio_data {
			let max_freq = data.sample_rate as f32 / 2.;

			let freqs = [
				2000., 4000., 6000., 8000., 10000., 12000., 14000., 16000., 18000., 20000.,
			]
			.into_iter()
			.filter(|&f| f <= max_freq);

			for freq in freqs {
				let y = rect.bottom() - (freq / max_freq) * rect.height();

				if y >= rect.top() && y <= rect.bottom() {
					painter.line_segment(
						[Pos2::new(rect.right() - 4.0, y), Pos2::new(rect.right(), y)],
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

	pub fn draw_timeline(&mut self, ui: &mut Ui, rect: Rect) {
		self.draw_spectrogram(ui, rect);

		let _painter = ui.painter_at(rect);

		self.draw_beat_ticks(ui, rect);
		self.draw_timing_points(ui, rect);
		self.draw_playhead(ui, rect);
		self.draw_cursor(ui, rect);
	}

	pub fn draw_spectrogram(&mut self, ui: &mut Ui, rect: Rect) {
		let painter = ui.painter_at(rect);

		painter.rect_filled(rect, 0., Color32::from_gray(15));

		if self.audio_data.is_none() {
			return;
		}

		let width = (rect.width() as usize).max(100);
		let height = (rect.height() as usize).max(100);

		if let Some(texture) = self.generate_spectrogram(ui.ctx(), width, height) {
			let uv = Rect::from_min_max(Pos2::new(0., 0.), Pos2::new(1., 1.));
			painter.image(texture.id(), rect, uv, Color32::WHITE);
		}
	}

	pub fn draw_cursor(&mut self, ui: &mut Ui, rect: Rect) {
		let ms = if self.snap_to_tick {
			self.snap_ms
		} else {
			self.hover_ms
		};

		if let Some(ms) = ms {
			let x = self.timeline.ms_to_x(ms, rect);

			ui.painter_at(rect).line_segment(
				[Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
				Stroke::new(1., Color32::from_gray(170)),
			);
		}
	}

	pub fn draw_playhead(&self, ui: &mut Ui, rect: Rect) {
		let x = self
			.timeline
			.ms_to_x(self.audio_player.get_position_ms(), rect);

		let color = Color32::from_rgb(102, 255, 204);

		if x >= rect.left() && x <= rect.right() {
			ui.painter_at(rect).line_segment(
				[Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
				Stroke::new(2., color),
			);

			let tri = vec![
				Pos2::new(x - 8., rect.top()),
				Pos2::new(x + 8., rect.top()),
				Pos2::new(x, rect.top() + 12.),
			];

			ui.painter_at(rect)
				.add(egui::Shape::convex_polygon(tri, color, Stroke::NONE));
		}
	}

	pub fn draw_timing_points(&self, ui: &mut Ui, rect: Rect) {
		for tp in self.timing_points.read().unwrap().iter() {
			let x = self.timeline.ms_to_x(tp.offset, rect);
			if x >= rect.left() && x <= rect.right() {
				ui.painter_at(rect).line_segment(
					[Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
					Stroke::new(2., Color32::GOLD),
				);

				let tri = vec![
					Pos2::new(x - 8., rect.top()),
					Pos2::new(x + 8., rect.top()),
					Pos2::new(x, rect.top() + 12.),
				];

				ui.painter_at(rect).add(egui::Shape::convex_polygon(
					tri,
					Color32::GOLD,
					Stroke::NONE,
				));
			}
		}

		match self.timing_mode {
			TimingMode::SelectedStart { start } => {
				let x = self.timeline.ms_to_x(start, rect);

				ui.painter_at(rect).line_segment(
					[Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
					Stroke::new(2., Color32::CYAN),
				);

				ui.painter_at(rect).text(
					Pos2::new(x, rect.top() + 5.),
					egui::Align2::CENTER_TOP,
					"START",
					egui::FontId::proportional(9.),
					Color32::CYAN,
				);
			},
			TimingMode::Idle => {},
		}
	}

	pub fn draw_beat_ticks(&mut self, ui: &mut Ui, rect: Rect) {
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
					Stroke::new(snap.width(), snap.color()),
				);
			}
		}
	}
}