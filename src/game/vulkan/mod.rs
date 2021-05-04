//! design good practise:
//! - using low-level `vulkanic` lib reducing some boilerplate, like error handling from C to Rust,
//!   out-parameters
//! - functions, that wrap vulkan boilerplate code, not in a generic fashion, but specialized for
//!   the project
//!
//! design bad practise:
//! - wrap vulkan API calls into easier calls, omitting pNext, sType, etc.
//!   => some data structures are dynamic size and we don't want to do DMA only for mapping a
//!      "nicer" userland API the vulkan API, which is not worth it.
//!      Userland should try to store data in a suitable structure, ready for fast vulkan API
//!      calls.
//! -

mod context;
mod error;
mod setup;
mod swapchain;
mod util;
mod version;
mod vertex;

use error::Result;
use vulkanic::{DevicePointers, InstancePointers};

use vk_sys as vk;

use self::error::to_vulkan;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct VulkanInit<'a> {
    pub debug: bool,
    pub window: &'a mut glfw::Window,
    pub req_ext: &'a Vec<String>,
    pub req_layers: &'a Vec<String>,
}

pub struct Vulkan {
    ctx: Context,
    sc_ctx: Option<Swapchain>,
    inflight_frames: Vec<InFlightFrame>,
    current_frame: usize,
}

impl Vulkan {
    pub fn wait_idle(&mut self) -> Result<()> {
        self.ctx
            .dp
            .queue_wait_idle(self.ctx.queue_families.present_queue)
            .map_err(to_vulkan)
    }
}

struct Context {
    ip: InstancePointers,
    dp: DevicePointers,
    instance: vk::Instance,
    debugger: vk::DebugUtilsMessengerEXT,
    physical_device: vk::PhysicalDevice,
    device: vk::Device,
    queue_family_indices: QueueFamilyIndices,
    queue_families: QueueFamilies,
    surface: vk::SurfaceKHR,
    command_pool: vk::CommandPool,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
}

#[derive(Debug)]
pub struct QueueFamilies {
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

#[derive(Debug)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
    pub present: u32,
}

struct SwapchainContext {
    swapchain: vk::SwapchainKHR,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    vertex_shader_module: vk::ShaderModule,
    fragment_shader_module: vk::ShaderModule,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    extent: vk::Extent2D,
    surface_format: vk::SurfaceFormatKHR,
}
struct Swapchain {
    images: Vec<SwapchainImage>,
    ctx: SwapchainContext,
}

struct SwapchainImage {
    image_view: vk::ImageView,
    framebuffer: vk::Framebuffer,
    command_buffer: vk::CommandBuffer,
    in_flight_fence: vk::Fence,
}

struct InFlightFrame {
    available_semaphore: vk::Semaphore,
    rendered_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
}
