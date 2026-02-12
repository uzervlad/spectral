use egui::Color32;

#[derive(Clone)]
pub struct TimingPoint {
	id: egui::Id,
	pub offset: f64,
	pub bpm: f64,
	pub signature: (u32, u32),
}

impl TimingPoint {
	pub fn new(offset: f64, bpm: f64) -> Self {
		Self {
			id: egui::Id::new(rand::random::<u128>()),
			offset,
			bpm,
			signature: (4, 4),
		}
	}

	pub fn id(&self) -> egui::Id {
		self.id
	}

	pub fn ms_per_beat(&self) -> f64 {
		60000. / self.bpm
	}
}

pub enum SnapDivision {
	Downbeat,
	Beat,
	Half,
	Third,
	Quarter,
	Sixth,
	Eighth,
	Twelfth,
	Sixteenth,
	Other,
}

impl SnapDivision {
	pub fn color(&self) -> Color32 {
		let color = match self {
			Self::Downbeat | Self::Beat => Color32::from_gray(230),
			Self::Half => Color32::RED,
			Self::Third => Color32::PURPLE,
			Self::Quarter => Color32::CYAN,
			Self::Sixth => Color32::GOLD,
			Self::Eighth => Color32::LIGHT_YELLOW,
			Self::Twelfth => Color32::ORANGE,
			Self::Sixteenth => Color32::PURPLE,
			_ => Color32::from_rgb(179, 217, 68),
		};

		let (r, g, b, _) = color.to_tuple();

		Color32::from_rgba_unmultiplied(r, g, b, 160)
	}

	pub fn height(&self) -> f32 {
		match self {
			Self::Downbeat => 1.,
			Self::Beat => 0.8,
			Self::Half => 0.65,
			Self::Third => 0.55,
			Self::Quarter => 0.48,
			Self::Sixth => 0.42,
			Self::Eighth => 0.38,
			Self::Twelfth => 0.34,
			Self::Sixteenth => 0.3,
			_ => 0.45,
		}
	}

	pub fn width(&self) -> f32 {
		match self {
			Self::Downbeat => 2.5,
			Self::Beat => 2.,
			Self::Half => 1.5,
			_ => 1.,
		}
	}

	pub fn from_tick(in_beat: i64, divisor: i64, in_measure: i64) -> Self {
		if in_beat == 0 {
			if in_measure == 0 {
				return Self::Downbeat;
			}
			return Self::Beat;
		}

		if (in_beat * 2) % divisor == 0 {
			return Self::Half;
		}
		if (in_beat * 3) % divisor == 0 {
			return Self::Third;
		}
		if (in_beat * 4) % divisor == 0 {
			return Self::Quarter;
		}
		if (in_beat * 6) % divisor == 0 {
			return Self::Sixth;
		}
		if (in_beat * 8) % divisor == 0 {
			return Self::Eighth;
		}
		if (in_beat * 12) % divisor == 0 {
			return Self::Twelfth;
		}
		if (in_beat * 16) % divisor == 0 {
			return Self::Sixteenth;
		}
		Self::Other
	}
}
