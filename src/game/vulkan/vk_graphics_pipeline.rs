use super::error::{to_other, to_vulkan, Result};
use super::util::copy_extent_2d;
use super::vertex::Vertex;
use inline_spirv::include_spirv;
use std::{ffi::CString, mem::size_of};
use vk_sys as vk;
use vulkanic::DevicePointers;

pub fn create_graphics_pipeline(
    dp: &DevicePointers,
    device: vk::Device,
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

    let vertex_shader_module = create_shader_module(dp, device, vert_shader)?;
    let fragment_shader_module = create_shader_module(dp, device, frag_shader)?;

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

    let pipeline_layout =
        unsafe { dp.create_pipeline_layout(device, &pipeline_layout_info) }.map_err(to_vulkan)?;

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

    let pipelines =
        unsafe { dp.create_graphics_pipelines(device, vk::NULL_HANDLE, &[pipeline_info]) }
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
