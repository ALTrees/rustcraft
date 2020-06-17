use bit_vec::BitVec;
use rand::{random, Rng};
use rand::distributions::Standard;
use rand::prelude::Distribution;

use crate::chunk_manager::{CHUNK_SIZE, CHUNK_VOLUME};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum BlockID {
    Air,
    Dirt,
    GrassBlock,
    Stone,
    Cobblestone,
    Bedrock,
    Obsidian,
    OakLog,
    OakLeaves,
    OakPlanks,
    Glass,
    Urss,
    Hitler,
    Debug,
    Debug2,
}

impl BlockID {
    pub fn is_air(&self) -> bool {
        self == &BlockID::Air
    }
    pub fn is_transparent(&self) -> bool {
        match self {
            &BlockID::Air |
            &BlockID::OakLeaves |
            &BlockID::Glass => true,
            _ => false
        }
    }
    pub fn is_transparent_no_leaves(&self) -> bool {
        match self {
            &BlockID::Air |
            &BlockID::Glass => true,
            _ => false
        }
    }
}

impl Distribution<BlockID> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BlockID {
        match rng.gen_range(1, 4) {
            // 0 => BlockID::AIR,
            1 => BlockID::Dirt,
            2 => BlockID::Cobblestone,
            3 => BlockID::Obsidian,
            _ => BlockID::Air,
        }
    }
}

fn create_vao_vbo() -> (u32, u32) {
    let mut vao = 0;
    gl_call!(gl::CreateVertexArrays(1, &mut vao));

    // Position
    gl_call!(gl::EnableVertexArrayAttrib(vao, 0));
    gl_call!(gl::VertexArrayAttribFormat(vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    gl_call!(gl::VertexArrayAttribBinding(vao, 0, 0));

    // Texture coords
    gl_call!(gl::EnableVertexArrayAttrib(vao, 1));
    gl_call!(gl::VertexArrayAttribFormat(vao, 1, 3 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(vao, 1, 0));

    // Normals
    gl_call!(gl::EnableVertexArrayAttrib(vao, 2));
    gl_call!(gl::VertexArrayAttribFormat(vao, 2, 3 as i32, gl::FLOAT, gl::FALSE, 6 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(vao, 2, 0));

    // Ambient occlusion
    gl_call!(gl::EnableVertexArrayAttrib(vao, 3));
    gl_call!(gl::VertexArrayAttribFormat(vao, 3, 1 as i32, gl::FLOAT, gl::FALSE, 9 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(vao, 3, 0));

    let mut vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut vbo));
    // We intentionally don't initialize the buffer's data store because it's dynamically created
    // when the chunk is invalidated

    gl_call!(gl::VertexArrayVertexBuffer(vao, 0, vbo, 0, (10 * std::mem::size_of::<f32>()) as i32));
    (vao, vbo)
}

pub struct ChunkColumn {
    pub chunks: [Chunk; 16],
}

impl ChunkColumn {
    pub fn new() -> Self {
        Self {
            chunks: [
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
            ]
        }
    }

    pub fn random() -> Self {
        Self {
            chunks: [
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
            ],
        }
    }

    pub fn full_of_block(block: BlockID) -> Self {
        Self {
            chunks: [
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
            ],
        }
    }

    pub fn alternating() -> Self {
        Self {
            chunks: [
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
            ],
        }
    }

    pub fn set_block(&mut self, block: BlockID, x: u32, y: u32, z: u32) {
        self.chunks[(y / 16) as usize].set_block(block, x, y % 16, z);
    }
}

pub struct Chunk {
    pub blocks: [BlockID; CHUNK_VOLUME as usize],
    pub active_faces: BitVec,
    pub ao_vertices: [[[u8; 4]; 6]; CHUNK_VOLUME as usize],
    pub needs_complete_rebuild: bool,

    pub vao: u32,
    pub vbo: u32,
    pub vertices_drawn: u32,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::empty()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self::empty()
    }

    /// Creates a chunk where every block is the same
    pub fn full_of_block(block: BlockID) -> Self {
        let (vao, vbo) = create_vao_vbo();

        Self {
            blocks: [block; CHUNK_VOLUME as usize],
            active_faces: BitVec::from_elem(6 * CHUNK_VOLUME as usize, false),
            ao_vertices: [[[0; 4]; 6]; CHUNK_VOLUME as usize],
            needs_complete_rebuild: true,

            vao,
            vbo,
            vertices_drawn: 0,
        }
    }

    /// Creates an empty chunk with no blocks
    pub fn empty() -> Self {
        Self::full_of_block(BlockID::Air)
    }

    /// Creates a chunk where every block is random
    pub fn random() -> Self {
        let (vao, vbo) = create_vao_vbo();

        Self {
            blocks: {
                let mut blocks = [BlockID::Air; CHUNK_VOLUME as usize];
                for i in 0..blocks.len() {
                    blocks[i] = random::<BlockID>();
                }
                blocks
            },
            active_faces: BitVec::from_elem(6 * CHUNK_VOLUME as usize, false),
            ao_vertices: [[[0; 4]; 6]; CHUNK_VOLUME as usize],
            needs_complete_rebuild: true,

            vao,
            vbo,
            vertices_drawn: 0,
        }
    }

    #[inline]
    fn chunk_coords_to_array_index(x: u32, y: u32, z: u32) -> usize {
        (y * (CHUNK_SIZE * CHUNK_SIZE) + z * CHUNK_SIZE + x) as usize
    }

    #[inline]
    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockID {
        self.blocks[Chunk::chunk_coords_to_array_index(x, y, z)]
    }

    /// Sets a block at some given coordinates
    /// The coordinates must be within the chunk size
    #[inline]
    pub fn set_block(&mut self, block: BlockID, x: u32, y: u32, z: u32) {
        self.blocks[Chunk::chunk_coords_to_array_index(x, y, z)] = block;
    }
}

/// Iterator that iterates over all possible block coordinates of a chunk on all 3 axis
/// Equivalent in functionality to a triple for loop from 0 to 15 each
pub struct BlockIterator {
    x: u32,
    y: u32,
    z: u32
}

impl BlockIterator {
    pub fn new() -> BlockIterator {
        BlockIterator {
            x: 0,
            y: 0,
            z: 0
        }
    }
}

impl Iterator for BlockIterator {
    type Item = (u32, u32, u32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.y == CHUNK_SIZE {
            None
        } else {
            let to_return = (self.x, self.y, self.z);
            self.x += 1;
            if self.x >= CHUNK_SIZE {
                self.x = 0;
                self.z += 1;
                if self.z >= CHUNK_SIZE {
                    self.z = 0;
                    self.y += 1;
                }
            }
            Some(to_return)
        }
    }
}
