use std::fmt::Display;
use std::fs::File;
use std::thread;

use eyre::Result;
use rfd::FileDialog;

use crate::timing::TimingPoint;

mod osu;

trait ApplyExportFormat {
	fn apply_format(self, fmt: ExportFormat) -> Self;
}

#[derive(Clone, Copy)]
pub enum ExportFormat {
	Osu,
}

impl ApplyExportFormat for FileDialog {
	fn apply_format(self, fmt: ExportFormat) -> Self {
		match fmt {
			ExportFormat::Osu => self.add_filter("osu! beatmap", &["osu"]),
		}
	}
}

impl Display for ExportFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			ExportFormat::Osu => "osu! (.osu)",
		})
	}
}

impl ExportFormat {
	pub fn list() -> &'static [Self] {
		&[
			ExportFormat::Osu,
		]
	}

	fn run(self, file: File, timing_points: &[TimingPoint]) -> Result<()> {
		match self {
			Self::Osu => osu::export(file, timing_points)
		}
	}
}

pub fn export_timing_points(timing_points: Vec<TimingPoint>, fmt: ExportFormat) {
	thread::spawn(move || {
		if let Some(path) = FileDialog::new()
			.apply_format(fmt)
			.save_file()
		{
			let file = File::create(path).unwrap();

			let _ = fmt.run(file, &timing_points);
		}
	});
}