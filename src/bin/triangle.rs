use ash::util::*;
use ash::vk;
use std::default::Default;
use std::ffi::CString;
use std::io::Cursor;
use std::mem;
use std::mem::align_of;
use std::path::PathBuf;
use rt_vk_example::*;

#[derive(Clone, Debug, Copy)]
struct Vertex {
    pos: [f32; 4],
    color: [f32; 4],
}

fn main() 
{
    let base = InstanceBase::new(InstanceCreateInfo {
        window_width: 1920,
        window_height: 1090,
        app_name: String::from("triangle"),
    });

    // attachment
    let render_attachment = 
    [
        vk::AttachmentDescription {
            format: base.surface_format.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        },
        vk::AttachmentDescription {
            format: vk::Format::D16_UNORM,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            initial_layout: vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ..Default::default()
        },
    ];

    // render pass
    let render_pass;
    {
        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        },];

        let depth_attachment_refs = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
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
            .attachments(&render_attachment)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        
        unsafe {
            render_pass = base
                .device
                .create_render_pass(
                    &render_pass_create_info,None)
                .unwrap();
        }
    }

    // frame buffer
    let frame_buffers;
    {
        frame_buffers = base
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachment = [present_image_view];
                let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&framebuffer_attachment)
                    .width(base.surface_resolution.width)
                    .height(base.surface_resolution.height)
                    .layers(1);
                
                unsafe {
                    return base.device
                        .create_framebuffer(
                            &framebuffer_create_info, None)
                        .unwrap();
                }

            })
            .collect::<Vec<vk::Framebuffer>>();
    }

    let index_buffer_data = [0u32, 1, 2];
    let index_buffer;
    {
        let ib_ci = vk::BufferCreateInfo::builder()
            .size(std::mem::size_of_val(&index_buffer_data) as u64)
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        unsafe {
            index_buffer = base.device
                .create_buffer(&ib_ci, None)
                .unwrap();
        }
    }
    let index_buffer_memory;
    {
        let ib_memory_req;
        unsafe {
            ib_memory_req = base.device.get_buffer_memory_requirements(index_buffer);
        }
        let ib_memory_index = find_memorytype_index(
            &ib_memory_req, 
            &base.device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
            .unwrap();
        let ib_memory_allocate_ci = vk::MemoryAllocateInfo {
            allocation_size: ib_memory_req.size,
            memory_type_index: ib_memory_index,
            ..Default::default()
        };
        
        unsafe {
            index_buffer_memory = base
                .device
                .allocate_memory(&ib_memory_allocate_ci, None)
                .unwrap();
        } 
    }
    // write data to index buffer
    {
        let ib_memory_req;
        unsafe {
            ib_memory_req = base.device.get_buffer_memory_requirements(index_buffer);
        }
        let mut index_slice: Align<u32>;
        unsafe {
            let index_ptr;
            index_ptr = base.device
                .map_memory(index_buffer_memory, 0, 
                    ib_memory_req.size,
                    vk::MemoryMapFlags::empty())
                .unwrap();

            index_slice = Align::new(
                index_ptr,
                align_of::<u32>() as u64,
                ib_memory_req.size
            );
        }

        index_slice.copy_from_slice(&index_buffer_data)
    }
    // bind index buffer memory to index buffer
    unsafe {
        base.device.unmap_memory(index_buffer_memory);
        base.device.bind_buffer_memory(index_buffer, index_buffer_memory, 0)
            .unwrap();
    }

    let vertices;
    {
        vertices = [
            Vertex {
                pos: [-1.0, 1.0, 0.0, 1.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                pos: [1.0, 1.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            Vertex {
                pos: [0.0, -1.0, 0.0, 1.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
        ];
    }
    let vertex_input_buffer;
    let vertex_input_buffer_memory;
    unsafe {
        let vb_ci = vk::BufferCreateInfo {
            size: (vertices.len() * std::mem::size_of::<Vertex>()) as u64,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let vb = base.device
            .create_buffer(&vb_ci, None)
            .unwrap();
        
        let vb_m_req = base.device.get_buffer_memory_requirements(vb);
        let vb_mi = find_memorytype_index(
            &vb_m_req, 
            &base.device_memory_properties, 
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
            .unwrap();
        let vb_mai = vk::MemoryAllocateInfo {
            allocation_size: vb_m_req.size,
            memory_type_index: vb_mi,
            ..Default::default()
        };
        let vb_m = base.device
            .allocate_memory(&vb_mai, None)
            .unwrap();

        vertex_input_buffer = vb;
        vertex_input_buffer_memory = vb_m;
    }
    // write data to vertex input buffer
    {
        let mut vb_slice;
        unsafe {
            let vb_m_req = base.device.get_buffer_memory_requirements(vertex_input_buffer);
            let vb_ptr = base.device
                .map_memory(
                    vertex_input_buffer_memory, 
                    0, 
                    vb_m_req.size, 
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            
            vb_slice = Align::new(
                vb_ptr, 
                align_of::<Vertex>() as u64, 
                vb_m_req.size
            ) as Align<Vertex>;
        }
        vb_slice.copy_from_slice(&vertices);
    }
    // bind vertex input mrmory to buffer
    unsafe {
        base.device.unmap_memory(vertex_input_buffer_memory);
        base.device.bind_buffer_memory(
            vertex_input_buffer, 
            vertex_input_buffer_memory, 
            0)
            .unwrap();
    }
    //
    let shader_root_path = String::from("shader/");
    // glsl shader module load func
    let f_shader_mod = |glsl_path: &str| {
        let path = shader_root_path.clone() + glsl_path;
        let bytes = std::fs::read(&path).expect(&format!("open file {:?} failed", PathBuf::from(&path)));
        let mut spv_file = Cursor::new(bytes);
        let code = read_spv(&mut spv_file)
            .expect("Failed to read shader spv file");
        let ci = vk::ShaderModuleCreateInfo::builder()
            .code(&code);
        unsafe {
            base.device
                .create_shader_module(&ci, None)
                .expect("Vertex shader module error")
        }
    };
    // vert shader
    let vert_smod= f_shader_mod("triangle/triangle.vert.spv");
    // frag shader
    let frag_smod = f_shader_mod("triangle/triangle.frag.spv");
    // shader stage create info
    let shader_stage_ci;
    {
        let shader_entry_name = CString::new("main").unwrap();
        shader_stage_ci = [
            vk::PipelineShaderStageCreateInfo {
                module: vert_smod,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: frag_smod,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            }
        ]
    }
    // vert input binding desc
    let vert_input_binding_desc = [
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    ];
    // vert input attr desc
    let vert_input_attr_desc = [
        vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: offset_of!(Vertex, pos) as u32,
            ..Default::default()
        },
        vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(Vertex, color) as u32,
            ..Default::default()
        }
    ];
    let viewports = [vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: base.surface_resolution.width as f32,
        height: base.surface_resolution.height as f32,
        min_depth: 0.0,
        max_depth: 1.0
    }];
    let scissors = [vk::Rect2D {
        offset: vk::Offset2D {x:0, y:0},
        extent: base.surface_resolution,
    }];
    let pipeline_layout;
    // create graphic pipeline
    let graphic_pipelines;
    {
        let vert_input_state_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_attribute_description_count: vert_input_attr_desc.len() as u32,
            p_vertex_attribute_descriptions: vert_input_attr_desc.as_ptr(),
            vertex_binding_description_count: vert_input_binding_desc.len() as u32,
            p_vertex_binding_descriptions: vert_input_binding_desc.as_ptr(),
            ..Default::default()
        };

        let vert_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };


        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);
        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };
        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ZERO,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::all(),
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op(vk::LogicOp::CLEAR)
        .attachments(&color_blend_attachment_states);
        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);
        let layout_create_info = vk::PipelineLayoutCreateInfo::default();
        unsafe {
            pipeline_layout = base
                .device
                .create_pipeline_layout(&layout_create_info, None)
                .unwrap();
        } 
        let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_ci)
            .vertex_input_state(&vert_input_state_info)
            .input_assembly_state(&vert_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(render_pass);
        
        unsafe {
            graphic_pipelines = base
                .device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[graphic_pipeline_info.build()],
                    None,
                )
                .expect("Unable to create graphics pipeline");
        }
    }
    let graphic_pipeline = graphic_pipelines[0];

    // render loop
    base.render_loop(|| {
        let present_index;
        unsafe {
            let (_present_index, _) = base
                .swapchain_loader
                .acquire_next_image(
                    base.swapchain,
                    std::u64::MAX, 
                    base.present_complete_semaphore, 
                    vk::Fence::null())
                .unwrap();
            present_index = _present_index;
        }

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
               depth_stencil: vk::ClearDepthStencilValue {
                   depth: 1.0,
                   stencil: 0
               } 
            }
        ];
           
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .framebuffer(frame_buffers[present_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D{ x: 0, y: 0},
                extent: base.surface_resolution,
            })
            .clear_values(&clear_values);

        record_submit_commandbuffer(
            &base.device, 
            base.draw_command_buffer, 
            base.present_queue, 
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT], 
            &[base.present_complete_semaphore], 
            &[base.rendering_complete_semaphore], 
            |device, draw_command_buffer| unsafe {
                device.cmd_begin_render_pass(
                    draw_command_buffer, 
                    &render_pass_begin_info, 
                    vk::SubpassContents::INLINE
                );
                device.cmd_bind_pipeline(
                    draw_command_buffer, 
                    vk::PipelineBindPoint::GRAPHICS, 
                    graphic_pipeline
                );
                device.cmd_set_viewport(
                    draw_command_buffer, 
                    0,
                    &viewports,
                );
                device.cmd_set_scissor(
                    draw_command_buffer, 
                    0,
                    &scissors,
                );
                device.cmd_bind_vertex_buffers(
                    draw_command_buffer, 
                    0,
                    &[vertex_input_buffer],
                    &[0]
                );
                device.cmd_bind_index_buffer(
                    draw_command_buffer, 
                    index_buffer, 
                    0,
                    vk::IndexType::UINT32
                );
                device.cmd_draw_indexed(
                    draw_command_buffer, 
                    index_buffer_data.len() as u32, 
                    1,
                    0,
                    0,
                    1
                );
                device.cmd_end_render_pass(draw_command_buffer);
            },
        );

        let wait_semaphore = [base.rendering_complete_semaphore];
        let swapchains = [base.swapchain];
        let image_indices = [present_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphore)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        
        unsafe {
            base.swapchain_loader
                .queue_present(base.present_queue, &present_info)
                .unwrap();
        }

    });

    unsafe {
        base.device.device_wait_idle().unwrap();
        for pipeline in graphic_pipelines {
            base.device.destroy_pipeline(pipeline, None);
        }
        base.device.destroy_pipeline_layout(pipeline_layout, None);
        base.device
            .destroy_shader_module(vert_smod, None);
        base.device
            .destroy_shader_module(frag_smod, None);
        base.device.free_memory(index_buffer_memory, None);
        base.device.destroy_buffer(index_buffer, None);
        base.device.free_memory(vertex_input_buffer_memory, None);
        base.device.destroy_buffer(vertex_input_buffer, None);
        for framebuffer in frame_buffers {
            base.device.destroy_framebuffer(framebuffer, None);
        }
        base.device.destroy_render_pass(render_pass, None);
    }

}
