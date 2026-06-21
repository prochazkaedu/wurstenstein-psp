use std::io::Cursor;

use image::ImageReader;

include!("./assets_struct.rs");

fn parse_model(mut bytes: &[u8]) -> Model {
	let (model, _material) =
		tobj::load_obj_buf(&mut bytes, &tobj::GPU_LOAD_OPTIONS, |_| {
			Err(tobj::LoadError::ReadError)
		})
		.unwrap();
	let mut mesh = model.into_iter().next().unwrap().mesh;

	if mesh.texcoords.is_empty() {
		mesh.texcoords.resize(mesh.positions.len() / 3 * 2, 0.0);
	}
	assert_eq!(mesh.positions.len() / 3, mesh.normals.len() / 3);
	assert_eq!(mesh.positions.len() / 3, mesh.texcoords.len() / 2);

	let vertices = mesh
		.positions
		.chunks(3)
		.zip(mesh.normals.chunks(3))
		.zip(mesh.texcoords.chunks(2))
		// .flat_map(|((p, n), t)| [p[0], p[1], p[2], n[0], n[1], n[2], t[0], t[1]])
		.flat_map(|((p, n), t)| [t[0], t[1], n[0], n[1], n[2], p[0], p[1], p[2]])
		.collect::<Vec<_>>();

	Model {
		vertices,
		indices: mesh.indices.into_iter().map(|x| x.try_into().unwrap()).collect()
	}
}

fn parse_texture(bytes: &[u8]) -> Texture {
	let image = ImageReader::new(Cursor::new(bytes))
		.with_guessed_format()
		.unwrap()
		.decode()
		.unwrap();

	let width = image.width() as usize;
	let height = image.height() as usize;
	let mut bytes = image.flipv().into_rgba8().into_raw();

	for i in (0..bytes.len()).step_by(4) {
		let a = bytes[0 + i];
		let r = bytes[1 + i];
		let g = bytes[2 + i];
		let b = bytes[3 + i];

		bytes[0 + i] = r;
		bytes[1 + i] = g;
		bytes[2 + i] = b;
		bytes[3 + i] = a;
	}

	assert_eq!(width * height * 4, bytes.len());

	Texture {
		width,
		height,
		bytes
	}
}

fn main() {
	println!("cargo::rerun-if-changed=files");

	let assets = Assets {
		models: Models {
			pastry: parse_model(include_bytes!("./files/objects/pastry/pastry.obj")),
			sausage_bullet: parse_model(include_bytes!("./files/objects/pastry/sausage_body.obj")),
			sausage_tip: parse_model(include_bytes!("./files/objects/pastry/sausage_tip.obj")),
			apple: parse_model(include_bytes!("./files/objects/apple/apple.obj")),
			pear: parse_model(include_bytes!("./files/objects/pear/pear.obj")),
			powerup_hp: parse_model(include_bytes!("./files/objects/powerups/powerup-hp.obj")),
			powerup_energy: parse_model(include_bytes!("./files/objects/powerups/powerup-energy.obj")),
			powerup_speed: parse_model(include_bytes!("./files/objects/powerups/powerup-speed.obj")),
		},
		textures: Textures {
			player: parse_texture(include_bytes!("./files/objects/pastry/pastry.png")),
			sausage_bullet: parse_texture(include_bytes!("./files/objects/pastry/sausage_body.png")),
			sausage_tip: parse_texture(include_bytes!("./files/objects/pastry/sausage_tip.png")),
			apple: parse_texture(include_bytes!("./files/objects/apple/apple_tex.png")),
			pear: parse_texture(include_bytes!("./files/objects/pear/pear_tex.png")),
			terrain: parse_texture(include_bytes!("./files/textures/terrain.png")),
		},
		music: Music {
			space_debris: include_bytes!("./files/music/space_debris.mod").to_vec(),
			humntrgt: include_bytes!("./files/music/humntrgt.mod").to_vec(),
			brewery: include_bytes!("./files/music/brewery.mod").to_vec(),
		},
		sounds: Sounds {
			player_jump: include_bytes!("./files/sounds/sfx_movement_jump14.wav").to_vec(),
			player_explosion: include_bytes!("./files/sounds/sfx_exp_medium1.wav").to_vec(),
			player_death: include_bytes!("./files/sounds/sfx_deathscream_human11.wav").to_vec(),
			player_shoot: include_bytes!("./files/sounds/sfx_weapon_shotgun3.wav").to_vec(),
			enemy_hit: include_bytes!("./files/sounds/sfx_weapon_shotgun2.wav").to_vec(),
			enemy_explosion: include_bytes!("./files/sounds/sfx_exp_medium2.wav").to_vec(),
			enemy_death: include_bytes!("./files/sounds/sfx_deathscream_alien4.wav").to_vec(),
			enemy_shoot: include_bytes!("./files/sounds/sfx_weapon_shotgun2.wav").to_vec(),
			powerup_hp_pickup: include_bytes!("./files/sounds/sfx_sounds_powerup6.wav").to_vec(),
			powerup_energy_pickup: include_bytes!("./files/sounds/sfx_sounds_powerup9.wav").to_vec(),
			powerup_speed_pickup: include_bytes!("./files/sounds/sfx_sounds_powerup16.wav").to_vec(),
		}
	};

	let data = wincode::serialize(&assets).unwrap();

	std::fs::write("../../assets.bin", data).unwrap();
}
