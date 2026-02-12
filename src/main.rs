use spectral::app::SpectralApp;

fn main() -> eframe::Result {
	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([1280., 720.])
			.with_min_inner_size([800., 600.]),
		..Default::default()
	};

	eframe::run_native(
		"Spectral",
		options,
		Box::new(|_| Ok(Box::new(SpectralApp::new()))),
	)
}
