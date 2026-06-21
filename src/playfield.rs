extern crate alloc;

pub trait PlayfieldPiece {
	// Return whether this piece is a hole in the floor or not.
	fn is_empty(&self) -> bool;

	// Returns coordinates for the border wall texture (undefined if not at the border).
	// (X1, Y1, X2, Y2) - all in range 0.0..=1.0.
	fn vert_texture(&self) -> (f32, f32, f32, f32);

	// Returns coordinates for the floor texture.
	// (X1, Y1, X2, Y2) - all in range 0.0..=1.0.
	fn horiz_top_texture(&self) -> (f32, f32, f32, f32);

	// Returns coordinates for the "ceiling" texture.
	// (X1, Y1, X2, Y2) - all in range 0.0..=1.0.
	fn horiz_bottom_texture(&self) -> (f32, f32, f32, f32);
}

pub struct Playfield<'a, T: PlayfieldPiece> {
	// X/Z size of each wall piece.
	pub scale: f32,
	// Y size of each wall piece.
	pub height: f32,
	// Y coordinate of the death barrier when the player falls off.
	pub death_barrier: f32,
	// Vector of rows, containing a vector of cells.
	pub field: &'a [&'a [T]],
	// Spawn point for the player.
	pub player_spawn_point: [usize; 2],
	// Possible spawn points for powerups.
	pub powerup_spawn_points: &'a [[usize; 2]],
	// Possible spawn points for enemies.
	pub enemy_spawn_points: &'a [[usize; 2]],
}

impl<'a, T: PlayfieldPiece> Playfield<'a, T> {
	pub fn dimensions(&self) -> (usize, usize) {
		let width = self.field.iter()
			.map(|x| x.len())
			.min()
			.unwrap();

		let height = self.field.len();

		(width, height)
	}

