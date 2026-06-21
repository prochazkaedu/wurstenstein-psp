use core::ffi::c_void;

use assets::{Music, Sounds};
use heapless::spsc::{self, Consumer, Producer};
use modplay::{ModAction, ModPlayer, ModRule};

use psp::sys::{self, AUDIO_VOLUME_MAX, AudioFormat, ThreadAttributes};

use crate::audio;
use crate::util::algebra::{Vector2, Vector3};

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::boxed::Box;

const TITLE_RULES: &[ModRule<'static>] = &[
	ModRule {
		order: 7,
		row: 0,
		actions: &[
			ModAction::Jump { order: 6, row: 0 }
		]
	}
];

const INGAME_RULES: &[ModRule<'static>] = &[
	ModRule {
		order: 0,
		row: 0,
		actions: &[
			ModAction::Jump { order: 8, row: 0 }
		]
	},
	ModRule {
		order: 9,
		row: 0,
		actions: &[
			ModAction::Jump { order: 19, row: 0 }
		]
	},
	ModRule {
		order: 28,
		row: 0,
		actions: &[
			ModAction::Jump { order: 19, row: 0 }
		]
	}
];

pub enum MusicRequest {
	Title,
	Stop,
	InGame,
	Death,
	Win,
}

pub enum SoundRequest {
	PlayerJump,
	PlayerExplosion,
	PlayerDeath,
	PlayerShoot,
	EnemyHit,
	EnemyDeath,
	EnemyExplosion,
	EnemyShoot,
	PowerupHpPickup,
	PowerupEnergyPickup,
	PowerupSpeedPickup
}

pub enum AudioRequest {
	PlayMusic { kind: MusicRequest },
	PlaySound { kind: SoundRequest },
}

pub struct Audio {
	tx: Producer<'static, AudioRequest>,
}

struct AudioData {
	audio_channel: i32,
	rx: Consumer<'static, AudioRequest>,
	space_debris: Vec<u8>,
	humntrgt: Vec<u8>,
	brewery: Vec<u8>,

	player_jump: Vec<i16>,
	player_explosion: Vec<i16>,
	player_death: Vec<i16>,
	player_shoot: Vec<i16>,

	enemy_hit: Vec<i16>,
	enemy_explosion: Vec<i16>,
	enemy_death: Vec<i16>,
	enemy_shoot: Vec<i16>,

	powerup_hp_pickup: Vec<i16>,
	powerup_energy_pickup: Vec<i16>,
	powerup_speed_pickup: Vec<i16>,
}

struct PlayingSound<'a> {
	samples: &'a [i16],
	idx: usize,
	atten: i16
}

