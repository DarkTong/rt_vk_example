use ash::vk;
use ash::version::*;
use std::default::Default;
use std::ffi::CString;
use std::{mem, boxed};
use std::cell;
use rt_vk_example::app;
use rt_vk_example::offset_of;
use rt_vk_example::base::*;
use rt_vk_example::base::pso::ShaderProgramDescriptor;
use std::ops;
use rt_vk_example::app::RenderLoopAction;

#[derive(Clone, Debug, Copy)]
struct Vertex {
    pos: [f32; 4],
    color: [f32; 4],
}

struct TriangleRenderLoop {
    pub device: ash::Device,
    pub render_pass: vk::RenderPass,
    pub frame_buffers: Vec<vk::Framebuffer>,
    pub pso_obj: boxed::Box<pso::PipelineStateObject>,
    pub vb: buffer::BufferSlice<Vertex>,
    pub ib: buffer::BufferSlice<u16>,
}

impl app::RenderLoop for TriangleRenderLoop {
    fn render(&self, app_obj: &app::App)
    {
        let present_idx = app_obj.acquire_next_image() as usize;
        if present_idx >= self.frame_buffers.len() {
            return;
        }
        let clear_values = {
            [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    }
                },
            ]
        };
        let render_pass_begin_info = {
            vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .clear_values(&clear_values)
                .framebuffer(self.frame_buffers[present_idx])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D{x: 0, y: 0},
                    extent: app_obj.surface.surface_resolution,
                })
                .build()
        };

        let device = &app_obj.backend.borrow().device;
        let cmd_buf = app_obj.graphic_cmd_buffer;
        unsafe {
            device.cmd_begin_render_pass(
                cmd_buf,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE
            );
            device.cmd_bind_pipeline(
                cmd_buf,
                vk::PipelineBindPoint::GRAPHICS,
                self.pso_obj.pipeline
            );
            device.cmd_set_viewport(
                cmd_buf,
                0,
                &self.pso_obj.pso_desc.viewports,
            );
            device.cmd_set_scissor(
                cmd_buf,
                0,
                &self.pso_obj.pso_desc.scissors
            );
            device.cmd_bind_vertex_buffers(
                cmd_buf,
                0,
                &[app_obj.buf_mgr_sys.vertex_buffer.buffer],
                &[self.vb.offset],
            );
            device.cmd_bind_index_buffer(
                cmd_buf,
                app_obj.buf_mgr_sys.index_buffer.buffer,
                self.ib.offset,
                vk::IndexType::UINT16
            );
            device.cmd_draw_indexed(
                cmd_buf,
                self.ib.size as u32,
                1, 0, 0, 1
            );
            device.cmd_end_render_pass(
                cmd_buf,
            );
        }
    }

    fn update(&self, app_obj: &app::App, delta_time: f64)
    {

    }
}

impl Drop for TriangleRenderLoop {
    fn drop(&mut self)
    {
        unsafe {
            self.frame_buffers.iter().map(|framebuffer|{
                self.device.destroy_framebuffer(*framebuffer, None);
            }).next();
        }
    }
}

