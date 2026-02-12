use std::fs::File;
use std::io::Write;

use eyre::Result;

use crate::timing::TimingPoint;

pub fn export(mut file: File, timing_points: &[TimingPoint]) -> Result<()> {
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
