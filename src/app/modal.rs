use crate::app::SpectralApp;

pub struct ResultModalData {
	id: egui::Id,
	message: String,
}

impl ResultModalData {
	pub fn new(id: u128, message: String) -> Self {
		Self {
			id: egui::Id::new(id),
			message,
		}
	}
}

impl SpectralApp {
	pub fn draw_result_modal(&mut self, ctx: &egui::Context) {
		if let Some(data) = &self.result_data {
			let response = egui::Modal::new(data.id).show(ctx, |ui| ui.label(&data.message));

			if response.should_close() {
				self.result_data = None;
			}
		}
	}
}
