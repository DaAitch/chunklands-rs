use super::util::copy_extent_2d;
use super::{error::to_vulkan, Context};
use super::{Result, SwapchainContext};
use vk_sys as vk;

impl Context {
    pub fn allocate_primary_command_buffer(&self) -> Result<vk::CommandBuffer> {
        let command_buffers = unsafe {
            self.dp
                .allocate_command_buffers(
                    self.device,
                    &vk::CommandBufferAllocateInfo {
                        sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
                        pNext: std::ptr::null(),
                        commandPool: self.command_pool,
                        level: vk::COMMAND_BUFFER_LEVEL_PRIMARY,
                        commandBufferCount: 1,
                    },
                )
                .map_err(to_vulkan)
        }?;

        Ok(command_buffers.iter().cloned().next().unwrap())
    }

    pub fn begin_command_buffer(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        unsafe {
            self.dp
                .begin_command_buffer(
                    command_buffer,
                    &vk::CommandBufferBeginInfo {
                        sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        pInheritanceInfo: std::ptr::null(),
                    },
                )
                .map_err(to_vulkan)
        }
    }

    pub fn begin_render_pass(
        &self,
        sc_ctx: &SwapchainContext,
        command_buffer: vk::CommandBuffer,
        framebuffer: vk::Framebuffer,
    ) {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        }];

        let info = vk::RenderPassBeginInfo {
            sType: vk::STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
            pNext: std::ptr::null(),
            renderPass: sc_ctx.render_pass,
            framebuffer,
            renderArea: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: copy_extent_2d(&sc_ctx.extent),
            },
            clearValueCount: clear_values.len() as u32,
            pClearValues: clear_values.as_ptr(),
        };

        unsafe {
            self.dp
                .cmd_begin_render_pass(command_buffer, &info, vk::SUBPASS_CONTENTS_INLINE)
        };
    }

    pub fn cmd_bind_pipeline(&self, sc_ctx: &SwapchainContext, command_buffer: vk::CommandBuffer) {
        self.dp.cmd_bind_pipeline(
            command_buffer,
            vk::PIPELINE_BIND_POINT_GRAPHICS,
            sc_ctx.pipeline,
        );
    }

    pub fn create_semaphore(&self) -> Result<vk::Semaphore> {
        unsafe {
            self.dp.create_semaphore(
                self.device,
                &vk::SemaphoreCreateInfo {
                    sType: vk::STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                },
            )
        }
        .map_err(to_vulkan)
    }

    pub fn destroy_semaphore(&self, semaphore: vk::Semaphore) {
        self.dp.destroy_semaphore(self.device, semaphore);
    }

    pub fn destory_fence(&self, fence: vk::Fence) {
        self.dp.destroy_fence(self.device, fence);
    }

    pub fn create_signaled_fence(&self) -> Result<vk::Fence> {
        unsafe {
            self.dp.create_fence(
                self.device,
                &vk::FenceCreateInfo {
                    sType: vk::STRUCTURE_TYPE_FENCE_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: vk::FENCE_CREATE_SIGNALED_BIT,
                },
            )
        }
        .map_err(to_vulkan)
    }
}
