use crate::util::font::Font;
use crate::util::model::Model;
use crate::util::algebra::Vector3;

use assets::{Models, Textures};
use parry2d::{math::{Pose, Vec2}, shape::Cuboid};

fn get_bounding_box_from_model(model: &assets::Model, scale: f32) -> BoundingBox {
	// TODO - unhardcode 8
	let min_x = model.vertices.chunks(8).map(|pos| pos[5]).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() * scale;
	let min_y = model.vertices.chunks(8).map(|pos| pos[6]).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() * scale;
	let min_z = model.vertices.chunks(8).map(|pos| pos[7]).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() * scale;

	let max_x = model.vertices.chunks(8).map(|pos| pos[5]).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() * scale;
	let max_y = model.vertices.chunks(8).map(|pos| pos[6]).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() * scale;
	let max_z = model.vertices.chunks(8).map(|pos| pos[7]).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() * scale;

	BoundingBox {
		min: (min_x, min_y, min_z),
		max: (max_x, max_y, max_z)
	}
}

pub struct Assets {
	pub font: Font<'static>,
	pub terrain: Model,
	pub player: Model,
	pub sausage_bullet: Model,
	pub sausage_tip: Model,
	pub apple: Model,
	pub pear: Model,
	pub powerup_hp: Model,
	pub powerup_energy: Model,
	pub powerup_speed: Model,
	pub player_bounding_box: BoundingBox,
	pub bullet_bounding_box: BoundingBox,
}

#[derive(Debug, Clone)]
pub struct BoundingBox {
	pub min: (f32, f32, f32),
	pub max: (f32, f32, f32)
}

impl BoundingBox {
	// pub fn generate_mesh(&self) -> Model {
	// 	let positions = [
	// 		[self.min.0, self.min.1, self.min.2],
	// 		[self.max.0, self.min.1, self.min.2],
	// 		[self.min.0, self.max.1, self.min.2],
	// 		[self.max.0, self.max.1, self.min.2],
	// 		[self.min.0, self.min.1, self.max.2],
	// 		[self.max.0, self.min.1, self.max.2],
	// 		[self.min.0, self.max.1, self.max.2],
	// 		[self.max.0, self.max.1, self.max.2],
	// 	];
	//
	// 	let normals = std::array::from_fn::<_, 8, _>(|idx| {
	// 		let x = if idx & 1 != 0 { 1.0 } else { -1.0 };
	// 		let y = if idx & 2 != 0 { 1.0 } else { -1.0 };
	// 		let z = if idx & 4 != 0 { 1.0 } else { -1.0 };
	//
	// 		[
	// 			x, 0.0, 0.0,
	// 			0.0, y, 0.0,
	// 			0.0, 0.0, z
	// 		]
	// 	}).concat();
	//
	// 	let texcoords = std::array::from_fn::<_, 24, _>(|_| [0.0, 0.0]).concat();
	//
	// 	// Repeat each vertex 3 times for it to have its own normal
	// 	// (first one points in X direction, second in Y and third in Z)
	// 	let positions = positions.map(|pos| pos.repeat(3)).concat();
	//
	// 	let indices = vec![
	// 		// Left face
	// 		0, 12, 6,
	// 		6, 12, 18,
	//
	// 		// Right face
	// 		3, 9, 15,
	// 		9, 21, 15,
	//
	// 		// Bottom face
	// 		1, 4, 13,
	// 		4, 16, 13,
	//
	// 		// Top face
	// 		7, 19, 10,
	// 		10, 19, 22,
	//
	// 		// Back face
	// 		2, 8, 5,
	// 		5, 8, 11,
	//
	// 		// Front face
	// 		14, 17, 20,
	// 		17, 23, 20
	// 	];
	//
	// 	Mesh {
	// 		positions,
	// 		normals,
	// 		texcoords,
	// 		indices,
	// 		vertex_color: vec![],
	// 		face_arities: vec![],
	// 		normal_indices: vec![],
	// 		texcoord_indices: vec![],
	// 		material_id: None,
	// 	}
	// }

	pub fn get_collision_shape(&self) -> (Cuboid, Pose) {
		let w = (self.max.0 - self.min.0) / 2.0;
		let d = (self.max.2 - self.min.2) / 2.0;

		let dx = (self.max.0 + self.min.0) / 2.0;
		let dz = (self.max.2 + self.min.2) / 2.0;

		(
			Cuboid::new(Vec2::new(w, d)),
			Pose::translation(dx, dz)
		)
	}
}

impl Assets {
	pub fn init(models: Models, textures: Textures) -> Self {
		let font = Font::new(include_bytes!("../notosansmono-ascii.ttf"));

		let player_scale = 20.0;

		let player_bounding_box = get_bounding_box_from_model(&models.pastry, player_scale);
		let bullet_bounding_box = get_bounding_box_from_model(&models.sausage_bullet, player_scale);

		let terrain = Model::new()
			.with_mesh(crate::playfield::EXAMPLE_MAZE.generate_mesh())
			.with_texture(textures.terrain, false);

		let player = Model::new()
			.with_mesh(models.pastry)
			.with_texture(textures.player, true)
			.with_scale(Vector3::UNIT * player_scale);

		let sausage_bullet = Model::new()
			.with_mesh(models.sausage_bullet)
			.with_texture(textures.sausage_bullet, true)
			.with_scale(Vector3::UNIT * player_scale);

		let sausage_tip = Model::new()
			.with_mesh(models.sausage_tip)
			.with_texture(textures.sausage_tip, true)
			.with_scale(Vector3::UNIT * player_scale);

		let apple = Model::new()
			.with_mesh(models.apple)
			.with_texture(textures.apple, true)
			.with_scale(Vector3::UNIT * 30.0);

		let pear = Model::new()
			.with_mesh(models.pear)
			.with_texture(textures.pear, true)
			.with_scale(Vector3::UNIT * 20.0);

		let powerup_hp = Model::new()
			.with_mesh(models.powerup_hp)
			.with_scale(Vector3::UNIT * 2.0);

		let powerup_energy = Model::new()
			.with_mesh(models.powerup_energy)
			.with_scale(Vector3::UNIT * 2.0);

		let powerup_speed = Model::new()
			.with_mesh(models.powerup_speed)
			.with_scale(Vector3::UNIT * 2.0);

		Self {
			font,
			terrain,
			player,
			sausage_tip,
			sausage_bullet,
			powerup_hp,
			powerup_energy,
			powerup_speed,
			apple,
			pear,
			player_bounding_box,
			bullet_bounding_box,
		}
	}
}

