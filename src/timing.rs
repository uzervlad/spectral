use egui::Color32;

use crate::colors::{
	COLOR_SNAP_BEAT, COLOR_SNAP_EIGHTH, COLOR_SNAP_HALF, COLOR_SNAP_OTHER, COLOR_SNAP_QUARTER,
	COLOR_SNAP_SIXTEENTH, COLOR_SNAP_SIXTH, COLOR_SNAP_THIRD, COLOR_SNAP_TWELFTH,
};

#[derive(Clone, Copy, PartialEq)]
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
			Self::Downbeat | Self::Beat => COLOR_SNAP_BEAT,
			Self::Half => COLOR_SNAP_HALF,
			Self::Third => COLOR_SNAP_THIRD,
			Self::Quarter => COLOR_SNAP_QUARTER,
			Self::Sixth => COLOR_SNAP_SIXTH,
			Self::Eighth => COLOR_SNAP_EIGHTH,
			Self::Twelfth => COLOR_SNAP_TWELFTH,
			Self::Sixteenth => COLOR_SNAP_SIXTEENTH,
			_ => COLOR_SNAP_OTHER,
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
