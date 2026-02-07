use std::fmt::Display;
use std::fs::File;
use std::io::Write;

use eyre::Result;
use rfd::FileDialog;

use crate::timing::TimingPoint;

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

	fn run(self, mut file: File, timing_points: &[TimingPoint]) -> Result<()> {
		writeln!(file, "osu file format v14")?;
		writeln!(file)?;
		writeln!(file, "[TimingPoints]")?;

		for tp in timing_points {
			writeln!(
				file,
				"{:.0},{:.8},{},2,1,100,1,0",
				tp.offset,
				tp.ms_per_beat(),
				tp.signature.0
			)?;
		}

		Ok(())
	}
}

pub fn export_timing_points(timing_points: &[TimingPoint], fmt: ExportFormat) -> Result<()> {
	if let Some(path) = FileDialog::new()
		.apply_format(fmt)
		.save_file()
	{
		let file = File::create(path)?;

		fmt.run(file, timing_points)?;
	}

	Ok(())
}