unsafe extern "C" fn audio_thread_entry(args: usize, data: *mut c_void) -> i32 {
	let data = &mut *(data as *mut AudioData);

	let mut sounds: [Option<PlayingSound>; 16] = [const { None }; 16];

	let mut buffer = [0i16; 2048];

	let mut play_music = false;

	let mut player = ModPlayer::<4>::new(&data.space_debris, 44100).unwrap();

	loop {
		while let Some(request) = data.rx.dequeue() {
			match request {
				AudioRequest::PlayMusic { kind: MusicRequest::Stop } => {
					play_music = false;
				},
				AudioRequest::PlayMusic { kind: MusicRequest::Title } => {
					play_music = true;
					player = ModPlayer::<4>::new(&data.space_debris, 44100).unwrap();
					player.set_rules(Some(TITLE_RULES));
				},
				AudioRequest::PlayMusic { kind: MusicRequest::InGame } => {
					play_music = true;
					player = ModPlayer::<4>::new(&data.space_debris, 44100).unwrap();
					player.set_rules(Some(INGAME_RULES));
				},
				AudioRequest::PlayMusic { kind: MusicRequest::Death } => {
					play_music = true;
					player = ModPlayer::<4>::new(&data.humntrgt, 44100).unwrap();
				},
				AudioRequest::PlayMusic { kind: MusicRequest::Win } => {
					play_music = true;
					player = ModPlayer::<4>::new(&data.brewery, 44100).unwrap();
				},
				AudioRequest::PlaySound { kind } => {
					for sound in &mut sounds {
						if sound.is_some() { continue };

						let samples = match kind {
							SoundRequest::PlayerJump => &data.player_jump,
							SoundRequest::PlayerExplosion => &data.player_explosion,
							SoundRequest::PlayerDeath => &data.player_death,
							SoundRequest::PlayerShoot => &data.player_shoot,
							SoundRequest::EnemyHit => &data.enemy_hit,
							SoundRequest::EnemyDeath => &data.enemy_death,
							SoundRequest::EnemyExplosion => &data.enemy_explosion,
							SoundRequest::EnemyShoot => &data.enemy_shoot,
							SoundRequest::PowerupHpPickup => &data.powerup_hp_pickup,
							SoundRequest::PowerupEnergyPickup => &data.powerup_energy_pickup,
							SoundRequest::PowerupSpeedPickup => &data.powerup_speed_pickup,
						};

						*sound = Some(PlayingSound { samples, idx: 0, atten: i16::MAX });
						break;
					}
				}
			}
		}

		if play_music {
			player.render(&mut buffer);
		} else {
			buffer.fill(0);
		}

		for handle in &mut sounds {
			let Some(sound) = handle else { continue };

			let start = sound.idx;
			let end = (sound.idx + buffer.len() / 2).min(sound.samples.len());

			for i in start..end {
				buffer[(i - start) * 2] += sound.samples[i] / 2;
				buffer[(i - start) * 2 + 1] += sound.samples[i] / 2;
			}

			sound.idx = end;

			if sound.idx == sound.samples.len() {
				*handle = None;
			}
		}

		sys::sceAudioOutputPannedBlocking(data.audio_channel, AUDIO_VOLUME_MAX as i32, AUDIO_VOLUME_MAX as i32, buffer.as_ptr() as _);
	}

	0
}

impl Audio {
	pub fn init(music: Music, sounds: Sounds) -> Self {
		let mut queue = Box::leak(Box::new(spsc::Queue::<AudioRequest, 64>::new()));
		let (tx, rx) = queue.split();

		let audio_channel = unsafe { sys::sceAudioChReserve(-1, 1024, AudioFormat::Stereo) };

		let audio_data = AudioData {
			audio_channel,
			rx,
			space_debris: music.space_debris,
			humntrgt: music.humntrgt,
			brewery: music.brewery,
			player_jump: sounds.player_jump,
			player_explosion: sounds.player_explosion,
			player_death: sounds.player_death,
			player_shoot: sounds.player_shoot,
			enemy_hit: sounds.enemy_hit,
			enemy_explosion: sounds.enemy_explosion,
			enemy_death: sounds.enemy_death,
			enemy_shoot: sounds.enemy_shoot,
			powerup_hp_pickup: sounds.powerup_hp_pickup,
			powerup_energy_pickup: sounds.powerup_energy_pickup,
			powerup_speed_pickup: sounds.powerup_speed_pickup,
		};

		let audio_data = Box::into_raw(Box::new(audio_data));

		unsafe {
			let handle = sys::sceKernelCreateThread(c"audio_thread".as_ptr() as _, audio_thread_entry, 0x12, 0x10000, ThreadAttributes::empty(), core::ptr::null_mut() as _);
			sys::sceKernelStartThread(handle, core::mem::size_of::<AudioData>(), audio_data as _);
		}

		Self {
			tx
		}
	}

	pub fn play_music(&mut self, kind: MusicRequest) {
		let _ = self.tx.enqueue(AudioRequest::PlayMusic { kind });
	}

	pub fn play_sound(&mut self, kind: SoundRequest, position: Option<Vector3>, radius: f32) {
		// let _ = self.tx.enqueue(AudioRequest::PlaySound { kind, position, radius });
		let _ = self.tx.enqueue(AudioRequest::PlaySound { kind });
	}

	pub fn update_position(&mut self, position: Vector3, rotation: f32) {
		// let _ = self.tx.enqueue(AudioRequest::SetPosition { position, rotation });
	}
}

