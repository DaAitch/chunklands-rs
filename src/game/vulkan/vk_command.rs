use super::util::copy_extent_2d;
use super::{
    error::{to_vulkan, Result},
    vk_queue::QueueFamilyIndices,
};
use vk_sys as vk;
use vulkanic::DevicePointers;

pub fn create_command_pool(
    dp: &DevicePointers,
    device: vk::Device,
    queue_family_indices: &QueueFamilyIndices,
) -> Result<vk::CommandPool> {
    let info = vk::CommandPoolCreateInfo {
        sType: vk::STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        queueFamilyIndex: queue_family_indices.graphics,
    };

    unsafe { dp.create_command_pool(device, &info) }.map_err(to_vulkan)
}

pub fn create_command_buffer(
    dp: &DevicePointers,
    device: vk::Device,
    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
    command_pool: vk::CommandPool,
    framebuffer: vk::Framebuffer,
    extent: &vk::Extent2D,
    vertex_buffer: vk::Buffer,
) -> Result<vk::CommandBuffer> {
    let command_buffer = {
        let info = vk::CommandBufferAllocateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            pNext: std::ptr::null(),
            commandPool: command_pool,
            level: vk::COMMAND_BUFFER_LEVEL_PRIMARY,
            commandBufferCount: 1,
        };

        let b = unsafe { dp.allocate_command_buffers(device, &info) }.map_err(to_vulkan)?;
        *b.iter().next().unwrap()
    };

    {
        let info = vk::CommandBufferBeginInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            pInheritanceInfo: std::ptr::null(),
        };

        unsafe { dp.begin_command_buffer(command_buffer, &info) }.map_err(to_vulkan)?;
    }

    {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        }];

        let info = vk::RenderPassBeginInfo {
            sType: vk::STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
            pNext: std::ptr::null(),
            renderPass: render_pass,
            framebuffer,
            renderArea: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: copy_extent_2d(extent),
            },
            clearValueCount: clear_values.len() as u32,
            pClearValues: clear_values.as_ptr(),
        };

        unsafe { dp.cmd_begin_render_pass(command_buffer, &info, vk::SUBPASS_CONTENTS_INLINE) };
    }

    dp.cmd_bind_pipeline(command_buffer, vk::PIPELINE_BIND_POINT_GRAPHICS, pipeline);

    let vertex_buffers = [vertex_buffer];
    let offsets: [vk::DeviceSize; 1] = [0];
    dp.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);

    dp.cmd_draw(command_buffer, 3, 1, 0, 0);
    dp.cmd_end_render_pass(command_buffer);

    dp.end_command_buffer(command_buffer).map_err(to_vulkan)?;

    Ok(command_buffer)
}
