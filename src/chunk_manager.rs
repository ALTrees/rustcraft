use std::collections::{HashMap, HashSet};

use nalgebra::Matrix4;
use nalgebra_glm::{Mat4, vec3};

use crate::ambient_occlusion::compute_ao_of_block;
use crate::chunk::{BlockID, Chunk, ChunkColumn};
use crate::shader_compilation::ShaderProgram;
use std::sync::Arc;
use parking_lot::RwLock;
use owning_ref::OwningRef;

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_VOLUME: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Default)]
pub struct ChunkManager {
    pub loaded_chunk_columns: RwLock<HashMap<(i32, i32), Arc<ChunkColumn>>>,
    pub(crate) block_changelist: RwLock<HashSet<(i32, BlockID, i32, i32, i32)>>,
}

impl ChunkManager {
    pub fn new() -> ChunkManager {
        ChunkManager {
            loaded_chunk_columns: RwLock::new(HashMap::new()),
            block_changelist: RwLock::new(HashSet::new()),
        }
    }

    #[inline]
    pub fn get_column(&self, x: i32, z: i32) -> Option<Arc<ChunkColumn>> {
        self.loaded_chunk_columns.read().get(&(x, z)).map(|col| Arc::clone(col))
    }

    #[inline]
    pub fn get_chunk(&self, x: i32, y: i32, z: i32) -> Option<OwningRef<Arc<ChunkColumn>, Chunk>> {
        if y < 0 || y >= 16 {
            return None;
        }
        self.loaded_chunk_columns.read().get(&(x, z))
            .map(|column| {
                OwningRef::new(Arc::clone(column)).map(|column| column.get_chunk(y))
            })
    }

    #[inline]
    pub fn add_chunk_column(&self, xz: (i32, i32), chunk_column: Arc<ChunkColumn>) {
        let mut guard = self.loaded_chunk_columns.write();
        if !guard.contains_key(&xz) {
            guard.insert(xz, chunk_column);
        }
    }

    #[inline]
    pub fn remove_chunk_column(&self, xz: &(i32, i32)) -> Option<Arc<ChunkColumn>> {
        self.loaded_chunk_columns.write().remove(&xz)
    }

    pub fn preload_some_chunks(&mut self) {
        for z in 0..2 {
            for x in 0..2 {
                self.add_chunk_column((x, z), Arc::new(ChunkColumn::new()));
            }
        }
    }

    pub fn single(&mut self) {
        self.add_chunk_column((0, 0), Arc::new(ChunkColumn::new()));
        self.set_block(BlockID::Cobblestone, 0, 0, 0);
    }

    pub fn single_chunk(&mut self) {
        self.add_chunk_column((0, 0), Arc::new(ChunkColumn::full_of_block(BlockID::Cobblestone)));
    }

