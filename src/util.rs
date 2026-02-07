use egui::Color32;

pub fn format_time(ms: f64) -> String {
	let ms = ms.max(0.) as i64;
	let total_seconds = ms / 1000;
	let minutes = total_seconds / 60;
	let seconds = total_seconds % 60;
	let millis = ms % 1000;
	
	format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
}

pub fn magma_colormap(t: f32) -> Color32 {
	let t = t.clamp(0.0, 1.0);
	
	let (r, g, b) = if t < 0.2 {
		let s = t / 0.2;
		(
			0.001 + s * 0.131,
			0.0 + s * 0.025,
			0.014 + s * 0.227,
		)
	} else if t < 0.4 {
		let s = (t - 0.2) / 0.2;
		(
			0.132 + s * 0.347,
			0.025 + s * 0.041,
			0.241 + s * 0.224,
		)
	} else if t < 0.6 {
		let s = (t - 0.4) / 0.2;
		(
			0.479 + s * 0.286,
			0.066 + s * 0.159,
			0.465 + s * 0.004,
		)
	} else if t < 0.8 {
		let s = (t - 0.6) / 0.2;
		(
			0.765 + s * 0.180,
			0.225 + s * 0.339,
			0.469 - s * 0.169,
		)
	} else {
		let s = (t - 0.8) / 0.2;
		(
			0.945 + s * 0.046,
			0.564 + s * 0.360,
			0.300 + s * 0.463,
		)
	};
	
	Color32::from_rgb(
		(r.clamp(0.0, 1.0) * 255.0) as u8,
		(g.clamp(0.0, 1.0) * 255.0) as u8,
		(b.clamp(0.0, 1.0) * 255.0) as u8,
	)
}