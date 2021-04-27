mod error;
mod util;
mod version;
mod vertex;
mod vk_command;
mod vk_device;
mod vk_framebuffer;
mod vk_graphics_pipeline;
mod vk_image;
mod vk_instance;
mod vk_physical_device;
mod vk_queue;
mod vk_render_pass;
mod vk_surface;
mod vk_swapchain;
mod vk_vertex_buffer;

use error::{to_other, Error, Result};
use vk_command::{create_command_buffer, create_command_pool};
use vk_device::create_device;
use vk_framebuffer::create_framebuffer;
use vk_graphics_pipeline::create_graphics_pipeline;
use vk_image::create_image_view;
use vk_instance::{create_debug_messenger, create_instance};
use vk_physical_device::find_physical_device;
use vk_queue::{find_queue_families, get_device_queue_families, QueueFamilies, QueueFamilyIndices};
use vk_render_pass::create_render_pass;
use vk_surface::create_surface;
use vk_swapchain::create_swapchain;
use vulkanic::{DevicePointers, EntryPoints, InstancePointers};

use vk_sys as vk;

use self::{error::to_vulkan, vk_vertex_buffer::create_vertex_buffer};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct VulkanInit<'a> {
    pub debug: bool,
    pub window: &'a mut glfw::Window,
    pub req_ext: &'a Vec<String>,
    pub req_layers: &'a Vec<String>,
}

pub struct Vulkan {
    ep: EntryPoints,
    ip: InstancePointers,
    dp: DevicePointers,
    instance: vk::Instance,
    debugger: vk::DebugUtilsMessengerEXT,
    physical_device: vk::PhysicalDevice,
    device: vk::Device,
    queue_family_indices: QueueFamilyIndices,
    queue_families: QueueFamilies,
    surface: vk::SurfaceKHR,
    swapchain: Option<Swapchain>,
    command_pool: vk::CommandPool,
    inflight_frames: Vec<InFlightFrame>,
    current_frame: usize,
}

impl Vulkan {
    pub fn new(init: VulkanInit) -> Result<Self> {
        let ep: EntryPoints = vk::EntryPoints::load(|procname| {
            init.window
                .get_instance_proc_address(0, procname.to_str().unwrap())
        })
        .into();

        let instance = create_instance(&ep, init.req_layers, init.req_ext, init.debug)?;
        let ip: InstancePointers = vk::InstancePointers::load(|procname| {
            init.window
                .get_instance_proc_address(instance, procname.to_str().unwrap())
        })
        .into();
        let dp: DevicePointers = vk::DevicePointers::load(|procname| {
            init.window
                .get_instance_proc_address(instance, procname.to_str().unwrap())
        })
        .into();

        let debugger = if init.debug {
            create_debug_messenger(&ip, instance)?
        } else {
            vk::NULL_HANDLE
        };

        let surface = create_surface(init.window, instance)?;

        let req_dev_exts = vec!["VK_KHR_swapchain".to_owned()];

        let physical_device = find_physical_device(&ip, instance, &req_dev_exts)?;
        let queue_family_indices = find_queue_families(&ip, physical_device, surface)?;

        let device = create_device(&ip, physical_device, &queue_family_indices, &req_dev_exts)?;
        let queues = get_device_queue_families(&dp, device, &queue_family_indices);

        let command_pool = create_command_pool(&dp, device, &queue_family_indices)?;

        let mut inflight_frames = Vec::<InFlightFrame>::new();
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let frame = InFlightFrame::new(&dp, device)?;
            inflight_frames.push(frame);
        }

