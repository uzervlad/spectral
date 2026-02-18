use std::env::current_dir;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings {
	#[serde(skip, default)]
	_save_path: PathBuf,

	pub audio_volume: f32,
	pub metronome_volume: f32,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			_save_path: Default::default(),

			audio_volume: 0.4,
			metronome_volume: 0.2,
		}
	}
}

impl Settings {
	fn load() -> Self {
		let _save_path = dirs::config_local_dir()
			.unwrap_or_else(|| current_dir().unwrap())
			.join("spectral");

		if !_save_path.exists() {
			fs::create_dir_all(&_save_path).expect("unable to create config directory");
		}

		let _save_path = _save_path.join("settings.json");

		let mut settings: Self = fs::read(&_save_path)
			.ok()
			.and_then(|content| serde_json::from_slice(&content).ok())
			.unwrap_or_default();

		settings._save_path = _save_path;
		settings
	}

	fn save(&self) {
		let _ = fs::write(&self._save_path, serde_json::to_string(&self).unwrap());
	}
}

type SettingsWriteCallback = dyn FnOnce(&mut Settings) + Send + 'static;

fn settings_store_listener(
	settings: Arc<RwLock<Settings>>,
	rx: Receiver<Box<SettingsWriteCallback>>,
) {
	let save_delay = Duration::from_secs(2);
	let mut last_change = Instant::now();
	let mut pending_save = false;

	loop {
		let timeout = save_delay.checked_sub(last_change.elapsed());

		println!("{:?}", timeout);

		match timeout {
			Some(timeout) => match rx.recv_timeout(timeout) {
				Ok(cb) => {
					let mut settings = settings.write().unwrap();
					cb(&mut settings);

					last_change = Instant::now();
					pending_save = true;
				},
				Err(RecvTimeoutError::Timeout) => {
					if pending_save {
						println!("saving");
						let settings = settings.read().unwrap();
						settings.save();
						pending_save = false;
					}
				},
				Err(RecvTimeoutError::Disconnected) => break,
			},
			None => match rx.recv() {
				Ok(cb) => {
					let mut settings = settings.write().unwrap();
					cb(&mut settings);

					last_change = Instant::now();
					pending_save = true;
				},
				_ => break,
			},
		}
	}
}

pub struct SettingsManager {
	settings: Arc<RwLock<Settings>>,
	tx: Sender<Box<SettingsWriteCallback>>,
}

impl SettingsManager {
	pub fn new() -> Self {
		let settings = Arc::new(RwLock::new(Settings::load()));

		let (tx, rx) = mpsc::channel();

		let _settings = settings.clone();
		thread::spawn(move || {
			settings_store_listener(_settings, rx);
		});

		Self { settings, tx }
	}

	pub fn read<T>(&self, cb: impl Fn(&Settings) -> T) -> T {
		let settings = self.settings.read().unwrap();

		cb(&settings)
	}

	pub fn write(&self, cb: impl FnOnce(&mut Settings) + Send + 'static) {
		let cb = Box::new(cb);

		let _ = self.tx.send(cb);
	}
}
