use std::{ffi::CString, mem::size_of, ptr};

use crate::game::vulkan::vertex::Vertex;

use super::util::{copy_extent_2d, copy_surface_format_khr};
use super::Result;
use super::{
    error::{to_other, to_vulkan, Error},
    Context, InFlightFrame, Swapchain, SwapchainContext, SwapchainImage, Vulkan,
    MAX_FRAMES_IN_FLIGHT,
};
use glfw::Window;
use glm::{Vec2, Vec3};
use inline_spirv::include_spirv;
use vk_sys as vk;
use vulkanic::DevicePointers;

impl Vulkan {
    pub fn draw_frame(&mut self, window: &glfw::Window) -> Result<()> {
        if self.sc_ctx.is_none() {
            self.create_swapchain(window)?;
        }

        let acquire_result = {
            let swapchain = self.sc_ctx.as_mut().unwrap();

            let current_inflight_frame = self
                .inflight_frames
                .get(self.current_frame)
                .ok_or_else(|| to_other("invalid current frame"))?;

            self.ctx
                .dp
                .wait_for_fences(
                    self.ctx.device,
                    &[current_inflight_frame.in_flight_fence],
                    true,
                    u64::MAX,
                )
                .map_err(to_vulkan)?;
            self.ctx
                .dp
                .acquire_next_image_khr(
                    self.ctx.device,
                    swapchain.ctx.swapchain,
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

        let swapchain = self.sc_ctx.as_mut().unwrap();

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
            self.ctx
                .dp
                .wait_for_fences(
                    self.ctx.device,
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

        self.ctx
            .dp
            .reset_fences(self.ctx.device, &[current_inflight_frame.in_flight_fence])
            .map_err(to_vulkan)?;

        unsafe {
            self.ctx.dp.queue_submit(
                self.ctx.queue_families.graphics_queue,
                &[submit_info],
                current_inflight_frame.in_flight_fence,
            )
        }
        .map_err(to_vulkan)?;

        let swapchains = [swapchain.ctx.swapchain];

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
            self.ctx
                .dp
                .queue_present_khr(self.ctx.queue_families.present_queue, &present_info)
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

    pub fn on_framebuffer_changed(&mut self) -> Result<()> {
        if self.sc_ctx.is_some() {
            self.destroy_swapchain()?;
        }

        Ok(())
    }

    fn create_swapchain(&mut self, window: &glfw::Window) -> Result<()> {
        assert!(self.sc_ctx.is_none());

        self.sc_ctx = Some(Swapchain::new(&self.ctx, window)?);

        Ok(())
    }

    fn destroy_swapchain(&mut self) -> Result<()> {
        let swapchain = self.sc_ctx.take().unwrap();
        swapchain.destroy(&self.ctx)
    }
}

impl Swapchain {
    fn new(ctx: &Context, window: &glfw::Window) -> Result<Self> {
        let (swapchain, surface_format, _, extent) = create_swapchain(ctx, window)?;
        let render_pass = create_render_pass(ctx, &surface_format)?;

        let (vertex_shader_module, fragment_shader_module, pipeline_layout, pipeline) =
            create_graphics_pipeline(ctx, &extent, render_pass)?;

        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(ctx)?;

        let sc_ctx = SwapchainContext {
            pipeline,
            pipeline_layout,
            render_pass,
            swapchain,
            vertex_shader_module,
            fragment_shader_module,
            vertex_buffer,
            vertex_buffer_memory,
            extent,
            surface_format,
        };

        let images = ctx
            .dp
            .get_swapchain_images_khr(ctx.device, swapchain)
            .map_err(to_vulkan)?;

        let mut swapchain_images = Vec::<SwapchainImage>::with_capacity(images.len());
        for image in &images {
            let swapchain_image = SwapchainImage::new(ctx, &sc_ctx, *image)?;
            swapchain_images.push(swapchain_image);
        }

        Ok(Self {
            images: swapchain_images,
            ctx: sc_ctx,
        })
    }

    pub fn destroy(self, ctx: &Context) -> Result<()> {
        ctx.dp.device_wait_idle(ctx.device).map_err(to_vulkan)?;

        ctx.dp
            .free_memory(ctx.device, self.ctx.vertex_buffer_memory);
        ctx.dp.destroy_buffer(ctx.device, self.ctx.vertex_buffer);

        for image in &self.images {
            ctx.dp.destroy_framebuffer(ctx.device, image.framebuffer);
            ctx.dp.destroy_image_view(ctx.device, image.image_view);
            ctx.dp
                .free_command_buffers(ctx.device, ctx.command_pool, &[image.command_buffer]);
        }

        ctx.dp.destroy_pipeline(ctx.device, self.ctx.pipeline);
        ctx.dp
            .destroy_pipeline_layout(ctx.device, self.ctx.pipeline_layout);
        ctx.dp.destroy_render_pass(ctx.device, self.ctx.render_pass);
        ctx.dp
            .destroy_shader_module(ctx.device, self.ctx.vertex_shader_module);
        ctx.dp
            .destroy_shader_module(ctx.device, self.ctx.fragment_shader_module);
        ctx.dp.destroy_swapchain_khr(ctx.device, self.ctx.swapchain);

        Ok(())
    }
}

impl SwapchainImage {
    fn new(ctx: &Context, sc_ctx: &SwapchainContext, image: vk::Image) -> Result<Self> {
        let image_view =
            create_image_view(&ctx.dp, ctx.device, image, sc_ctx.surface_format.format)?;
        let framebuffer = create_framebuffer(
            &ctx.dp,
            ctx.device,
            sc_ctx.render_pass,
            image_view,
            &sc_ctx.extent,
        )?;
        let command_buffer = create_command_buffer(ctx, sc_ctx, framebuffer)?;

        Ok(Self {
            framebuffer,
            image_view,
            command_buffer,
            in_flight_fence: vk::NULL_HANDLE,
        })
    }
}

impl InFlightFrame {
    pub fn new(ctx: &Context) -> Result<Self> {
        Ok(Self {
            available_semaphore: ctx.create_semaphore()?,
            rendered_semaphore: ctx.create_semaphore()?,
            in_flight_fence: ctx.create_signaled_fence()?,
        })
    }

    pub fn destroy(self, ctx: &Context) {
        ctx.destroy_semaphore(self.available_semaphore);
        ctx.destroy_semaphore(self.rendered_semaphore);
        ctx.destory_fence(self.in_flight_fence);
    }
}

fn create_render_pass(ctx: &Context, format: &vk::SurfaceFormatKHR) -> Result<vk::RenderPass> {
    let color_attachment_desc = vk::AttachmentDescription {
        flags: 0,
        format: format.format,
        samples: vk::SAMPLE_COUNT_1_BIT,
        loadOp: vk::ATTACHMENT_LOAD_OP_CLEAR,
        storeOp: vk::ATTACHMENT_STORE_OP_STORE,
        stencilLoadOp: vk::ATTACHMENT_LOAD_OP_DONT_CARE,
        stencilStoreOp: vk::ATTACHMENT_STORE_OP_DONT_CARE,
        initialLayout: vk::IMAGE_LAYOUT_UNDEFINED,
        finalLayout: vk::IMAGE_LAYOUT_PRESENT_SRC_KHR,
    };

    let color_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
    };

    let subpass_desc = vk::SubpassDescription {
        flags: 0,
        pipelineBindPoint: vk::PIPELINE_BIND_POINT_GRAPHICS,
        inputAttachmentCount: 0,
        pInputAttachments: std::ptr::null(),
        colorAttachmentCount: 1,
        pColorAttachments: &color_attachment_ref,
        pResolveAttachments: std::ptr::null(),
        pDepthStencilAttachment: std::ptr::null(),
        preserveAttachmentCount: 0,
        pPreserveAttachments: std::ptr::null(),
    };

    let subpass_dep = vk::SubpassDependency {
        srcSubpass: vk::SUBPASS_EXTERNAL,
        dstSubpass: 0,
        srcStageMask: vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        dstStageMask: vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        srcAccessMask: 0,
        dstAccessMask: vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
        dependencyFlags: 0,
    };

    let render_pass_info = vk::RenderPassCreateInfo {
        sType: vk::STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        attachmentCount: 1,
        pAttachments: &color_attachment_desc,
        subpassCount: 1,
        pSubpasses: &subpass_desc,
        dependencyCount: 1,
        pDependencies: &subpass_dep,
    };

    unsafe { ctx.dp.create_render_pass(ctx.device, &render_pass_info) }.map_err(to_vulkan)
}

fn create_swapchain(
    ctx: &Context,
    window: &Window,
) -> Result<(
    vk::SwapchainKHR,
    vk::SurfaceFormatKHR,
    vk::PresentModeKHR,
    vk::Extent2D,
)> {
    let formats = ctx
        .ip
        .get_physical_device_surface_formats_khr(ctx.physical_device, ctx.surface)
        .map_err(to_vulkan)?;
    let modes = ctx
        .ip
        .get_physical_device_surface_present_modes_khr(ctx.physical_device, ctx.surface)
        .map_err(to_vulkan)?;

    let good_format = formats
        .iter()
        .find(|format| {
            format.format == vk::FORMAT_B8G8R8A8_SRGB
                && format.colorSpace == vk::COLOR_SPACE_SRGB_NONLINEAR_KHR
        })
        .or_else(|| formats.iter().next()) // first
        .ok_or_else(|| to_other(Error::Other("no good format found".to_owned())))?;
    let good_mode = modes
        .iter()
        .find(|mode| **mode == vk::PRESENT_MODE_MAILBOX_KHR)
        .unwrap_or(&vk::PRESENT_MODE_FIFO_KHR);

    let capabilities = ctx
        .ip
        .get_physical_device_surface_capabilities_khr(ctx.physical_device, ctx.surface)
        .map_err(to_vulkan)?;
    let extent = choose_swap_extent(&capabilities, window);

    let image_count = (capabilities.minImageCount + 1).min(capabilities.maxImageCount);
    let (image_sharing_mode, queue_families) =
        if ctx.queue_family_indices.graphics != ctx.queue_family_indices.present {
            (
                vk::SHARING_MODE_CONCURRENT,
                vec![
                    ctx.queue_family_indices.graphics,
                    ctx.queue_family_indices.present,
                ],
            )
        } else {
            (vk::SHARING_MODE_EXCLUSIVE, vec![])
        };

    let info = vk::SwapchainCreateInfoKHR {
        sType: vk::STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
        pNext: std::ptr::null(),
        flags: 0,
        surface: ctx.surface,
        minImageCount: image_count,
        imageFormat: good_format.format,
        imageColorSpace: good_format.colorSpace,
        imageExtent: copy_extent_2d(&extent),
        imageArrayLayers: 1,
        imageUsage: vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
        imageSharingMode: image_sharing_mode,
        queueFamilyIndexCount: queue_families.len() as u32,
        pQueueFamilyIndices: queue_families.as_ptr(),
        preTransform: capabilities.currentTransform,
        compositeAlpha: vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
        presentMode: *good_mode,
        clipped: vk::TRUE,
        oldSwapchain: vk::NULL_HANDLE,
    };

    let swapchain = unsafe { ctx.dp.create_swapchain_khr(ctx.device, &info) }.map_err(to_vulkan)?;
    let good_format: vk::SurfaceFormatKHR = copy_surface_format_khr(good_format);

    Ok((swapchain, good_format, *good_mode, extent))
}

fn choose_swap_extent(caps: &vk::SurfaceCapabilitiesKHR, window: &glfw::Window) -> vk::Extent2D {
    if caps.currentExtent.width != u32::MAX {
        return vk::Extent2D {
            width: caps.currentExtent.width,
            height: caps.currentExtent.height,
        };
    }

    let (w, h) = window.get_framebuffer_size();
    let w = w as u32;
    let h = h as u32;

    vk::Extent2D {
        width: w.clamp(caps.minImageExtent.width, caps.maxImageExtent.width),
        height: h.clamp(caps.minImageExtent.height, caps.maxImageExtent.height),
    }
}

fn create_graphics_pipeline(
    ctx: &Context,
    extent: &vk::Extent2D,
    render_pass: vk::RenderPass,
) -> Result<(
    vk::ShaderModule,
    vk::ShaderModule,
    vk::PipelineLayout,
    vk::Pipeline,
)> {
    let vert_shader = include_spirv!("shader/vert.glsl", glsl, vert);
    let frag_shader = include_spirv!("shader/frag.glsl", glsl, frag);

    let vertex_shader_module = create_shader_module(&ctx.dp, ctx.device, vert_shader)?;
    let fragment_shader_module = create_shader_module(&ctx.dp, ctx.device, frag_shader)?;

    let name = CString::new("main").map_err(to_other)?;

    let vertex_shader_info = vk::PipelineShaderStageCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        stage: vk::SHADER_STAGE_VERTEX_BIT,
        module: vertex_shader_module,
        pName: name.as_ptr(),
        pSpecializationInfo: std::ptr::null(),
    };

    let fragment_shader_info = vk::PipelineShaderStageCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        stage: vk::SHADER_STAGE_FRAGMENT_BIT,
        module: fragment_shader_module,
        pName: name.as_ptr(),
        pSpecializationInfo: std::ptr::null(),
    };

    let shader_stages = [vertex_shader_info, fragment_shader_info];

    let binding_description = Vertex::get_binding_description();
    let attribute_descriptions = Vertex::get_attribute_descriptions();

    let vert_input_info = vk::PipelineVertexInputStateCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        vertexBindingDescriptionCount: 1,
        pVertexBindingDescriptions: &binding_description,
        vertexAttributeDescriptionCount: attribute_descriptions.len() as u32,
        pVertexAttributeDescriptions: attribute_descriptions.as_ptr(),
    };

