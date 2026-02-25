use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Ui};

use crate::app::{SpectralApp, TimingMode};
use crate::colors::{
	COLOR_AXES_STROKE, COLOR_AXES_TEXT, COLOR_CURSOR, COLOR_PLAYHEAD, COLOR_SCROLL,
	COLOR_SCROLL_OUTLINE, COLOR_SCROLL_OUTLINE_HOVER, COLOR_SCROLL_THUMB, COLOR_SCROLL_THUMB_HOVER,
	COLOR_TIMING_POINT, COLOR_TIMING_POINT_TEMPORARY,
};
use crate::util::format_time;

impl SpectralApp {
	pub fn draw_ruler(&self, ui: &mut Ui, rect: Rect) {
		let painter = ui.painter_at(rect);

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
					Stroke::new(1., COLOR_AXES_STROKE),
				);

				let time_text = format_time(ms);
				ui.painter().text(
					Pos2::new(x + 3., rect.center().y),
					egui::Align2::LEFT_CENTER,
					time_text,
					egui::FontId::proportional(10.),
					COLOR_AXES_TEXT,
				);
			}
		}
	}

	pub fn draw_frequency_axis(&self, ui: &mut Ui, rect: Rect) {
		let painter = ui.painter_at(rect);

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
						Stroke::new(1., COLOR_AXES_STROKE),
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
						COLOR_AXES_TEXT,
					);
				}
			}
		}
	}

	pub fn draw_timeline(&mut self, ui: &mut Ui, rect: Rect) {
		self.draw_spectrogram(ui, rect);

		let painter = ui.painter_at(rect);

		self.draw_beat_ticks(ui, rect);
		self.draw_timing_points(ui, rect);
		self.draw_playhead(ui, rect);
		self.draw_cursor(ui, rect);

		if self.audio_loading {
			painter.rect_filled(rect, 0., Color32::from_rgba_premultiplied(0, 0, 0, 120));
			painter.text(
				rect.center(),
				egui::Align2::CENTER_CENTER,
				"Loading audio...",
				egui::FontId::proportional(14.),
				Color32::WHITE,
			);
		}
	}

	pub fn draw_spectrogram(&mut self, ui: &mut Ui, rect: Rect) {
		let painter = ui.painter_at(rect);

		if self.audio_data.is_none() {
			return;
		}

		if let Some((texture, x_from, x_to)) = self.generate_spectrogram(ui.ctx(), rect.width() as _, rect.height() as _) {
			let uv = Rect::from_min_max(Pos2::new(x_from as _, 0.), Pos2::new(x_to as _, 1.));
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
				Stroke::new(1., COLOR_CURSOR),
			);
		}
	}

	pub fn draw_playhead(&self, ui: &mut Ui, rect: Rect) {
		let x = self
			.timeline
			.ms_to_x(self.audio_player.get_position_ms(), rect);

		if x >= rect.left() && x <= rect.right() {
			ui.painter_at(rect).line_segment(
				[Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
				Stroke::new(2., COLOR_PLAYHEAD),
			);

			let tri = vec![
				Pos2::new(x - 8., rect.top()),
				Pos2::new(x + 8., rect.top()),
				Pos2::new(x, rect.top() + 12.),
			];

			ui.painter_at(rect).add(egui::Shape::convex_polygon(
				tri,
				COLOR_PLAYHEAD,
				Stroke::NONE,
			));
		}
	}

	pub fn draw_timing_points(&self, ui: &mut Ui, rect: Rect) {
		for tp in self.timing_points.read().unwrap().iter() {
			let x = self.timeline.ms_to_x(tp.offset, rect);
			if x >= rect.left() && x <= rect.right() {
				ui.painter_at(rect).line_segment(
					[Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
					Stroke::new(2., COLOR_TIMING_POINT),
				);

				let tri = vec![
					Pos2::new(x - 8., rect.top()),
					Pos2::new(x + 8., rect.top()),
					Pos2::new(x, rect.top() + 12.),
				];

				ui.painter_at(rect).add(egui::Shape::convex_polygon(
					tri,
					COLOR_TIMING_POINT,
					Stroke::NONE,
				));
			}
		}

		match self.timing_mode {
			TimingMode::SelectedStart { start } => {
				let x = self.timeline.ms_to_x(start, rect);

				ui.painter_at(rect).line_segment(
					[Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
					Stroke::new(2., COLOR_TIMING_POINT_TEMPORARY),
				);

				ui.painter_at(rect).text(
					Pos2::new(x, rect.top() + 5.),
					egui::Align2::CENTER_TOP,
					"START",
					egui::FontId::proportional(9.),
					COLOR_TIMING_POINT_TEMPORARY,
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
				if self.snap_to_tick
					&& let Some(mx) = mouse_x
				{
					let dist = (x - mx).abs();
					if dist < closest_dist {
						closest_dist = dist;
						self.snap_ms = Some(tick_ms);
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

	pub fn draw_scrollbar(&mut self, ui: &mut Ui, rect: Rect) {
		let bar_response = ui.allocate_rect(rect, Sense::click());

		let painter = ui.painter_at(rect);
		painter.rect_filled(rect, 0., COLOR_SCROLL);

		let Some(ref audio_data) = self.audio_data else {
			return;
		};

		let playhead_x = rect.left()
			+ (self.audio_player.get_position_ms() / audio_data.duration) as f32 * rect.width();

		painter.line_segment(
			[
				Pos2::new(playhead_x, rect.top()),
				Pos2::new(playhead_x, rect.bottom()),
			],
			Stroke::new(1., COLOR_PLAYHEAD),
		);

		for tp in self.timing_points.read().unwrap().iter() {
			let tp_x = rect.left() + (tp.offset / audio_data.duration) as f32 * rect.width();

			painter.line_segment(
				[Pos2::new(tp_x, rect.top()), Pos2::new(tp_x, rect.bottom())],
				Stroke::new(1., COLOR_TIMING_POINT),
			);
		}

		let (start, end) = self.timeline.visible_range(rect.width());

		let start = start / audio_data.duration;
		let end = end / audio_data.duration;

		let thumb_rect = Rect::from_min_max(
			Pos2::new(rect.left() + rect.width() * start as f32, rect.top()),
			Pos2::new(rect.left() + rect.width() * end as f32, rect.bottom()),
		);

		let thumb_response = ui.allocate_rect(thumb_rect, Sense::drag());

		painter.rect(
			thumb_rect,
			0.,
			if thumb_response.hovered() {
				COLOR_SCROLL_THUMB_HOVER
			} else {
				COLOR_SCROLL_THUMB
			},
			Stroke::new(
				1.,
				if thumb_response.hovered() {
					COLOR_SCROLL_OUTLINE_HOVER
				} else {
					COLOR_SCROLL_OUTLINE
				},
			),
			StrokeKind::Inside,
		);

		if thumb_response.dragged() {
			let delta_pixels = thumb_response.drag_delta().x as f64;
			let delta_ms = delta_pixels * audio_data.duration / rect.width() as f64;
			self.timeline
				.scroll_ms(delta_ms, audio_data.duration, rect.width());
		}

		if bar_response.clicked()
			&& !thumb_response.hovered()
			&& let Some(pos) = bar_response.interact_pointer_pos()
		{
			let clicked_ms = ((pos.x - rect.left()) / rect.width()) as f64 * audio_data.duration;
			self.timeline
				.scroll_to(clicked_ms, audio_data.duration, rect.width());
		}
	}
}
