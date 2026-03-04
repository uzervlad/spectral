use std::fmt::Write as _;
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
	let mut done = false;

	let mut section = "[TimingPoints]\n".to_owned();

	for tp in timing_points {
		writeln!(
			section,
			"{:.0},{:.8},{},2,1,100,1,0",
			tp.offset,
			tp.ms_per_beat(),
			tp.signature.0
		)?;
	}

	for line in contents.lines() {
		if line.trim() == "[TimingPoints]" {
			writeln!(file, "{}", section)?;
			writeln!(file)?;

			in_timing = true;
			done = true;
			continue;
		} else if line.trim() == "[HitObjects]" && !done {
			// New .osu files without timing points don't have
			// a [TimingPoints] section, so we insert one
			// before [HitObjects]
			writeln!(file, "{}", section)?;
			writeln!(file)?;
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
