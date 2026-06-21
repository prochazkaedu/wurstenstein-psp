#[derive(wincode::SchemaWrite, wincode::SchemaRead)]
pub struct Model {
	pub vertices: Vec<f32>,
	pub indices: Vec<u16>
}

#[derive(wincode::SchemaWrite, wincode::SchemaRead)]
pub struct Models {
	pub pastry: Model,
	pub sausage_bullet: Model,
	pub sausage_tip: Model,
	pub apple: Model,
	pub pear: Model,
	pub powerup_hp: Model,
	pub powerup_energy: Model,
	pub powerup_speed: Model,
}

#[derive(wincode::SchemaWrite, wincode::SchemaRead)]
pub struct Texture {
	pub width: usize,
	pub height: usize,
	pub bytes: Vec<u8>
}

#[derive(wincode::SchemaWrite, wincode::SchemaRead)]
pub struct Textures {
	pub player: Texture,
	pub sausage_bullet: Texture,
	pub sausage_tip: Texture,
	pub apple: Texture,
	pub pear: Texture,
	pub terrain: Texture,
}

#[derive(wincode::SchemaWrite, wincode::SchemaRead)]
pub struct Music {
	pub space_debris: Vec<u8>,
	pub humntrgt: Vec<u8>,
	pub brewery: Vec<u8>,
}

#[derive(wincode::SchemaWrite, wincode::SchemaRead)]
pub struct Sounds {
	pub player_jump: Vec<f32>,
	pub player_explosion: Vec<f32>,
	pub player_death: Vec<f32>,
	pub player_shoot: Vec<f32>,
	pub enemy_hit: Vec<f32>,
	pub enemy_death: Vec<f32>,
	pub enemy_explosion: Vec<f32>,
	pub enemy_shoot: Vec<f32>,
	pub powerup_hp_pickup: Vec<f32>,
	pub powerup_energy_pickup: Vec<f32>,
	pub powerup_speed_pickup: Vec<f32>,
}

#[derive(wincode::SchemaWrite, wincode::SchemaRead)]
pub struct Assets {
	pub models: Models,
	pub textures: Textures,
	pub music: Music,
	pub sounds: Sounds,
}

