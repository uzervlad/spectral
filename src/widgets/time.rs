use crate::util::format_time;

pub struct TimeInput;

impl TimeInput {
	pub fn ui(ui: &mut egui::Ui, time: &mut f64, id: egui::Id) -> egui::Response {
		let editing = ui.memory(|m| m.data.get_temp::<bool>(id)).unwrap_or(false);

		let focus = ui
			.memory(|m| m.data.get_temp::<bool>(id.with("focus")))
			.unwrap_or(false);

		if editing {
			let response = ui.add(
				egui::DragValue::new(time)
					.speed(1.0)
					.range(0.0..=f64::MAX)
					.max_decimals(0)
					.suffix(" ms"),
			);

			if focus {
				ui.memory_mut(|m| {
					m.request_focus(response.id);
					m.data.remove_temp::<bool>(id.with("focus"));
				});
			}

			if response.lost_focus()
				|| ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape))
			{
				ui.memory_mut(|m| m.data.remove_temp::<bool>(id));
			}

			response
		} else {
			let text = format_time(*time);

			let response = ui.add(
				egui::Label::new(text)
					.sense(egui::Sense::click_and_drag())
					.selectable(false),
			);

			if response.hovered() {
				ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
			}

			if response.clicked() {
				ui.memory_mut(|m| m.data.insert_temp(id, true));
				ui.memory_mut(|m| m.data.insert_temp(id.with("focus"), true));
			}

			if response.dragged() {
				let delta_x = response.drag_delta().x;
				*time = (*time + delta_x as f64).max(0.);
			}

			response
		}
	}
}