    // Transform global block coordinates into chunk local coordinates
    pub fn get_chunk_coords(x: i32, y: i32, z: i32) -> (i32, i32, i32, u32, u32, u32) {
        let chunk_x = if x < 0 { (x + 1) / 16 - 1 } else { x / 16 };
        let chunk_y = if y < 0 { (y + 1) / 16 - 1 } else { y / 16 };
        let chunk_z = if z < 0 { (z + 1) / 16 - 1 } else { z / 16 };

        let block_x = x.rem_euclid(16) as u32;
        let block_y = y.rem_euclid(16) as u32;
        let block_z = z.rem_euclid(16) as u32;

        (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
    }

    // Transform chunk local coordinates into global coordinates
    pub fn get_global_coords((chunk_x, chunk_y, chunk_z, block_x, block_y, block_z): (i32, i32, i32, u32, u32, u32)) -> (i32, i32, i32) {
        let x = 16 * chunk_x + block_x as i32;
        let y = 16 * chunk_y + block_y as i32;
        let z = 16 * chunk_z + block_z as i32;
        (x, y, z)
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<BlockID> {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_coords(x, y, z);

        self.get_chunk(chunk_x, chunk_y, chunk_z)
            .map(|chunk|
                chunk.get_block(block_x, block_y, block_z))
    }

    /// Replaces the block at (x, y, z) with `block`.
    fn _set_block(&self, priority: i32, block: BlockID, x: i32, y: i32, z: i32) -> bool {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_coords(x, y, z);

        match self.get_chunk(chunk_x, chunk_y, chunk_z) {
            None => false,
            Some(chunk) => {
                chunk.set_block(block, block_x, block_y, block_z);
                if *chunk.is_uploaded_to_gpu.read() {
                    self.block_changelist.write().insert((priority, block, x, y, z));
                }
                true
            }
        }
    }

    pub fn set_block(&self, block: BlockID, x: i32, y: i32, z: i32) -> bool {
        self._set_block(0, block, x, y, z)
    }

    pub fn put_block(&self, block: BlockID, x: i32, y: i32, z: i32) -> bool {
        self._set_block(1, block, x, y, z)
    }

    pub fn is_solid_block_at(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y, z)
            .filter(|&block| block != BlockID::Air)
            .is_some()
    }

    pub fn update_blocks<I>(&self, c_x: i32, c_y: i32, c_z: i32, blocks: I)
        where I: Iterator<Item = (u32, u32, u32)> {

        let this_column = match self.get_column(c_x, c_z) {
            Some(column) => column,
            None => {
                error!("Cannot update chunk {:?} because its column doesn't exist", (c_x, c_y, c_z));
                return;
            }
        };
        let this_chunk = this_column.get_chunk(c_y);
        if this_chunk.is_empty() {
            return;
        }

        let mut neighbourhood = [
            None, None, None,
            None, None, None,
            None, None, None];
        for x in -1..=1 {
            for z in -1..=1 {
                neighbourhood[3 * (x + 1) as usize + (z + 1) as usize] = if x == 0 && z == 0 {
                    None
                } else {
                    self.get_column(c_x + x, c_z + z)
                };
            }
        }

        #[inline]
        fn block_at(column: &ChunkColumn, neighbourhood: &[Option<Arc<ChunkColumn>>; 9], c_x: i32, c_z: i32, w_x: i32, w_y: i32, w_z: i32) -> BlockID {
            let to_index = |x: i32, z: i32| -> usize {
                3 * (x - c_x + 1) as usize + (z - c_z + 1) as usize
            };

            let (c_x_n, c_y_n, c_z_n, b_x, b_y, b_z) = ChunkManager::get_chunk_coords(w_x, w_y, w_z);

            if c_y_n < 0 || c_y_n >= 16 {
                return BlockID::Air;
            }

            if c_x == c_x_n && c_z == c_z_n {
                column.get_chunk(c_y_n).get_block(b_x, b_y, b_z)
            } else {
                if let Some(neighbour_column) = neighbourhood[to_index(c_x_n, c_z_n)].as_ref() {
                    neighbour_column.get_chunk(c_y_n).get_block(b_x, b_y, b_z)
                } else {
                    BlockID::Air
                }
            }
        };

        #[inline]
        fn compute_active_faces(column: &ChunkColumn, neighbourhood: &[Option<Arc<ChunkColumn>>; 9], c_x: i32, c_z: i32, x: i32, y: i32, z: i32) -> [bool; 6] {
            let right = block_at(&column, &neighbourhood, c_x, c_z, x + 1, y, z).is_transparent();
            let left = block_at(&column, &neighbourhood, c_x, c_z, x - 1, y, z).is_transparent();
            let top = block_at(&column, &neighbourhood, c_x, c_z, x, y + 1, z).is_transparent();
            let bottom = block_at(&column, &neighbourhood, c_x, c_z, x, y - 1, z).is_transparent();
            let front = block_at(&column, &neighbourhood, c_x, c_z, x, y, z + 1).is_transparent();
            let back = block_at(&column, &neighbourhood, c_x, c_z, x, y, z - 1).is_transparent();
            [right, left, top, bottom, front, back]
        };

        let mut active_faces = this_chunk.active_faces.write();
        let mut ao_vertices = this_chunk.ao_vertices.write();

        for (b_x, b_y, b_z) in blocks {
            if this_chunk.get_block(b_x, b_y, b_z) == BlockID::Air {
                continue;
            }
            let (w_x, w_y, w_z) = ChunkManager::get_global_coords((c_x, c_y, c_z, b_x, b_y, b_z));

            let af = compute_active_faces(&this_column, &neighbourhood, c_x, c_z, w_x, w_y, w_z);
            let array_index = (b_y * CHUNK_SIZE * CHUNK_SIZE + b_z * CHUNK_SIZE + b_x) as usize;

            active_faces.set(6 * array_index, af[0]);
            active_faces.set(6 * array_index + 1, af[1]);
            active_faces.set(6 * array_index + 2, af[2]);
            active_faces.set(6 * array_index + 3, af[3]);
            active_faces.set(6 * array_index + 4, af[4]);
            active_faces.set(6 * array_index + 5, af[5]);

            // Ambient Occlusion

            let block_ao = compute_ao_of_block(&|rx: i32, ry: i32, rz: i32| {
                !block_at(&this_column, &neighbourhood, c_x, c_z, w_x + rx, w_y + ry, w_z + rz).is_transparent_no_leaves()
            });

            ao_vertices[array_index] = block_ao;
        }
    }

    pub fn update_block(&self, c_x: i32, c_y: i32, c_z: i32, b_x: u32, b_y: u32, b_z: u32) {
        let chunk = self.get_chunk(c_x, c_y, c_z).unwrap();
        if chunk.get_block(b_x, b_y, b_z) == BlockID::Air {
            return;
        }

        let (w_x, w_y, w_z) = ChunkManager::get_global_coords((c_x, c_y, c_z, b_x, b_y, b_z));
        let array_index = (b_y * CHUNK_SIZE * CHUNK_SIZE + b_z * CHUNK_SIZE + b_x) as usize;

        let active_faces_of_block = self.get_active_faces_of_block(w_x, w_y, w_z);
        {
            let mut active_faces = chunk.active_faces.write();
            active_faces.set(6 * array_index, active_faces_of_block[0]);
            active_faces.set(6 * array_index + 1, active_faces_of_block[1]);
            active_faces.set(6 * array_index + 2, active_faces_of_block[2]);
            active_faces.set(6 * array_index + 3, active_faces_of_block[3]);
            active_faces.set(6 * array_index + 4, active_faces_of_block[4]);
            active_faces.set(6 * array_index + 5, active_faces_of_block[5]);
        }

        // Ambient Occlusion

        let block_ao = compute_ao_of_block(&|rx: i32, ry: i32, rz: i32| {
            self.get_block(w_x + rx, w_y + ry, w_z + rz)
                .filter(|b| !b.is_transparent_no_leaves())
                .is_some()
        });
        self.get_chunk(c_x, c_y, c_z).unwrap().ao_vertices.write()[array_index] = block_ao;
    }

    // An active face is a block face next to a transparent block that needs to be rendered
    pub fn get_active_faces_of_block(&self, x: i32, y: i32, z: i32) -> [bool; 6] {
        let right = self.get_block(x + 1, y, z).filter(|&b| !b.is_transparent()).is_none();
        let left = self.get_block(x - 1, y, z).filter(|&b| !b.is_transparent()).is_none();
        let top = self.get_block(x, y + 1, z).filter(|&b| !b.is_transparent()).is_none();
        let bottom = self.get_block(x, y - 1, z).filter(|&b| !b.is_transparent()).is_none();
        let front = self.get_block(x, y, z + 1).filter(|&b| !b.is_transparent()).is_none();
        let back = self.get_block(x, y, z - 1).filter(|&b| !b.is_transparent()).is_none();
        [right, left, top, bottom, front, back]
    }

    pub fn render_loaded_chunks(&self, program: &mut ShaderProgram) {
        for ((x, z), chunk_column) in self.loaded_chunk_columns.read().iter() {
            for (ref y, chunk) in chunk_column.chunks.iter().enumerate() {
                // Skip rendering the chunk if there is nothing to draw
                let vao = *chunk.vao.read();
                if !*chunk.is_uploaded_to_gpu.read() || chunk.is_empty() || vao == 0 {
                    continue;
                }

                let model_matrix = {
                    let translate_matrix = Matrix4::new_translation(&vec3(
                        *x as f32, *y as f32, *z as f32).scale(16.0));
                    let rotate_matrix = Matrix4::from_euler_angles(
                        0.0f32,
                        0.0,
                        0.0,
                    );
                    let scale_matrix: Mat4 = Matrix4::new_nonuniform_scaling(&vec3(1.0f32, 1.0f32, 1.0f32));
                    translate_matrix * rotate_matrix * scale_matrix
                };

                gl_call!(gl::BindVertexArray(vao));
                if vao == 0 {
                    dbg!(vao);
                    dbg!(*chunk.is_uploaded_to_gpu.read());
                    dbg!(chunk.is_empty());
                }
                program.set_uniform_matrix4fv("model", model_matrix.as_ptr());
                gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, *chunk.vertices_drawn.read() as i32));
            }
        }
    }
}