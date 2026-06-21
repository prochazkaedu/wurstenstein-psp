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
	PlaySound { kind: SoundRequest, position: Option<Vector3>, radius: f32 },
	SetPosition { position: Vector3, rotation: f32 },
}

pub struct Audio {
	tx: Producer<'static, AudioRequest>,
	music_tx: Producer<'static, MusicRequest>
}

struct AudioData {
	scene: oddio::SpatialScene,
	audio_channel: i32,
	music_rx: Consumer<'static, MusicRequest>,
	space_debris: Vec<u8>,
	humntrgt: Vec<u8>,
	brewery: Vec<u8>
}

struct SoundData {
	rx: Consumer<'static, AudioRequest>,
	scene_handle: oddio::SpatialSceneControl,

	player_jump: Arc<oddio::Frames<f32>>,
	player_explosion: Arc<oddio::Frames<f32>>,
	player_death: Arc<oddio::Frames<f32>>,
	player_shoot: Arc<oddio::Frames<f32>>,

	enemy_hit: Arc<oddio::Frames<f32>>,
	enemy_explosion: Arc<oddio::Frames<f32>>,
	enemy_death: Arc<oddio::Frames<f32>>,
	enemy_shoot: Arc<oddio::Frames<f32>>,

	powerup_hp_pickup: Arc<oddio::Frames<f32>>,
	powerup_energy_pickup: Arc<oddio::Frames<f32>>,
	powerup_speed_pickup: Arc<oddio::Frames<f32>>,
}

unsafe extern "C" fn audio_thread_entry(args: usize, data: *mut c_void) -> i32 {
	let data = &mut *(data as *mut AudioData);

	let mut buffer = [0i16; 2048];
	let mut oddio_buffer = [0f32; 2048];

	let mut play_music = false;

	let mut player = ModPlayer::<4>::new(&data.space_debris, 44100).unwrap();

	loop {
		while let Some(request) = data.music_rx.dequeue() {
			match request {
				MusicRequest::Stop => {
					play_music = false;
				},
				MusicRequest::Title => {
					play_music = true;
					player = ModPlayer::<4>::new(&data.space_debris, 44100).unwrap();
					player.set_rules(Some(TITLE_RULES));
				},
				MusicRequest::InGame => {
					play_music = true;
					player = ModPlayer::<4>::new(&data.space_debris, 44100).unwrap();
					player.set_rules(Some(INGAME_RULES));
				},
				MusicRequest::Death => {
					play_music = true;
					player = ModPlayer::<4>::new(&data.humntrgt, 44100).unwrap();
				},
				MusicRequest::Win => {
					play_music = true;
					player = ModPlayer::<4>::new(&data.brewery, 44100).unwrap();
				},
			}
		}

		if play_music {
			player.render(&mut buffer);
		} else {
			buffer.fill(0);
		}

		let out_frames = oddio::frame_stereo(&mut oddio_buffer);
		oddio::run(&mut data.scene, 44100, out_frames);

		for (sample, out) in oddio_buffer.iter().zip(buffer.iter_mut()) {
			*out = (*out / 2 + (*sample * 32768.0 / 2.0) as i16);
		}

		sys::sceAudioOutputPannedBlocking(data.audio_channel, AUDIO_VOLUME_MAX as i32, AUDIO_VOLUME_MAX as i32, buffer.as_ptr() as _);
	}

	0
}

unsafe extern "C" fn sound_thread_entry(args: usize, data: *mut c_void) -> i32 {
	let data = &mut *(data as *mut SoundData);

	let mut handles: Vec<(Option<Vector3>, oddio::Spatial)> = vec![];
	let mut player_position = Vector3::ZERO;
	let mut player_rotation = 0.0;

	'requestloop: loop {
		let Some(request) = data.rx.dequeue() else {
			sys::sceKernelDelayThread(5000);
			continue
		};

		match request {
			AudioRequest::PlaySound { kind, position, radius } => {
				// Try to find an existing slot

				let mut generate_handle = || {
					let frames = match kind {
						SoundRequest::PlayerJump => data.player_jump.clone(),
						SoundRequest::PlayerExplosion => data.player_explosion.clone(),
						SoundRequest::PlayerDeath => data.player_death.clone(),
						SoundRequest::PlayerShoot => data.player_shoot.clone(),
						SoundRequest::EnemyHit => data.enemy_hit.clone(),
						SoundRequest::EnemyDeath => data.enemy_death.clone(),
						SoundRequest::EnemyExplosion => data.enemy_explosion.clone(),
						SoundRequest::EnemyShoot => data.enemy_shoot.clone(),
						SoundRequest::PowerupHpPickup => data.powerup_hp_pickup.clone(),
						SoundRequest::PowerupEnergyPickup => data.powerup_energy_pickup.clone(),
						SoundRequest::PowerupSpeedPickup => data.powerup_speed_pickup.clone(),
					};

					let frames = oddio::FramesSignal::from(frames);

					let translated_position = if let Some(position) = position {
						let pos = Vector2::new(position.x, position.z) - Vector2::new(player_position.x, player_position.z);
						let pos = pos.rotate(player_rotation);

						[pos.x, position.y - player_position.y, pos.y]
					} else {
						[0.0, 0.0, 0.0]
					};

					(
						position,
						data.scene_handle.play(frames, oddio::SpatialOptions {
							position: translated_position.into(),
							velocity: [0.0, 0.0, 0.0].into(),
							radius
						})
					)
				};

				for handle in &mut handles {
					if !handle.1.is_finished() { continue }

					*handle = generate_handle();
					continue 'requestloop;
				}

				handles.push(generate_handle());
			},
			AudioRequest::SetPosition { position, rotation } => {
				player_position = position;
				player_rotation = rotation;

				for handle in &mut handles {
					let Some(position) = handle.0 else { continue };

					let pos = Vector2::new(position.x, position.z) - Vector2::new(player_position.x, player_position.z);
					let pos = pos.rotate(player_rotation);

					let translated_position = [pos.x, position.y - player_position.y, pos.y];

					handle.1.set_motion(
						translated_position.into(),
						[0.0, 0.0, 0.0].into(),
						true
					);
				}
			}
		}
	}

	0
}

