use crate::debugging::*;
use std::ptr::null;
use std::ptr::copy_nonoverlapping;
use std::os::raw::c_void;
use std::collections::{HashMap, HashSet};
use rand::{random, Rng};
use rand::prelude::Distribution;
use rand::distributions::Standard;
use crate::chunk_manager::{CHUNK_VOLUME, CHUNK_SIZE};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum BlockID {
    AIR,
    DIRT,
    COBBLESTONE,
    OBSIDIAN,
}

impl Distribution<BlockID> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BlockID {
        match rng.gen_range(1, 4) {
            // 0 => BlockID::AIR,
            1 => BlockID::DIRT,
            2 => BlockID::COBBLESTONE,
            3 => BlockID::OBSIDIAN,
            _ => BlockID::AIR,
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
    gl_call!(gl::VertexArrayAttribFormat(vao, 1, 2 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(vao, 1, 0));

    let mut vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut vbo));
    gl_call!(gl::NamedBufferData(vbo,
            (180 * std::mem::size_of::<f32>() * CHUNK_VOLUME as usize) as isize,
            null(),
            gl::DYNAMIC_DRAW));

    gl_call!(gl::VertexArrayVertexBuffer(vao, 0, vbo, 0, (5 * std::mem::size_of::<f32>()) as i32));
    (vao, vbo)
}

pub struct Chunk {
    blocks: [BlockID; CHUNK_VOLUME as usize],
    pub vao: u32,
    pub vbo: u32,
    pub vertices_drawn: u32,
    pub dirty: bool,
    pub dirty_neighbours: HashSet<(i32, i32, i32)>
}

impl Chunk {
    fn all_neighbours() -> HashSet<(i32, i32, i32)> {
        let mut hs = HashSet::new();
        hs.insert((1, 0, 0));
        hs.insert((0, 1, 0));
        hs.insert((0, 0, 1));
        hs.insert((-1, 0, 0));
        hs.insert((0, -1, 0));
        hs.insert((0, 0, -1));
        hs
    }

    pub fn empty() -> Chunk {
        let (vao, vbo) = create_vao_vbo();

        Chunk {
            blocks: [BlockID::AIR; CHUNK_VOLUME as usize],
            vao,
            vbo,
            vertices_drawn: 0,
            dirty: true,
            dirty_neighbours: Chunk::all_neighbours(),
        }
    }

    pub fn full_of_block(block: BlockID) -> Chunk {
        let (vao, vbo) = create_vao_vbo();

        Chunk {
            blocks: [block; CHUNK_VOLUME as usize],
            vao,
            vbo,
            vertices_drawn: 0,
            dirty: true,
            dirty_neighbours: Chunk::all_neighbours(),
        }
    }

    pub fn random() -> Chunk {
        let (vao, vbo) = create_vao_vbo();

        let mut c = Chunk {
            blocks: [BlockID::AIR; CHUNK_VOLUME as usize],
            vao,
            vbo,
            vertices_drawn: 0,
            dirty: true,
            dirty_neighbours: Chunk::all_neighbours(),
        };

        for i in 0..c.blocks.len() {
            c.blocks[i] = random::<BlockID>();
        }
        c
    }

    #[inline]
    fn coords_to_index(x: u32, y: u32, z: u32) -> usize {
        (y * (CHUNK_SIZE * CHUNK_SIZE) + z * CHUNK_SIZE + x) as usize
    }

    // #[inline]
    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockID {
        self.blocks[Chunk::coords_to_index(x, y, z)]
    }

    // #[inline]
    pub fn set_block(&mut self, block: BlockID, x: u32, y: u32, z: u32) {
        self.blocks[Chunk::coords_to_index(x, y, z)] = block;
        self.dirty = true;
        if x == 0 {
            self.dirty_neighbours.insert((-1, 0, 0));
        } else if x == 15 {
            self.dirty_neighbours.insert((1, 0, 0));
        }
        if y == 0 {
            self.dirty_neighbours.insert((0, -1, 0));
        } else if y == 15 {
            self.dirty_neighbours.insert((0, 1, 0));
        }
        if z == 0 {
            self.dirty_neighbours.insert((0, 0, -1));
        } else if z == 15 {
            self.dirty_neighbours.insert((0, 0, 1));
        }
    }
}