    let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        topology: vk::PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
        primitiveRestartEnable: vk::FALSE,
    };

    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: extent.width as f32,
        height: extent.height as f32,
        minDepth: 0.0,
        maxDepth: 1.0,
    };

    let scissor = vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: copy_extent_2d(extent),
    };

    let viewport_state_info = vk::PipelineViewportStateCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        viewportCount: 1,
        pViewports: &viewport,
        scissorCount: 1,
        pScissors: &scissor,
    };

    let rasterizer_info = vk::PipelineRasterizationStateCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        depthClampEnable: vk::FALSE,
        rasterizerDiscardEnable: vk::FALSE,
        polygonMode: vk::POLYGON_MODE_FILL,
        cullMode: vk::CULL_MODE_BACK_BIT,
        frontFace: vk::FRONT_FACE_CLOCKWISE,
        depthBiasEnable: vk::FALSE,
        depthBiasConstantFactor: 0.0,
        depthBiasClamp: 0.0,
        depthBiasSlopeFactor: 0.0,
        lineWidth: 1.0,
    };

    let multisample_info = vk::PipelineMultisampleStateCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        rasterizationSamples: vk::SAMPLE_COUNT_1_BIT,
        sampleShadingEnable: vk::FALSE,
        minSampleShading: 1.0,
        pSampleMask: std::ptr::null(),
        alphaToCoverageEnable: vk::FALSE,
        alphaToOneEnable: vk::FALSE,
    };

    let color_blend_attach = vk::PipelineColorBlendAttachmentState {
        blendEnable: vk::FALSE,
        srcColorBlendFactor: vk::BLEND_FACTOR_ONE,
        dstColorBlendFactor: vk::BLEND_FACTOR_ZERO,
        colorBlendOp: vk::BLEND_OP_ADD,
        srcAlphaBlendFactor: vk::BLEND_FACTOR_ONE,
        dstAlphaBlendFactor: vk::BLEND_FACTOR_ZERO,
        alphaBlendOp: vk::BLEND_OP_ADD,
        colorWriteMask: vk::COLOR_COMPONENT_R_BIT
            | vk::COLOR_COMPONENT_G_BIT
            | vk::COLOR_COMPONENT_B_BIT
            | vk::COLOR_COMPONENT_A_BIT,
    };

    let color_blend = vk::PipelineColorBlendStateCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        logicOpEnable: vk::FALSE,
        logicOp: vk::LOGIC_OP_COPY,
        attachmentCount: 1,
        pAttachments: &color_blend_attach,
        blendConstants: [0.0, 0.0, 0.0, 0.0],
    };

    // let dynamic_states = [vk::DYNAMIC_STATE_VIEWPORT, vk::DYNAMIC_STATE_LINE_WIDTH];

    // let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
    //     sType: vk::STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO,
    //     pNext: std::ptr::null(),
    //     flags: 0,
    //     dynamicStateCount: dynamic_states.len() as u32,
    //     pDynamicStates: dynamic_states.as_ptr(),
    // };

    let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
        sType: vk::STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        setLayoutCount: 0,
        pSetLayouts: std::ptr::null(),
        pushConstantRangeCount: 0,
        pPushConstantRanges: std::ptr::null(),
    };

    let pipeline_layout = unsafe {
        ctx.dp
            .create_pipeline_layout(ctx.device, &pipeline_layout_info)
    }
    .map_err(to_vulkan)?;

    let pipeline_info = vk::GraphicsPipelineCreateInfo {
        sType: vk::STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        stageCount: shader_stages.len() as u32,
        pStages: shader_stages.as_ptr(),
        pVertexInputState: &vert_input_info,
        pInputAssemblyState: &input_assembly_info,
        pTessellationState: std::ptr::null(),
        pViewportState: &viewport_state_info,
        pRasterizationState: &rasterizer_info,
        pMultisampleState: &multisample_info,
        pDepthStencilState: std::ptr::null(),
        pColorBlendState: &color_blend,
        pDynamicState: std::ptr::null(),
        layout: pipeline_layout,
        renderPass: render_pass,
        subpass: 0,
        basePipelineHandle: vk::NULL_HANDLE,
        basePipelineIndex: -1,
    };

    let pipelines = unsafe {
        ctx.dp
            .create_graphics_pipelines(ctx.device, vk::NULL_HANDLE, &[pipeline_info])
    }
    .map_err(to_vulkan)?;
    let pipeline: vk::Pipeline = *pipelines.iter().next().unwrap();

    Ok((
        vertex_shader_module,
        fragment_shader_module,
        pipeline_layout,
        pipeline,
    ))
}