        Ok(Vulkan {
            ep,
            instance,
            ip,
            debugger,
            dp,
            physical_device,
            device,
            queue_family_indices,
            queue_families: queues,
            surface,
            command_pool,
            inflight_frames,
            current_frame: 0,
            swapchain: None,
        })
    }

    pub fn change_framebuffer(&mut self) -> Result<()> {
        if self.swapchain.is_some() {
            self.destroy_swapchain()?;
        }

        Ok(())
    }

    fn create_swapchain(&mut self, window: &glfw::Window) -> Result<()> {
        assert!(self.swapchain.is_none());

        self.swapchain = Some(Swapchain::new(&SwapchainInit {
            ip: &self.ip,
            dp: &self.dp,
            physical_device: self.physical_device,
            device: self.device,
            queue_family_indices: &self.queue_family_indices,
            command_pool: self.command_pool,
            surface: self.surface,
            window,
        })?);

        Ok(())
    }

    fn destroy_swapchain(&mut self) -> Result<()> {
        let swapchain = self.swapchain.take().unwrap();
        swapchain.destroy(&self.dp, self.device, self.command_pool)
    }

    pub fn draw_frame(&mut self, window: &glfw::Window) -> Result<()> {
        if self.swapchain.is_none() {
            self.create_swapchain(window)?;
        }

        let acquire_result = {
            let swapchain = self.swapchain.as_mut().unwrap();

            let current_inflight_frame = self
                .inflight_frames
                .get(self.current_frame)
                .ok_or_else(|| to_other("invalid current frame"))?;

            self.dp
                .wait_for_fences(
                    self.device,
                    &[current_inflight_frame.in_flight_fence],
                    true,
                    u64::MAX,
                )
                .map_err(to_vulkan)?;
            self.dp
                .acquire_next_image_khr(
                    self.device,
                    swapchain.swapchain,
                    u64::MAX,
                    current_inflight_frame.available_semaphore,
                    vk::NULL_HANDLE,
                )
                .map_err(to_vulkan)
                .map(|next_image| (next_image, current_inflight_frame))
        };

        if let Err(Error::VulkanError(vk::ERROR_OUT_OF_DATE_KHR)) = acquire_result {
            self.destroy_swapchain()?;
            return Ok(());
        }

        let (image_index_index, current_inflight_frame) = acquire_result?;

        let swapchain = self.swapchain.as_mut().unwrap();

        let swapchain_images_len = swapchain.images.len();
        let swapchain_image = swapchain
            .images
            .get_mut(image_index_index as usize)
            .ok_or_else(|| {
                to_other(format!(
                    "invalid current image index {} of len {} sync objects",
                    image_index_index, swapchain_images_len
                ))
            })?;

        if swapchain_image.in_flight_fence != vk::NULL_HANDLE {
            self.dp
                .wait_for_fences(
                    self.device,
                    &[swapchain_image.in_flight_fence],
                    true,
                    u64::MAX,
                )
                .map_err(to_vulkan)?;
        }

        swapchain_image.in_flight_fence = current_inflight_frame.in_flight_fence;

        let command_buffers = [swapchain_image.command_buffer];

        let wait_dst_stage_mask = [vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT];

        let wait_semaphores = [current_inflight_frame.available_semaphore];
        let signal_semaphores = [current_inflight_frame.rendered_semaphore];

        let submit_info = vk::SubmitInfo {
            sType: vk::STRUCTURE_TYPE_SUBMIT_INFO,
            pNext: std::ptr::null(),
            waitSemaphoreCount: wait_semaphores.len() as u32,
            pWaitSemaphores: wait_semaphores.as_ptr(),
            pWaitDstStageMask: wait_dst_stage_mask.as_ptr(),
            commandBufferCount: command_buffers.len() as u32,
            pCommandBuffers: command_buffers.as_ptr(),
            signalSemaphoreCount: signal_semaphores.len() as u32,
            pSignalSemaphores: signal_semaphores.as_ptr(),
        };

        self.dp
            .reset_fences(self.device, &[current_inflight_frame.in_flight_fence])
            .map_err(to_vulkan)?;

        unsafe {
            self.dp.queue_submit(
                self.queue_families.graphics_queue,
                &[submit_info],
                current_inflight_frame.in_flight_fence,
            )
        }
        .map_err(to_vulkan)?;

        let swapchains = [swapchain.swapchain];

        let present_info = vk::PresentInfoKHR {
            sType: vk::STRUCTURE_TYPE_PRESENT_INFO_KHR,
            pNext: std::ptr::null(),
            waitSemaphoreCount: signal_semaphores.len() as u32,
            pWaitSemaphores: signal_semaphores.as_ptr(),
            swapchainCount: swapchains.len() as u32,
            pSwapchains: swapchains.as_ptr(),
            pImageIndices: &image_index_index,
            pResults: std::ptr::null_mut(),
        };

        let present_result = unsafe {
            self.dp
                .queue_present_khr(self.queue_families.present_queue, &present_info)
                .map_err(to_vulkan)
        };
        match present_result {
            Ok(_) => {
                // go on
            }
            Err(Error::VulkanError(vk::ERROR_OUT_OF_DATE_KHR)) => {
                self.destroy_swapchain()?;
                return Ok(());
            }
            Err(err) => {
                return Err(err);
            }
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    pub fn wait_idle(&mut self) -> Result<()> {
        self.dp
            .queue_wait_idle(self.queue_families.present_queue)
            .map_err(to_vulkan)
    }

    pub fn destroy(mut self) -> Result<()> {
        for inflight_frame in self.inflight_frames.drain(..) {
            inflight_frame.destroy(&self.dp, self.device);
        }

        self.swapchain
            .take()
            .map(|sc| sc.destroy(&self.dp, self.device, self.command_pool));

        self.dp.destroy_command_pool(self.device, self.command_pool);
        self.command_pool = vk::NULL_HANDLE;

        self.dp.destroy_device(self.device);
        self.device = 0;

        self.ip.destroy_surface_khr(self.instance, self.surface);
        self.surface = vk::NULL_HANDLE;

        if self.debugger != vk::NULL_HANDLE {
            self.ip
                .destroy_debug_utils_messenger_ext(self.instance, self.debugger)
                .map_err(to_vulkan)?;
            self.debugger = vk::NULL_HANDLE;
        }

        self.ip.destroy_instance(self.instance);
        self.instance = 0;

        Ok(())
    }
}

// Sc

struct SwapchainInit<'a> {
    ip: &'a InstancePointers,
    dp: &'a DevicePointers,
    physical_device: vk::PhysicalDevice,
    device: vk::Device,
    surface: vk::SurfaceKHR,
    window: &'a glfw::Window,
    queue_family_indices: &'a QueueFamilyIndices,
    command_pool: vk::CommandPool,
}

