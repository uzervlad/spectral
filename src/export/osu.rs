use std::fs::File;
use std::io::Write as _;

use eyre::Result;

use crate::timing::TimingPoint;

pub fn create(mut file: File, timing_points: &[TimingPoint]) -> Result<()> {
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

pub fn patch(mut file: File, contents: String, timing_points: &[TimingPoint]) -> Result<()> {
	let mut in_timing = false;

	for line in contents.lines() {
		if line.trim() == "[TimingPoints]" {
			writeln!(file, "{}", line)?;

			for tp in timing_points {
				writeln!(
					file,
					"{:.0},{:.8},{},2,1,100,1,0",
					tp.offset,
					tp.ms_per_beat(),
					tp.signature.0
				)?;
			}

			writeln!(file)?;

			in_timing = true;
			continue;
		}

		if in_timing && line.starts_with('[') {
			in_timing = false;
		}

		if !in_timing {
			writeln!(file, "{}", line)?;
		}
	}

	Ok(())
}