fn create_shader_module(
    dp: &DevicePointers,
    device: vk::Device,
    code: &[u32],
) -> Result<vk::ShaderModule> {
    let info = vk::ShaderModuleCreateInfo {
        sType: vk::STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        codeSize: code.len() * size_of::<u32>(), // not the len, but the size
        pCode: code.as_ptr(),
    };

    unsafe { dp.create_shader_module(device, &info) }.map_err(to_vulkan)
}

fn create_vertex_buffer(ctx: &Context) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let vertices = [
        Vertex {
            pos: Vec2::new(0.0, -0.5),
            color: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: Vec2::new(0.5, 0.5),
            color: Vec3::new(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: Vec2::new(-0.5, 0.5),
            color: Vec3::new(0.0, 0.0, 1.0),
        },
    ];

    let buffer_info = vk::BufferCreateInfo {
        sType: vk::STRUCTURE_TYPE_BUFFER_CREATE_INFO,
        pNext: ptr::null(),
        flags: 0,
        size: (size_of::<Vertex>() * vertices.len()) as u64,
        usage: vk::BUFFER_USAGE_VERTEX_BUFFER_BIT,
        sharingMode: vk::SHARING_MODE_EXCLUSIVE,
        queueFamilyIndexCount: 0,
        pQueueFamilyIndices: ptr::null(),
    };

    let buffer = unsafe { ctx.dp.create_buffer(ctx.device, &buffer_info) }.map_err(to_vulkan)?;

    let memory_requirements = ctx.dp.get_buffer_memory_requirements(ctx.device, buffer);

    let allocate_info = vk::MemoryAllocateInfo {
        sType: vk::STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
        pNext: ptr::null(),
        allocationSize: memory_requirements.size,
        memoryTypeIndex: find_memory_type(
            ctx,
            memory_requirements.memoryTypeBits,
            vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT | vk::MEMORY_PROPERTY_HOST_COHERENT_BIT,
        )?,
    };

    let device_memory =
        unsafe { ctx.dp.allocate_memory(ctx.device, &allocate_info) }.map_err(to_vulkan)?;

    ctx.dp
        .bind_buffer_memory(ctx.device, buffer, device_memory, 0)
        .map_err(to_vulkan)?;

    let data = ctx
        .dp
        .map_memory(ctx.device, device_memory, 0, buffer_info.size, 0)
        .map_err(to_vulkan)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr(),
            data as *mut Vertex,
            buffer_info.size as usize,
        )
    };
    ctx.dp.unmap_memory(ctx.device, device_memory);

    Ok((buffer, device_memory))
}