	pub fn generate_mesh(&self) -> assets::Model {
		let (w, h) = self.dimensions();
		let mut vertices = alloc::vec![];
		let mut indices = alloc::vec![];

		for z in 0..h {
			for x in 0..w {
				let piece = &self.field[z][x];

				if piece.is_empty() { continue }

				// Generate horizontal wall

				{
					let (tx1, ty1, tx2, ty2) = piece.horiz_top_texture();
					let (tx1, ty1, tx2, ty2) = (tx1 + 0.0001, ty1 + 0.0001, tx2 - 0.0001, ty2 - 0.0001);

					// Create 4 points

					let pos_start = vertices.len() as u16 / 8;

					for inc_z in 0..=1 {
						for inc_x in 0..=1 {
							vertices.push(if inc_x != 0 { tx1 } else { tx2 });
							vertices.push(if inc_z == 0 { ty1 } else { ty2 });

							// Normals will always point up
							vertices.push(0.0);
							vertices.push(1.0);
							vertices.push(0.0);

							vertices.push((x + inc_x) as f32 * self.scale);
							vertices.push(0.0);
							vertices.push((z + inc_z) as f32 * self.scale);
						}
					}

					// Create 2 CCW polygons

					indices.push(pos_start + 2);
					indices.push(pos_start + 1);
					indices.push(pos_start);

					indices.push(pos_start + 2);
					indices.push(pos_start + 3);
					indices.push(pos_start + 1);
				}

				// Generate horizontal bottom wall

				{
					let (tx1, ty1, tx2, ty2) = piece.horiz_bottom_texture();
					let (tx1, ty1, tx2, ty2) = (tx1 + 0.0001, ty1 + 0.0001, tx2 - 0.0001, ty2 - 0.0001);

					// Create 4 points

					let pos_start = vertices.len() as u16 / 8;

					for inc_z in 0..=1 {
						for inc_x in 0..=1 {
							vertices.push(if inc_x == 0 { tx1 } else { tx2 });
							vertices.push(if inc_z == 0 { ty1 } else { ty2 });

							// Normals will always point down
							vertices.push(0.0);
							vertices.push(-1.0);
							vertices.push(0.0);

							vertices.push((x + inc_x) as f32 * self.scale);
							vertices.push(-self.height);
							vertices.push((z + inc_z) as f32 * self.scale);
						}
					}

					// Create 2 CCW polygons

					indices.push(pos_start + 2);
					indices.push(pos_start);
					indices.push(pos_start + 1);

					indices.push(pos_start + 2);
					indices.push(pos_start + 1);
					indices.push(pos_start + 3);
				}

				// Generate up to 4 vertical walls

				{
					let (tx1, ty1, tx2, ty2) = piece.vert_texture();
					let (tx1, ty1, tx2, ty2) = (tx1 + 0.0001, ty1 + 0.0001, tx2 - 0.0001, ty2 - 0.0001);

					let left_wall = match x {
						0 => true,
						x => self.field[z][x - 1].is_empty()
					};

					let right_wall = match x {
						x if x == w - 1 => true,
						x => self.field[z][x + 1].is_empty()
					};

					let front_wall = match z {
						0 => true,
						z => self.field[z - 1][x].is_empty()
					};

					let back_wall = match z {
						z if z == h - 1 => true,
						z => self.field[z + 1][x].is_empty()
					};
					

					let mut generate_wall = |base: [usize; 2], diff: [usize; 2], normal: [f32; 3], reverse: bool| {
						// Create 4 points

						let pos_start = vertices.len() as u16 / 8;

						for inc_y in 0..=1 {
							for inc_xz in 0..=1 {
								vertices.push(if (inc_xz == 0) == reverse { tx1 } else { tx2 });
								vertices.push(if inc_y == 0 { ty1 } else { ty2 });

								vertices.extend_from_slice(&normal);

								vertices.push((base[0] + inc_xz * diff[0]) as f32 * self.scale);
								vertices.push(if inc_y > 0 { 0.0 } else { -self.height });
								vertices.push((base[1] + inc_xz * diff[1]) as f32 * self.scale);
							}
						}

						// Create 2 CCW polygons

						if reverse {
							indices.push(pos_start + 2);
							indices.push(pos_start);
							indices.push(pos_start + 1);

							indices.push(pos_start + 3);
							indices.push(pos_start + 2);
							indices.push(pos_start + 1);
						} else {
							indices.push(pos_start + 2);
							indices.push(pos_start + 1);
							indices.push(pos_start);

							indices.push(pos_start + 3);
							indices.push(pos_start + 1);
							indices.push(pos_start + 2);
						}
					};

					if left_wall {
						generate_wall([ x, z ], [ 0, 1 ], [ -1.0, 0.0, 0.0 ], true);
					}

					if right_wall {
						generate_wall([ x + 1, z ], [ 0, 1 ], [ 1.0, 0.0, 0.0 ], false);
					}

					if front_wall {
						generate_wall([ x, z ], [ 1, 0 ], [ 0.0, 0.0, -1.0 ], false);
					}

					if back_wall {
						generate_wall([ x, z + 1 ], [ 1, 0 ], [ 0.0, 0.0, 1.0 ], true);
					}
				}
			}
		}

		assets::Model {
			vertices,
			indices
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TestPiece {
	__,
	Stone,
	Brick,
	Grass
}

impl PlayfieldPiece for TestPiece {
	fn is_empty(&self) -> bool {
		*self == Self::__
	}

	fn vert_texture(&self) -> (f32, f32, f32, f32) {
		match self {
			TestPiece::Stone => (0.0, 0.875, 0.0625, 0.9375),
			TestPiece::Brick => (0.4375, 0.9375, 0.5, 1.0),
			TestPiece::Grass => (0.1875, 0.9375, 0.25, 1.0),
			TestPiece::__ => unreachable!(),
		}
	}

	fn horiz_top_texture(&self) -> (f32, f32, f32, f32) {
		match self {
			TestPiece::Stone => (0.0, 0.875, 0.0625, 0.9375),
			TestPiece::Brick => (0.4375, 0.9375, 0.5, 1.0),
			TestPiece::Grass => (0.0, 0.9375, 0.0625, 1.0),
			TestPiece::__ => unreachable!(),
		}
	}

	fn horiz_bottom_texture(&self) -> (f32, f32, f32, f32) {
		match self {
			TestPiece::Stone => (0.0, 0.875, 0.0625, 0.9375),
			TestPiece::Brick => (0.4375, 0.9375, 0.5, 1.0),
			TestPiece::Grass => (0.125, 0.9375, 0.1875, 1.0),
			TestPiece::__ => unreachable!(),
		}
	}
}

use TestPiece::*;

pub const EXAMPLE_MAZE: Playfield<TestPiece> = Playfield {
	scale: 3.0,
	height: 3.0,
	death_barrier: -20.0,
	field: &[
		&[Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass],
		&[Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass],
		&[Grass, Grass, __,    __,    __,    __,    __,    __,    __,    __,    __,    Grass, Grass],
		&[Grass, Grass, __,    Brick, Brick, __,    __,    __,    Brick, Brick, __,    Grass, Grass],
		&[Grass, Grass, __,    Brick, __,    __,    __,    __,    __,    Brick, __,    Grass, Grass],
		&[Grass, Grass, __,    __,    __,    Stone, Stone, Stone, __,    __,    __,    Grass, Grass],
		&[Grass, Grass, __,    __,    __,    Stone, Stone, Stone, __,    __,    __,    Grass, Grass],
		&[Grass, Grass, __,    __,    __,    Stone, Stone, Stone, __,    __,    __,    Grass, Grass],
		&[Grass, Grass, __,    Brick, __,    __,    __,    __,    __,    Brick, __,    Grass, Grass],
		&[Grass, Grass, __,    Brick, Brick, __,    __,    __,    Brick, Brick, __,    Grass, Grass],
		&[Grass, Grass, __,    __,    __,    __,    __,    __,    __,    __,    __,    Grass, Grass],
		&[Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass],
		&[Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass, Grass],
	],
	player_spawn_point: [6, 6],
	powerup_spawn_points: &[
		[0, 0],
		[12, 0],
		[12, 12],
		[0, 12]
	],
	enemy_spawn_points: &[
		// [5, 0],
		// [5, 10],
		// [0, 5],
		// [10, 5],
		[3, 3],
		[3, 9],
		[9, 9],
		[9, 3]
	]
};

