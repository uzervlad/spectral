use std::fs::File;
use std::io::Write as _;

use eyre::Result;

use crate::timing::TimingPoint;

pub fn create(mut file: File, timing_points: &[TimingPoint]) -> Result<()> {
	writeln!(file, "offset,bpm,signature_numerator,signature_denominator")?;

	for tp in timing_points {
		writeln!(
			file,
			"{},{},{},{}",
			tp.offset, tp.bpm, tp.signature.0, tp.signature.1,
		)?;
	}

	Ok(())
}

pub fn patch(file: File, timing_points: &[TimingPoint]) -> Result<()> {
	create(file, timing_points)
}