fn find_memory_type(
    ctx: &Context,
    type_filter: u32,
    flags: vk::MemoryPropertyFlags,
) -> Result<u32> {
    for i in 0..ctx.memory_properties.memoryTypeCount {
        if (type_filter & (1 << i)) != 0
            && (ctx.memory_properties.memoryTypes[i as usize].propertyFlags & flags) != 0
        {
            return Ok(i);
        }
    }

    Err(to_other("could not find memory type"))
}

fn create_command_buffer(
    ctx: &Context,
    sc_ctx: &SwapchainContext,
    framebuffer: vk::Framebuffer,
) -> Result<vk::CommandBuffer> {
    let command_buffer = ctx.allocate_primary_command_buffer()?;
    ctx.begin_command_buffer(command_buffer)?;
    ctx.begin_render_pass(sc_ctx, command_buffer, framebuffer);

    ctx.cmd_bind_pipeline(sc_ctx, command_buffer);

    ctx.dp
        .cmd_bind_vertex_buffers(command_buffer, 0, &[sc_ctx.vertex_buffer], &[0]);
    ctx.dp.cmd_draw(command_buffer, 3, 1, 0, 0);
    ctx.dp.cmd_end_render_pass(command_buffer);

    ctx.dp
        .end_command_buffer(command_buffer)
        .map_err(to_vulkan)?;

    Ok(command_buffer)
}

