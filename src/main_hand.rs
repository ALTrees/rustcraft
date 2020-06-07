use core::ffi::c_void;

use specs::Component;
use specs::DenseVecStorage;

use crate::chunk::BlockID;
use crate::shapes::centered_unit_cube;
use crate::types::TexturePack;

#[derive(Component)]
pub struct MainHand {
    showing_item: Option<BlockID>,
    pub render: MainHandRender,
}

impl MainHand {
    pub fn new() -> Self {
        Self {
            showing_item: None,
            render: MainHandRender::new(),
        }
    }

    pub fn set_showing_item(&mut self, item: Option<BlockID>) {
        self.showing_item = item;
        self.render.dirty = true;
    }

    pub fn update_if_dirty(&mut self, texture_pack: &TexturePack) {
        if let Some(item) = self.showing_item {
            self.render.update_vbo_if_dirty(item, &texture_pack);
        }
    }
}

pub struct MainHandRender {
    vao: u32,
    pub vbo: u32,
    dirty: bool,
}

impl MainHandRender {
    pub fn new() -> Self {
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

        let mut vbo = 0;
        gl_call!(gl::CreateBuffers(1, &mut vbo));
        gl_call!(gl::VertexArrayVertexBuffer(vao, 0, vbo, 0, (9 * std::mem::size_of::<f32>()) as i32));

        Self {
            vao,
            vbo,
            dirty: true,
        }
    }

    pub fn update_vbo_if_dirty(&mut self, item: BlockID, texture_pack: &TexturePack) {
        if self.dirty {
            self.update_vbo(item, &texture_pack);
            self.dirty = false;
        }
    }

    pub fn update_vbo(&mut self, item: BlockID, texture_pack: &TexturePack) {
        let vbo_data = centered_unit_cube(
            -0.5, -0.5, -0.5,
            texture_pack.get(&item).unwrap().get_uv_of_every_face());

        gl_call!(gl::NamedBufferData(self.vbo,
                    (vbo_data.len() * std::mem::size_of::<f32>() as usize) as isize,
                    vbo_data.as_ptr() as *const c_void,
                    gl::DYNAMIC_DRAW));
    }
}