struct Swapchain {
    images: Vec<SwapchainImage>,
    swapchain: vk::SwapchainKHR,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    vertex_shader_module: vk::ShaderModule,
    fragment_shader_module: vk::ShaderModule,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
}

impl Swapchain {
    fn new(init: &SwapchainInit) -> Result<Self> {
        let (swapchain, format, _, extent) = create_swapchain(
            init.ip,
            init.dp,
            init.physical_device,
            init.device,
            init.surface,
            init.window,
            init.queue_family_indices,
        )?;

        let render_pass = create_render_pass(init.dp, init.device, &format)?;

        let (vertex_shader_module, fragment_shader_module, pipeline_layout, pipeline) =
            create_graphics_pipeline(init.dp, init.device, &extent, render_pass)?;

        let (vertex_buffer, vertex_buffer_memory) =
            create_vertex_buffer(init.ip, init.dp, init.physical_device, init.device)?;

        let images = init
            .dp
            .get_swapchain_images_khr(init.device, swapchain)
            .map_err(to_vulkan)?;

        let mut swapchain_images = Vec::<SwapchainImage>::with_capacity(images.len());
        for image in &images {
            let swapchain_image = SwapchainImage::new(&ScImageInit {
                dp: init.dp,
                command_pool: init.command_pool,
                device: init.device,
                extent: &extent,
                image: *image,
                pipeline,
                render_pass,
                surface_format: &format,
                vertex_buffer,
            })?;

            swapchain_images.push(swapchain_image);
        }

        Ok(Self {
            images: swapchain_images,
            pipeline,
            pipeline_layout,
            render_pass,
            swapchain,
            vertex_shader_module,
            fragment_shader_module,
            vertex_buffer,
            vertex_buffer_memory,
        })
    }

