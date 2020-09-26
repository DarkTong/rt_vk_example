use ash::vk;
use std::default::Default;
use std::ffi::CString;
use std::mem;
use rt_vk_example::offset_of;
use rt_vk_example::base::world::*;
use rt_vk_example::base::pso;
use rt_vk_example::base::pso::ShaderProgramDescriptor;

#[derive(Clone, Debug, Copy)]
struct Vertex {
    pos: [f32; 4],
    color: [f32; 4],
}

fn main() 
{
    println!("current dir: {:?}", std::env::current_dir());
    let mut base = InstanceBase::new(InstanceCreateInfo {
        window_width: 1920,
        window_height: 1090,
        app_name: String::from("triangle"),
    });

    // attachment
    let render_attachment = vec![
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
            initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ..Default::default()
        },
    ];
    // vert input binding desc
    let vert_input_binding_desc = vec![
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    ];
    // vert input attr desc
    let vert_input_attr_desc = vec![
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
    ] ;
    let pso_desc = pso::PipelineStateObjectDescriptor {
        vs_desc: ShaderProgramDescriptor {
            path: "./shader/triangle/triangle.vert".to_string(),
            entry: CString::new("main").unwrap(),
        },
        ps_desc: ShaderProgramDescriptor {
            path: "./shader/triangle/triangle.frag".to_string(),
            entry: CString::new("main").unwrap(),
        },
        attachment_desc: render_attachment, // move
        viewports: vec![vk::Viewport {
            x: 0.0, y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }],
        scissors: vec![vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: base.surface_resolution.width,
                height: base.surface_resolution.height
            }
        }],
        input_binding_desc: vert_input_binding_desc,
        input_attr_desc: vert_input_attr_desc,
    };
    let pso = base.create_pipeline_state_object(&pso_desc)
        .expect("create pso failed");

    // frame buffer
    let frame_buffers;
    {
        frame_buffers = base
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachment = [present_image_view, base.depth_image_view];
                let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(pso.render_pass)
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

    // vertex buffer
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
    let vb_size = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;
    let mut vb = base.allocate_vertex_buffer::<Vertex>(vb_size);
    vb.slice.copy_from_slice(&vertices);
    // index buffer
    let ib_data = [0u32, 1, 2];
    let ib_size = (ib_data.len() * std::mem::size_of::<u32>()) as u64;
    let mut ib = base.allocate_index_buffer(ib_size);
    ib.slice.copy_from_slice(&ib_data);

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
            .render_pass(pso.render_pass)
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
                    pso.pipeline
                );
                device.cmd_set_viewport(
                    draw_command_buffer, 
                    0,
                    &pso.pso_desc.viewports,
                );
                device.cmd_set_scissor(
                    draw_command_buffer, 
                    0,
                    &pso.pso_desc.scissors,
                );
                device.cmd_bind_vertex_buffers(
                    draw_command_buffer, 
                    0,
                    &[base.vertex_buffer.buffer],
                    &[vb.offset]
                );
                device.cmd_bind_index_buffer(
                    draw_command_buffer,
                    base.index_buffer.buffer,
                    ib.offset,
                    vk::IndexType::UINT32
                );
                device.cmd_draw_indexed(
                    draw_command_buffer,
                    ib_size as u32,
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
        drop(pso);
        for framebuffer in frame_buffers {
            base.device.destroy_framebuffer(framebuffer, None);
        }
    }

}
