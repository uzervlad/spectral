use std::fmt::Display;
use std::fs::{self, File};
use std::sync::mpsc::Sender;
use std::thread;

use eyre::Result;
use rfd::FileDialog;

use crate::events::SpectralEvent;
use crate::timing::TimingPoint;

mod csv;
mod osu;

trait ApplyExportFormat {
	fn apply_format(self, fmt: ExportFormat) -> Self;
}

#[derive(Clone, Copy)]
pub enum ExportFormat {
	Csv,
	Osu,
}

impl ApplyExportFormat for FileDialog {
	fn apply_format(self, fmt: ExportFormat) -> Self {
		match fmt {
			ExportFormat::Csv => self.add_filter("CSV", &["csv"]),
			ExportFormat::Osu => self.add_filter("osu! beatmap", &["osu"]),
		}
	}
}

impl Display for ExportFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Csv => "CSV (.csv)",
				Self::Osu => "osu! (.osu)",
			}
		)
	}
}

impl ExportFormat {
	pub fn game_formats() -> &'static [Self] {
		&[ExportFormat::Osu]
	}

	fn create(self, file: File, timing_points: &[TimingPoint]) -> Result<()> {
		match self {
			Self::Csv => csv::create(file, timing_points),
			Self::Osu => osu::create(file, timing_points),
		}
	}

	fn patch(self, file: File, contents: String, timing_points: &[TimingPoint]) -> Result<()> {
		match self {
			Self::Csv => csv::patch(file, timing_points),
			Self::Osu => osu::patch(file, contents, timing_points),
		}
	}
}

pub fn export_timing_points(
	timing_points: Vec<TimingPoint>,
	fmt: ExportFormat,
	tx: Sender<SpectralEvent>,
) {
	thread::spawn(move || {
		if let Some(path) = FileDialog::new().apply_format(fmt).save_file() {
			let result = if path.exists() {
				let contents = fs::read_to_string(&path).unwrap();
				let file = File::create(path).unwrap();
				fmt.patch(file, contents, &timing_points)
			} else {
				let file = File::create_new(path).unwrap();
				fmt.create(file, &timing_points)
			};

			let _ = tx.send(SpectralEvent::Export {
				error: result.err().map(|e| e.to_string()),
			});
		}
	});
}