    fn destroy(
        self,
        dp: &DevicePointers,
        device: vk::Device,
        command_pool: vk::CommandPool,
    ) -> Result<()> {
        dp.device_wait_idle(device).map_err(to_vulkan)?;

        dp.free_memory(device, self.vertex_buffer_memory);
        dp.destroy_buffer(device, self.vertex_buffer);

        for image in &self.images {
            dp.destroy_framebuffer(device, image.framebuffer);
            dp.destroy_image_view(device, image.image_view);
            dp.free_command_buffers(device, command_pool, &[image.command_buffer]);
        }

        dp.destroy_pipeline(device, self.pipeline);
        dp.destroy_pipeline_layout(device, self.pipeline_layout);
        dp.destroy_render_pass(device, self.render_pass);
        dp.destroy_shader_module(device, self.vertex_shader_module);
        dp.destroy_shader_module(device, self.fragment_shader_module);
        dp.destroy_swapchain_khr(device, self.swapchain);

        Ok(())
    }
}

struct SwapchainImage {
    image: vk::Image,
    image_view: vk::ImageView,
    framebuffer: vk::Framebuffer,
    command_buffer: vk::CommandBuffer,
    in_flight_fence: vk::Fence,
}

struct ScImageInit<'a> {
    dp: &'a DevicePointers,
    device: vk::Device,
    render_pass: vk::RenderPass,
    image: vk::Image,
    extent: &'a vk::Extent2D,
    command_pool: vk::CommandPool,
    pipeline: vk::Pipeline,
    surface_format: &'a vk::SurfaceFormatKHR,
    vertex_buffer: vk::Buffer,
}

impl SwapchainImage {
    fn new(init: &ScImageInit) -> Result<Self> {
        let image_view =
            create_image_view(init.dp, init.device, init.image, init.surface_format.format)?;
        let framebuffer = create_framebuffer(
            init.dp,
            init.device,
            init.render_pass,
            image_view,
            init.extent,
        )?;
        let command_buffer = create_command_buffer(
            init.dp,
            init.device,
            init.pipeline,
            init.render_pass,
            init.command_pool,
            framebuffer,
            init.extent,
            init.vertex_buffer,
        )?;

        Ok(Self {
            framebuffer,
            image: init.image,
            image_view,
            command_buffer,
            in_flight_fence: vk::NULL_HANDLE,
        })
    }
}

struct InFlightFrame {
    available_semaphore: vk::Semaphore,
    rendered_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
}

impl InFlightFrame {
    fn new(dp: &DevicePointers, device: vk::Device) -> Result<Self> {
        Ok(Self {
            available_semaphore: create_semaphore(dp, device)?,
            rendered_semaphore: create_semaphore(dp, device)?,
            in_flight_fence: create_signaled_fence(dp, device)?,
        })
    }

    fn destroy(self, dp: &DevicePointers, device: vk::Device) {
        dp.destroy_semaphore(device, self.available_semaphore);
        dp.destroy_semaphore(device, self.rendered_semaphore);
        dp.destroy_fence(device, self.in_flight_fence);
    }
}

fn create_primary_command_buffer(
    dp: &DevicePointers,
    device: vk::Device,
    command_pool: vk::CommandPool,
) -> Result<vk::CommandBuffer> {
    let command_buffers = unsafe {
        dp.allocate_command_buffers(
            device,
            &vk::CommandBufferAllocateInfo {
                sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
                pNext: std::ptr::null(),
                commandPool: command_pool,
                level: vk::COMMAND_BUFFER_LEVEL_PRIMARY,
                commandBufferCount: 1,
            },
        )
        .map_err(to_vulkan)
    }?;

    Ok(command_buffers.iter().cloned().next().unwrap())
}

fn create_semaphore(dp: &DevicePointers, device: vk::Device) -> Result<vk::Semaphore> {
    unsafe {
        dp.create_semaphore(
            device,
            &vk::SemaphoreCreateInfo {
                sType: vk::STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
            },
        )
    }
    .map_err(to_vulkan)
}

fn create_signaled_fence(dp: &DevicePointers, device: vk::Device) -> Result<vk::Fence> {
    unsafe {
        dp.create_fence(
            device,
            &vk::FenceCreateInfo {
                sType: vk::STRUCTURE_TYPE_FENCE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: vk::FENCE_CREATE_SIGNALED_BIT,
            },
        )
    }
    .map_err(to_vulkan)
}
