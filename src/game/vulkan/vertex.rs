use std::mem::size_of;

use vk_sys as vk;
use memoffset::offset_of;

#[repr(C)]
pub struct Vertex {
    pub pos: glm::Vec2,
    pub color: glm::Vec3,
}

impl Vertex {
    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as u32,
            inputRate: vk::VERTEX_INPUT_RATE_VERTEX,
        }
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::FORMAT_R32G32_SFLOAT,
                offset: offset_of!(Self, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::FORMAT_R32G32B32_SFLOAT,
                offset: offset_of!(Self, color) as u32,
            }
        ]
    }
}