use crate::app::SpectralApp;
use crate::timing::TimingPoint;

const MAX_HISTORY_CAPACITY: usize = 200;

#[derive(Clone, Copy)]
pub enum EditHistoryEntry {
	CreateTimingPoint(TimingPoint),
	DeleteTimingPoint(TimingPoint),
	ModifyTimingPoint {
		before: TimingPoint,
		after: TimingPoint,
	},
}

impl ToString for EditHistoryEntry {
	fn to_string(&self) -> String {
		match self {
			Self::CreateTimingPoint(_) => "Create timing point".into(),
			Self::DeleteTimingPoint(_) => "Delete timing point".into(),
			Self::ModifyTimingPoint { before, after } => {
				if before.bpm != after.bpm {
					format!("Change BPM {:.02} -> {:.02}", before.bpm, after.bpm)
				} else if before.offset != after.offset {
					format!("Change offset {} -> {}", before.offset, after.offset)
				} else {
					format!(
						"Change signature {}/{} -> {}/{}",
						before.signature.0,
						before.signature.1,
						after.signature.0,
						after.signature.1
					)
				}
			},
		}
	}
}

#[derive(Default)]
pub struct EditHistory {
	changes: Vec<EditHistoryEntry>,
	cursor: usize,
}

impl EditHistory {
	pub fn push(&mut self, entry: EditHistoryEntry) {
		self.changes.truncate(self.cursor);

		self.changes.push(entry);

		if self.changes.len() > MAX_HISTORY_CAPACITY {
			self.changes
				.drain(0..self.changes.len() - MAX_HISTORY_CAPACITY);
		}

		self.cursor = self.changes.len();
	}

	pub fn can_undo(&self) -> bool {
		self.cursor != 0
	}

	pub fn undo(&mut self) -> Option<EditHistoryEntry> {
		if self.cursor == 0 {
			None
		} else {
			self.cursor -= 1;
			self.changes.get(self.cursor).copied()
		}
	}

	pub fn can_redo(&mut self) -> bool {
		self.cursor < self.changes.len()
	}

	pub fn redo(&mut self) -> Option<EditHistoryEntry> {
		if self.cursor == self.changes.len() {
			None
		} else {
			self.cursor += 1;
			self.changes.get(self.cursor - 1).copied()
		}
	}
}

impl SpectralApp {
	pub fn undo(&mut self, entry: EditHistoryEntry) {
		match entry {
			EditHistoryEntry::CreateTimingPoint(created_tp) => {
				self.timing_points
					.write()
					.unwrap()
					.retain(|tp| created_tp.id() != tp.id());
			},
			EditHistoryEntry::DeleteTimingPoint(deleted_tp) => {
				self.timing_points.write().unwrap().push(deleted_tp.clone());
				self.sort_timing_points();
			},
			EditHistoryEntry::ModifyTimingPoint { before, after } => {
				if let Some(tp) = self
					.timing_points
					.write()
					.unwrap()
					.iter_mut()
					.find(|tp| tp.id() == after.id())
				{
					*tp = before.clone();
				}
			},
		}
	}

	pub fn redo(&mut self, entry: EditHistoryEntry) {
		match entry {
			EditHistoryEntry::CreateTimingPoint(created_tp) => {
				self.timing_points.write().unwrap().push(created_tp.clone());
				self.sort_timing_points();
			},
			EditHistoryEntry::DeleteTimingPoint(deleted_tp) => {
				self.timing_points
					.write()
					.unwrap()
					.retain(|tp| deleted_tp.id() != tp.id());
			},
			EditHistoryEntry::ModifyTimingPoint { before, after } => {
				if let Some(tp) = self
					.timing_points
					.write()
					.unwrap()
					.iter_mut()
					.find(|tp| tp.id() == before.id())
				{
					*tp = after.clone();
				}
			},
		}
	}
}