impl Audio {
	pub fn init(music: Music, sounds: Sounds) -> Self {
		let mut queue = Box::leak(Box::new(spsc::Queue::<AudioRequest, 16>::new()));
		let (tx, rx) = queue.split();
		let mut queue = Box::leak(Box::new(spsc::Queue::<MusicRequest, 16>::new()));
		let (music_tx, music_rx) = queue.split();

		let (mut scene_handle, mut scene) = oddio::SpatialScene::new();

		let audio_channel = unsafe { sys::sceAudioChReserve(-1, 1024, AudioFormat::Stereo) };

		let audio_data = AudioData {
			scene,
			audio_channel,
			music_rx,
			space_debris: music.space_debris,
			humntrgt: music.humntrgt,
			brewery: music.brewery
		};

		let audio_data = Box::into_raw(Box::new(audio_data));

		unsafe {
			let handle = sys::sceKernelCreateThread(c"audio_thread".as_ptr() as _, audio_thread_entry, 0x12, 0x10000, ThreadAttributes::empty(), core::ptr::null_mut() as _);
			sys::sceKernelStartThread(handle, core::mem::size_of::<AudioData>(), audio_data as _);
		}

		let player_jump = oddio::Frames::from_slice(44100, &sounds.player_jump);
		let player_explosion = oddio::Frames::from_slice(44100, &sounds.player_explosion);
		let player_death = oddio::Frames::from_slice(44100, &sounds.player_death);
		let player_shoot = oddio::Frames::from_slice(44100, &sounds.player_shoot);

		let enemy_hit = oddio::Frames::from_slice(44100, &sounds.enemy_hit);
		let enemy_explosion = oddio::Frames::from_slice(44100, &sounds.enemy_explosion);
		let enemy_death = oddio::Frames::from_slice(44100, &sounds.enemy_death);
		let enemy_shoot = oddio::Frames::from_slice(44100, &sounds.enemy_shoot);

		let powerup_hp_pickup = oddio::Frames::from_slice(44100, &sounds.powerup_hp_pickup);
		let powerup_energy_pickup = oddio::Frames::from_slice(44100, &sounds.powerup_energy_pickup);
		let powerup_speed_pickup = oddio::Frames::from_slice(44100, &sounds.powerup_speed_pickup);

		let sound_data = SoundData {
			rx,
			scene_handle,
			player_jump,
			player_explosion,
			player_death,
			player_shoot,
			enemy_hit,
			enemy_explosion,
			enemy_death,
			enemy_shoot,
			powerup_hp_pickup,
			powerup_energy_pickup,
			powerup_speed_pickup
		};

		let sound_data = Box::into_raw(Box::new(sound_data));

		unsafe {
			let handle = sys::sceKernelCreateThread(c"sound_thread".as_ptr() as _, sound_thread_entry, 0x12, 0x10000, ThreadAttributes::empty(), core::ptr::null_mut() as _);
			sys::sceKernelStartThread(handle, core::mem::size_of::<SoundData>(), sound_data as _);
		}

		Self {
			music_tx,
			tx
		}
	}

	pub fn play_music(&mut self, kind: MusicRequest) {
		let _ = self.music_tx.enqueue(kind);
	}

	pub fn play_sound(&mut self, kind: SoundRequest, position: Option<Vector3>, radius: f32) {
		let _ = self.tx.enqueue(AudioRequest::PlaySound { kind, position, radius });
	}

	pub fn update_position(&mut self, position: Vector3, rotation: f32) {
		let _ = self.tx.enqueue(AudioRequest::SetPosition { position, rotation });
	}
}