pub fn create_framebuffer(
    dp: &DevicePointers,
    device: vk::Device,
    render_pass: vk::RenderPass,
    image_view: vk::ImageView,
    extent: &vk::Extent2D,
) -> Result<vk::Framebuffer> {
    let attachments = [image_view];

    let create_info = vk::FramebufferCreateInfo {
        sType: vk::STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        renderPass: render_pass,
        attachmentCount: attachments.len() as u32,
        pAttachments: attachments.as_ptr(),
        width: extent.width,
        height: extent.height,
        layers: 1,
    };

    unsafe { dp.create_framebuffer(device, &create_info) }.map_err(to_vulkan)
}

pub fn create_image_view(
    dp: &DevicePointers,
    device: vk::Device,
    image: vk::Image,
    format: vk::Format,
) -> Result<vk::ImageView> {
    let info = vk::ImageViewCreateInfo {
        sType: vk::STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        image,
        viewType: vk::IMAGE_VIEW_TYPE_2D,
        format,
        components: vk::ComponentMapping {
            r: vk::COMPONENT_SWIZZLE_IDENTITY,
            g: vk::COMPONENT_SWIZZLE_IDENTITY,
            b: vk::COMPONENT_SWIZZLE_IDENTITY,
            a: vk::COMPONENT_SWIZZLE_IDENTITY,
        },
        subresourceRange: vk::ImageSubresourceRange {
            aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
            baseMipLevel: 0,
            levelCount: 1,
            baseArrayLayer: 0,
            layerCount: 1,
        },
    };

    unsafe { dp.create_image_view(device, &info) }.map_err(to_vulkan)
}
