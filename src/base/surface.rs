use ash::version::*;
use ash::extensions::{
    ext, khr
};
use ash::vk;
use std::rc::Rc;
use super::ri::Backend;
use super::pso::{PipelineStateObjectDescriptor, ShaderProgramDescriptor, PipelineStateObject};
use std::ffi::CString;
use super::utility;
use std::boxed::Box;
use super::buffer;

pub struct Surface {
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,
    pub swapchain: khr::Swapchain,
    pub swapchain_khr: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
    pub surface_pso_obj: Box<PipelineStateObject>,
    pub surface_frame_buffers: Vec<vk::Framebuffer>,
    backend: Rc<Backend>,
}

impl Surface {
    pub fn new(backend: Rc<Backend>, window: &winit::Window)
               -> Self
    {
        let surface = &backend.surface;
        let surface_khr = backend.surface_khr.clone();
        let surface_format = {
            let surface_formats = unsafe {
                backend.surface
                    .get_physical_device_surface_formats(backend.physical_device, surface_khr)
                    .unwrap()
            };
            surface_formats
                .iter()
                .map(|sfmt| match sfmt.format {
                    vk::Format::UNDEFINED => vk::SurfaceFormatKHR {
                        format: vk::Format::R8G8B8A8_SNORM,
                        color_space: sfmt.color_space,
                    },
                    _ => *sfmt
                })
                .next()
                .unwrap()
        };
        let surface_resolution = vk::Extent2D { width: 800, height: 600 };
        let swapchain = khr::Swapchain::new(&backend.instance, &backend.device);
        let swapchain_khr = {
            let surface_capabilities = unsafe {
                surface
                    .get_physical_device_surface_capabilities(
                        backend.physical_device, surface_khr
                    ).unwrap()
            };
            let desired_image_count = std::cmp::max(1u32, surface_capabilities.min_image_count);
            let pre_transform = surface_capabilities.current_transform;
            let present_mode = {
                let present_modes = unsafe {
                    surface
                        .get_physical_device_surface_present_modes(
                            backend.physical_device, surface_khr
                        ).unwrap()
                };
                present_modes
                    .iter()
                    .cloned()
                    .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                    .unwrap_or(vk::PresentModeKHR::FIFO)
            };
            let swapchain_ci = vk::SwapchainCreateInfoKHR {
                surface: surface_khr,
                min_image_count: desired_image_count,
                image_color_space: surface_format.color_space,
                image_format: surface_format.format,
                image_extent: surface_capabilities.min_image_extent,
                image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                image_sharing_mode: vk::SharingMode::EXCLUSIVE,
                image_array_layers: 1,
                pre_transform,
                composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                present_mode,
                clipped: true.into(),
                ..Default::default()
            };

            unsafe {
                swapchain
                    .create_swapchain(&swapchain_ci, None)
                    .unwrap()
            }
        };
        let present_images = unsafe {
            swapchain.get_swapchain_images(swapchain_khr).unwrap()
        };
        let present_image_views= unsafe {
            present_images
                .iter()
                .map(|&image| {
                    let image_view_ci = vk::ImageViewCreateInfo {
                        view_type: vk::ImageViewType::TYPE_2D,
                        format: surface_format.format,
                        components: vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
                        },
                        subresource_range: vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1
                        },
                        image: image,
                        ..Default::default()
                    };
                    backend.device.create_image_view(&image_view_ci, None).unwrap()
                })
                .collect::<Vec<vk::ImageView>>()
        };

        let surface_pso_obj = {
            let vert_input_binding_desc = {
                vec![
                    vk::VertexInputBindingDescription {
                        binding: 0,
                        stride: 8,
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
                        offset: 0,
                        ..Default::default()
                    }
                ]
            };
            let render_attachment = {
                vec![
                    vk::AttachmentDescription {
                        format: surface_format.format,
                        samples: vk::SampleCountFlags::TYPE_1,
                        load_op: vk::AttachmentLoadOp::CLEAR,
                        store_op: vk::AttachmentStoreOp::STORE,
                        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                        ..Default::default()
                    }
                ]
            };

            let pso_desc = PipelineStateObjectDescriptor {
                vs_desc: ShaderProgramDescriptor {
                    path: "./shader/full_screen/full_screen.vert".to_string(),
                    entry: CString::new("main").unwrap(),
                },
                ps_desc: ShaderProgramDescriptor {
                    path: "./shader/full_screen/full_screen.frag".to_string(),
                    entry: CString::new("main").unwrap(),
                },
                attachment_desc: render_attachment, // move
                viewports: vec![vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: surface_resolution.width as f32,
                    height: surface_resolution.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
                scissors: vec![vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: surface_resolution.width,
                        height: surface_resolution.height
                    }
                }],
                input_binding_desc: vert_input_binding_desc,
                input_attr_desc: vert_input_attr_desc,
            };

            utility::create_pipeline_state_object(&backend, &pso_desc)
                .expect("create surface pso obj failed")
        };
        let surface_frame_buffers = {
            present_image_views
                .iter()
                .map(|&present_image_view| {
                    let framebuffer_attachments = [present_image_view];
                    let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
                        .render_pass(surface_pso_obj.render_pass)
                        .attachments(&framebuffer_attachments)
                        .width(surface_resolution.width)
                        .height(surface_resolution.height)
                        .layers(1);

                    unsafe {
                        backend.device
                            .create_framebuffer(
                                &framebuffer_create_info, None)
                            .unwrap()
                    }

                })
                .collect::<Vec<vk::Framebuffer>>()
        };

        Surface {
            surface_format,
            surface_resolution,
            swapchain,
            swapchain_khr,
            present_images,
            present_image_views,
            surface_pso_obj,
            surface_frame_buffers,
            backend,
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self)
    {
        unsafe {
            for &image_view in self.present_image_views.iter()
            {
                self.backend.device.destroy_image_view(image_view, None);
            }
            self.swapchain.destroy_swapchain(self.swapchain_khr, None);
        }
    }
}