fn main()
{
    println!("current dir: {:?}", std::env::current_dir());
    let app_ci = app::AppCreateInfo {
        app_name: "triangle".to_string(),
        title: "triangle".to_string(),
        width: 800.0,
        height: 600.0,
    };
    let mut app_obj = app::App::new(&app_ci);

    // attachment
    let render_attachment = {
        vec![
            vk::AttachmentDescription {
                format: app_obj.surface.surface_format.format,
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
        ]
    };
    // vert input binding desc
    let vert_input_binding_desc = {
        vec![
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: mem::size_of::<Vertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            }
        ]
    };
    // vert input attr desc
    let vert_input_attr_desc = {
        vec![
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
        ]
    };
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
            width: app_obj.surface.surface_resolution.width as f32,
            height: app_obj.surface.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }],
        scissors: vec![vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: app_obj.surface.surface_resolution.width,
                height: app_obj.surface.surface_resolution.height
            }
        }],
        input_binding_desc: vert_input_binding_desc,
        input_attr_desc: vert_input_attr_desc,
    };
    let pso_obj = utility::create_pipeline_state_object(&app_obj.backend.borrow(), &pso_desc)
        .expect("create pso failed");
    // frame buffer
    let frame_buffers = {
        app_obj.surface
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachments = [present_image_view];
                let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(pso_obj.render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(app_obj.surface.surface_resolution.width)
                    .height(app_obj.surface.surface_resolution.height)
                    .layers(1);
                
                unsafe {
                    app_obj.backend.borrow().device
                        .create_framebuffer(
                            &framebuffer_create_info, None)
                        .unwrap()
                }

            })
            .collect::<Vec<vk::Framebuffer>>()
    };
    // vertex buffer
    let vertices= {
         vec![
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
        ]
    };
    let vb_size = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;
    let mut vb = app_obj.buf_mgr_sys.allocate_vertex_buffer::<Vertex>(vb_size);
    vb.slice.copy_from_slice(&vertices);
    // index buffer
    let ib_data = [0u16, 1, 2];
    let ib_size = (ib_data.len() * std::mem::size_of::<u32>()) as u64;
    let mut ib = app_obj.buf_mgr_sys.allocate_index_buffer(ib_size);
    ib.slice.copy_from_slice(&ib_data);

    let triangle_rl = {
        TriangleRenderLoop {
            device: app_obj.backend.borrow().device.clone(),
            render_pass: pso_obj.render_pass,
            frame_buffers,
            pso_obj,
            vb,
            ib,
        }
    };
    app_obj.render_loop_obj = boxed::Box::new(triangle_rl);

    {
        // // render loop
        // app_obj.render_loop(|| {
        //     let present_index;
        //     unsafe {
        //         let surface = &mut app_obj.surface;
        //         let (_present_index, _) = app_obj.surface
        //             .swapchain
        //             .acquire_next_image(
        //                 surface.swapchain_khr,
        //                 std::u64::MAX,
        //                 app_obj.present_complete_semaphore,
        //                 vk::Fence::null())
        //             .unwrap();
        //         present_index = _present_index;
        //     }
        //
        //     let clear_values = [
        //         vk::ClearValue {
        //             color: vk::ClearColorValue {
        //                 float32: [0.0, 0.0, 0.0, 0.0],
        //             },
        //         },
        //         vk::ClearValue {
        //            depth_stencil: vk::ClearDepthStencilValue {
        //                depth: 1.0,
        //                stencil: 0
        //            }
        //         }
        //     ];
        //
        //     let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
        //         .render_pass(pso_obj.render_pass)
        //         .framebuffer(frame_buffers[present_index as usize])
        //         .render_area(vk::Rect2D {
        //             offset: vk::Offset2D{ x: 0, y: 0},
        //             extent: app_obj.surface.surface_resolution,
        //         })
        //         .clear_values(&clear_values);
        //
        //     app_obj.record_submit_command_buffer(
        //         &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
        //         &[app_obj.present_complete_semaphore],
        //         &[app_obj.rendering_complete_semaphore],
        //         |&_app_obj, draw_command_buffer| unsafe {
        //             _app_obj.device.cmd_begin_render_pass(
        //                 draw_command_buffer,
        //                 &render_pass_begin_info,
        //                 vk::SubpassContents::INLINE
        //             );
        //             _app_obj.device.cmd_bind_pipeline(
        //                 draw_command_buffer,
        //                 vk::PipelineBindPoint::GRAPHICS,
        //                 pso_obj.pipeline
        //             );
        //             _app_obj.device.cmd_set_viewport(
        //                 draw_command_buffer,
        //                 0,
        //                 &pso_obj.pso_desc.viewports,
        //             );
        //             _app_obj.device.cmd_set_scissor(
        //                 draw_command_buffer,
        //                 0,
        //                 &pso_obj.pso_desc.scissors,
        //             );
        //             _app_obj.device.cmd_bind_vertex_buffers(
        //                 draw_command_buffer,
        //                 0,
        //                 &[_app_obj.buffer.vertex_buffer.buffer],
        //                 &[vb.offset]
        //             );
        //             _app_obj.device.cmd_bind_index_buffer(
        //                 draw_command_buffer,
        //                 _app_obj.buffer.index_buffer.buffer,
        //                 ib.offset,
        //                 vk::IndexType::UINT32
        //             );
        //             _app_obj.backend.device.cmd_draw_indexed(
        //                 draw_command_buffer,
        //                 ib_size as u32,
        //                 1,
        //                 0,
        //                 0,
        //                 1
        //             );
        //             _app_obj.backenddevice.cmd_end_render_pass(draw_command_buffer);
        //         },
        //     );
        //
        //     let wait_semaphore = [backend.rendering_complete_semaphore];
        //     let swapchains = [backend.swapchain];
        //     let image_indices = [present_index];
        //     let present_info = vk::PresentInfoKHR::builder()
        //         .wait_semaphores(&wait_semaphore)
        //         .swapchains(&swapchains)
        //         .image_indices(&image_indices);
        //
        //     unsafe {
        //         backend.swapchain_loader
        //             .queue_present(backend.present_queue, &present_info)
        //             .unwrap();
        //     }
        //
        // });
    }

    app_obj.render_loop();
}

