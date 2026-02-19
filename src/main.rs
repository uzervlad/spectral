#![windows_subsystem = "windows"]

use egui::IconData;
use spectral::app::SpectralApp;

fn main() -> eframe::Result {
	let (icon, w, h) = {
		let bytes = include_bytes!("./assets/spectral_128.png");
		let image = image::load_from_memory(bytes)
			.expect("failed to load icon")
			.into_rgba8();
		let (w, h) = image.dimensions();
		let rgba = image.into_raw();
		(rgba, w, h)
	};

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([1280., 720.])
			.with_min_inner_size([800., 600.])
			.with_icon(IconData {
				rgba: icon,
				width: w,
				height: h,
			}),
		..Default::default()
	};

	eframe::run_native(
		"Spectral",
		options,
		Box::new(|_| Ok(Box::new(SpectralApp::new()))),
	)
}
