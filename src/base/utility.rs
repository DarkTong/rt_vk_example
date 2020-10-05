use ash::vk;
use ash::version::*;
use super::ri;
use super::pso;
use super::loader;
use std::rc;
use std::boxed;
use std::io;

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    // Try to find an exactly matching memory flag
    let best_suitable_index =
        find_memorytype_index_f(memory_req, memory_prop, flags, |property_flags, flags| {
            property_flags == flags
        });
    if best_suitable_index.is_some() {
        return best_suitable_index;
    }
    // Otherwise find a memory flag that works
    find_memorytype_index_f(memory_req, memory_prop, flags, |property_flags, flags| {
        property_flags & flags == flags
    })
}

pub fn find_memorytype_index_f<F: Fn(vk::MemoryPropertyFlags, vk::MemoryPropertyFlags) -> bool>(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
    f: F,
) -> Option<u32> {
    let mut memory_type_bits = memory_req.memory_type_bits;
    for (index, ref memory_type) in memory_prop.memory_types.iter().enumerate() {
        if memory_type_bits & 1 == 1 && f(memory_type.property_flags, flags) {
            return Some(index as u32);
        }
        memory_type_bits >>= 1;
    }
    None
}

#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = mem::zeroed();
            (&b.$field as *const _ as isize) - (&b as *const _ as isize)
        }
    }};
}


pub fn create_pipeline_state_object(backend: &rc::Rc<ri::Backend>, desc: &pso::PipelineStateObjectDescriptor)
    -> io::Result<boxed::Box<pso::PipelineStateObject>>
{
    let vs_mod = loader::load_shader(&backend.device, &desc.vs_desc.path)
        .expect("vs shader create failed");
    let ps_mod = loader::load_shader(&backend.device, &desc.ps_desc.path)
        .expect("ps shader create failed");

    let render_pass;{
    let color_attachment_refs = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    },];

    let depth_attachment_refs = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let subpass1 = vk::SubpassDescription::builder()
        .color_attachments(&color_attachment_refs)
        .depth_stencil_attachment(&depth_attachment_refs)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .build();

    let subpasses = [subpass1,];

    let dependencies = [vk::SubpassDependency{
        src_subpass: vk::SUBPASS_EXTERNAL,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        ..Default::default()
    },];

    let render_pass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&desc.attachment_desc)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    unsafe {
        render_pass = backend.device
            .create_render_pass(
                &render_pass_create_info,None)
            .unwrap();
    }
}

    let stage_ci = vec![
        vk::PipelineShaderStageCreateInfo {
            module: vs_mod,
            p_name: desc.vs_desc.entry.as_ptr(),
            stage: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            module: ps_mod,
            p_name: desc.ps_desc.entry.as_ptr(),
            stage: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        }
    ];
    let vert_input_state_ci= vk::PipelineVertexInputStateCreateInfo {
        vertex_attribute_description_count: desc.input_attr_desc.len() as u32,
        p_vertex_attribute_descriptions: desc.input_attr_desc.as_ptr(),
        vertex_binding_description_count: desc.input_binding_desc.len() as u32,
        p_vertex_binding_descriptions: desc.input_binding_desc.as_ptr(),
        ..Default::default()
    };
    let input_assembly_state_ci = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        ..Default::default()
    };

    let viewport_state_ci = vk::PipelineViewportStateCreateInfo::builder()
        .scissors(&desc.scissors)
        .viewports(&desc.viewports);

    let rasterization_state_ci = vk::PipelineRasterizationStateCreateInfo {
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        line_width: 1.0,
        polygon_mode: vk::PolygonMode::FILL,
        ..Default::default()
    };

    let multi_sample_state_ci = vk::PipelineMultisampleStateCreateInfo {
        rasterization_samples: vk::SampleCountFlags::TYPE_1,
        ..Default::default()
    };

    let stencil_op_state = vk::StencilOpState {
        fail_op: vk::StencilOp::KEEP,
        pass_op: vk::StencilOp::KEEP,
        depth_fail_op: vk::StencilOp::KEEP,
        compare_op: vk::CompareOp::ALWAYS,
        ..Default::default()
    };

    let depth_stencil_state_ci = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: 1,
        depth_write_enable: 1,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        front: stencil_op_state,
        back: stencil_op_state,
        max_depth_bounds: 1.0,
        ..Default::default()
    };
    let attachment_blend_states_ci = vec![vk::PipelineColorBlendAttachmentState {
        blend_enable: 0,
        src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ZERO,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::all(),
    }];

    let dynamic_state = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let pipeline_layout;
    unsafe {
        let layout_create_info = vk::PipelineLayoutCreateInfo::default();
        pipeline_layout = backend.device
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap();
    }

    let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stage_ci)
        .vertex_input_state(&vert_input_state_ci)
        .input_assembly_state(&input_assembly_state_ci)
        .viewport_state(&viewport_state_ci)
        .rasterization_state(&rasterization_state_ci)
        .multisample_state(&multi_sample_state_ci)
        .depth_stencil_state(&depth_stencil_state_ci)
        .color_blend_state(&vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&attachment_blend_states_ci)
        )
        .dynamic_state(&vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_state)
        )
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .build();

    let pipeline;
    unsafe {
        pipeline = backend.device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_ci],
                None,
            ).expect("unable to create graphic pipeline");
    }

    Ok(Box::new(pso::PipelineStateObject{
        pso_desc: desc.clone(),
        vs_mod,
        ps_mod,
        render_pass,
        pipeline_layout,
        pipeline: pipeline[0],
        device: backend.device.clone(),
    }))
}
