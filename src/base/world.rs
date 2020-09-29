extern crate ash;
extern crate winit;

use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
};

pub use ash::version::*;
pub use ash::version::DeviceV1_0;
use ash::{vk, Device, Entry, Instance};
use std::ffi::{CString, CStr};
use super::pso::*;
use super::loader;
use super::buffer;

static VERTEX_BUFFER_SIZE: u64 = 4 * 1024 * 1024;
static INDEX_BUFFER_SIZE: u64 = 4 * 1024 * 1024;
static UNIFORM_BUFFER_SIZE: u64 = 1024 * 1024;


pub struct Backend {
    pub entry: Entry,
    pub instance: Instance,
    pub debug_utils_loader: DebugUtils,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,
    pub surface : vk::SurfaceKHR,
    pub surface_loader: Surface,
    pub pdevice: vk::PhysicalDevice,
    pub queue_family_index: u32,
    pub device: Device,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub present_queue: vk::Queue,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_loader: Swapchain,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
    pub depth_image: vk::Image,
    pub depth_image_view: vk::ImageView,
    pub depth_image_memory: vk::DeviceMemory,
    pub pool: vk::CommandPool,
    pub draw_command_buffer: vk::CommandBuffer,
    pub setup_command_buffer: vk::CommandBuffer,
    pub present_complete_semaphore: vk::Semaphore,
    pub rendering_complete_semaphore: vk::Semaphore,
    // buffer
    pub index_buffer: buffer::DeviceBuffer,
    pub vertex_buffer: buffer::DeviceBuffer,
    pub uniform_buffer: buffer::DeviceBuffer,
}

pub struct InstanceCreateInfo {
    pub app_name: String,
    pub window_height: f32,
    pub window_width: f32
}

impl Backend {
    pub fn new(create_info: &InstanceCreateInfo, window: &winit::Window) -> Self {
        let entry = Entry::new().unwrap();
        let instance: Instance;
        {
            let app_name = CString::new(create_info.app_name.clone()).unwrap();
            let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
            // let layer_names_raw: Vec<*const i8>;
            let layer_names_raw = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect::<Vec<*const i8>>();

            let extension_name_raw;
            let mut surface_extensions = ash_window::enumerate_required_extensions(window).unwrap();
            surface_extensions.push(&DebugUtils::name());
            extension_name_raw = surface_extensions.iter()
                .map(|ext| ext.as_ptr())
                .collect::<Vec<*const i8>>(); 

            let app_info = vk::ApplicationInfo::builder()
                .application_name(&app_name)
                .application_version(0)
                .engine_name(&app_name)
                .engine_version(0)
                .api_version(vk::make_version(1, 0, 0));

            let create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_layer_names(&layer_names_raw)
                .enabled_extension_names(&extension_name_raw);

            unsafe {
                instance = entry
                    .create_instance(&create_info, None)
                    .expect("Instance creation error");
            }
        }
        let debug_utils_loader = DebugUtils::new(&entry, &instance);
        let debug_callback;
        {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR |
                    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                    vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::all()
                )
                .pfn_user_callback(Some(vulkan_debug_callback));
            
            unsafe {
                debug_callback = debug_utils_loader
                    .create_debug_utils_messenger(&debug_info, None)
                    .unwrap();
            }
        }
        let surface;
        unsafe {
            surface = ash_window::create_surface(
                &entry, &instance, window, None)
                .unwrap();
        }
        let surface_loader = Surface::new(&entry, &instance);
        let pdevice;
        let queue_family_index;
        {
            let pdevices;
            unsafe {
                pdevices = instance.enumerate_physical_devices()
                    .expect("Physical device error");
            }
            let result = 
                pdevices.iter()
                .map(|pdevice| {
                    let _iter;
                    unsafe {
                        _iter = instance.get_physical_device_queue_family_properties(*pdevice);
                    }
                    _iter
                    .iter()
                    .enumerate()
                    .filter_map(|(index, ref info)| {
                        let support_;
                        unsafe {
                            support_ = surface_loader.get_physical_device_surface_support(
                                *pdevice, 
                                index as u32, 
                                surface).unwrap();
                        }
                        let supports_grahic_and_surface = 
                            info.queue_flags.contains(vk::QueueFlags::GRAPHICS) && support_;

                        match supports_grahic_and_surface {
                            true => Some((*pdevice, index as u32)),
                            false => None
                        }
                    })
                    .next()
                })
                .filter_map(|v| v)
                .next()
                .expect("Couldn't find suitable device.");
            
            pdevice = result.0;
            queue_family_index = result.1;
        }
        let device;
        {

            let device_extension_names_raw = [Swapchain::name().as_ptr()];
            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };
            let priorities = [1.0];
            let queue_create_infos = [vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities)
                .build(),
            ];
            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);
            unsafe {
                device = instance
                    .create_device(pdevice, &device_create_info, None)
                    .unwrap();
            }
        }
        let present_queue;
        unsafe {
            present_queue = device.get_device_queue(queue_family_index, 0);
        }
        let surface_format;
        {
            let surface_formats;
            unsafe {
                surface_formats = surface_loader
                    .get_physical_device_surface_formats(pdevice, surface)
                    .unwrap();
            }
            surface_format = surface_formats
                .iter()
                .map(|sfmt| match sfmt.format {
                    vk::Format::UNDEFINED => vk::SurfaceFormatKHR {
                        format: vk::Format::B8G8R8_UNORM,
                        color_space: sfmt.color_space,
                    },
                    _ => *sfmt
                })
                .next()
                .unwrap()
        }
        let swapchain_loader = Swapchain::new(&instance, &device);
        let surface_resolution;
        let swapchain;
        {
            let surface_capabilities;
            unsafe {
                surface_capabilities = surface_loader
                    .get_physical_device_surface_capabilities(pdevice, surface)
                    .unwrap()
            }
            surface_resolution = match surface_capabilities.current_extent.width {
                std::u32::MAX => vk::Extent2D {
                    width: create_info.window_width as u32,
                    height: create_info.window_height as u32,
                },
                _ => surface_capabilities.current_extent
            };

            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
            let pre_transform = if surface_capabilities.supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY) {
                    vk::SurfaceTransformFlagsKHR::IDENTITY
                } else {
                    surface_capabilities.current_transform
                };
            let present_modes;
            unsafe {
                present_modes = surface_loader
                    .get_physical_device_surface_present_modes(pdevice, surface)
                    .unwrap();
            }
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);
            let swapchain_ci = vk::SwapchainCreateInfoKHR {
                surface: surface,
                min_image_count: desired_image_count,
                image_color_space: surface_format.color_space,
                image_format: surface_format.format,
                image_extent: surface_resolution,
                image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                image_sharing_mode: vk::SharingMode::EXCLUSIVE,
                image_array_layers: 1,
                pre_transform: pre_transform,
                composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                present_mode: present_mode,
                clipped: true.into(),
                ..Default::default()
            };
            
            unsafe {
                swapchain = swapchain_loader
                    .create_swapchain(&swapchain_ci, None)
                    .unwrap();

            }
        }        
        let present_images;
        let present_image_views;
        unsafe {
            present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
            present_image_views = present_images
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
                    device.create_image_view(&image_view_ci, None).unwrap()
                })
                .collect::<Vec<vk::ImageView>>();
        }

        let device_memory_properties;
        unsafe {
            device_memory_properties = instance.get_physical_device_memory_properties(pdevice);
        }
        let depth_image;
        let depth_image_view;
        let depth_image_memory_index;
        let depth_image_memory;
        {
            let depth_image_ci = vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::D16_UNORM,
                extent: vk::Extent3D {
                    width: surface_resolution.width,
                    height: surface_resolution.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            unsafe {
                depth_image = device.create_image(&depth_image_ci, None).unwrap();
            }
            unsafe {
                let depth_image_memory_req = device.get_image_memory_requirements(depth_image);
                depth_image_memory_index = find_memorytype_index(
                    &depth_image_memory_req,
                    &device_memory_properties,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL).unwrap();
                let depth_image_ai = vk::MemoryAllocateInfo {
                    allocation_size: depth_image_memory_req.size,
                    memory_type_index: depth_image_memory_index,
                    ..Default::default()
                };
                depth_image_memory = device
                    .allocate_memory(&depth_image_ai, None)
                    .unwrap();

                device.bind_image_memory(depth_image, depth_image_memory, 0)
                    .unwrap();

                let depth_image_view_ci = vk::ImageViewCreateInfo {
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: depth_image_ci.format,
                    image: depth_image,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::DEPTH,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                depth_image_view = device
                    .create_image_view(&depth_image_view_ci, None)
                    .unwrap();
            }
           
        }

        let present_complete_semaphore;
        let rendering_complete_semaphore;
        unsafe {
            let semaphore_ci = vk::SemaphoreCreateInfo::default();
            present_complete_semaphore = device
                .create_semaphore(&semaphore_ci, None)
                .unwrap();
            rendering_complete_semaphore = device
                .create_semaphore(&semaphore_ci, None)
                .unwrap();
        }
 
        let pool;
        unsafe {
            let pool_ci  = vk::CommandPoolCreateInfo {
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index: queue_family_index,
                ..Default::default()
            };
            pool = device.create_command_pool(&pool_ci, None).unwrap();
        }
        let setup_command_buffer;
        let draw_command_buffer;
        unsafe {
            let command_buffer_ai = vk::CommandBufferAllocateInfo {
                command_buffer_count: 2,
                command_pool: pool,
                level: vk::CommandBufferLevel::PRIMARY,
                ..Default::default()
            };
            let command_buffers = device
                .allocate_command_buffers(&command_buffer_ai)
                .unwrap();
            setup_command_buffer = command_buffers[0];
            draw_command_buffer = command_buffers[1];
        }

        // buffer
        let vertex_buffer_ci = vk::BufferCreateInfo::builder()
            .size(VERTEX_BUFFER_SIZE )
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        let vertex_buffer = buffer::DeviceBuffer::new(
            &device,
            &device_memory_properties,
            &vertex_buffer_ci,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        let index_buffer_ci = vk::BufferCreateInfo::builder()
            .size(INDEX_BUFFER_SIZE)
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        let index_buffer = buffer::DeviceBuffer::new(
            &device,
            &device_memory_properties,
            &index_buffer_ci,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        let uniform_buffer_ci = vk::BufferCreateInfo::builder()
            .size(UNIFORM_BUFFER_SIZE)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        let uniform_buffer = buffer::DeviceBuffer::new(
            &device,
            &device_memory_properties,
            &uniform_buffer_ci,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        record_submit_commandbuffer(
            &device,
            setup_command_buffer,
            present_queue,
            &[],
            &[],
            &[],
            |device, setup_command_buffer| {
                let layout_transition_barriers = vk::ImageMemoryBarrier {
                    image: depth_image,
                    dst_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                        | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    new_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    old_layout: vk::ImageLayout::UNDEFINED,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::DEPTH,
                        layer_count: 1,
                        level_count : 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                unsafe {
                    device.cmd_pipeline_barrier(
                        setup_command_buffer, 
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE, 
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS, 
                        vk::DependencyFlags::empty(), 
                        &[], 
                        &[], 
                        &[layout_transition_barriers],
                    )
                }
            },
        );

        Backend {
            entry: entry,
            instance: instance,
            debug_utils_loader: debug_utils_loader,
            debug_call_back: debug_callback,
            surface: surface,
            surface_loader: surface_loader,
            pdevice: pdevice,
            queue_family_index: queue_family_index,
            device: device,
            device_memory_properties: device_memory_properties,
            present_queue: present_queue,
            surface_format: surface_format,
            surface_resolution: surface_resolution,
            swapchain_loader: swapchain_loader,
            swapchain: swapchain,
            present_images: present_images,
            present_image_views: present_image_views,
            depth_image_memory: depth_image_memory,
            depth_image: depth_image,
            depth_image_view: depth_image_view,
            pool: pool,
            setup_command_buffer: setup_command_buffer,
            draw_command_buffer: draw_command_buffer,
            present_complete_semaphore: present_complete_semaphore,
            rendering_complete_semaphore: rendering_complete_semaphore,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
        }

    }

    pub fn create_pipeline_state_object(&self, desc: &PipelineStateObjectDescriptor)
     -> std::io::Result<Box<PipelineStateObject>>
    {
        let vs_mod = loader::load_shader(&self.device, &desc.vs_desc.path)
            .expect("vs shader create failed");
        let ps_mod = loader::load_shader(&self.device, &desc.ps_desc.path)
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
                render_pass = self.device
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
            pipeline_layout = self.device
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
            pipeline = self.device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[pipeline_ci],
                    None,
                ).expect("unable to create graphic pipeline");
        }

        Ok(Box::new(PipelineStateObject{
            pso_desc: desc.clone(),
            vs_mod,
            ps_mod,
            render_pass,
            pipeline_layout,
            pipeline: pipeline[0],
            device: self.device.clone(),
        }))
    }

    pub fn allocate_vertex_buffer<T>(&mut self, size: u64)
        -> buffer::BufferSlice<T>
    {
        self.vertex_buffer.allocate::<T>(size)
    }

    pub fn allocate_index_buffer<T>(&mut self, size: u64)
        -> buffer::BufferSlice<T>
    {
        self.index_buffer.allocate::<T>(size)
    }
}


impl Drop for Backend {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device
                .destroy_semaphore(self.present_complete_semaphore, None);
            self.device
                .destroy_semaphore(self.rendering_complete_semaphore, None);
            self.device.free_memory(self.depth_image_memory, None);
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            for &image_view in self.present_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.device.destroy_command_pool(self.pool, None);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {

    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;
    let message_id_name = CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy();
    let message = CStr::from_ptr(callback_data.p_message).to_string_lossy();

    println!(
        "{:?}:{:?} [{}:{}] :\n{}\n",
        message_severity,
        message_type,
        &message_id_number.to_string(),
        message_id_name,
        message
    );

    return vk::FALSE;
}

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


pub fn record_submit_commandbuffer<D: DeviceV1_0, F: FnOnce(&D, vk::CommandBuffer)>(
    device: &D,
    command_buffer: vk::CommandBuffer,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) {
    unsafe {
        device
            .reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
            .expect("Reset command buffer failed.");

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .expect("Begin commandbuffer");
        f(device, command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("End commandbuffer");

        let submit_fence = device
            .create_fence(&vk::FenceCreateInfo::default(), None)
            .expect("Create fence failed.");

        let command_buffers = vec![command_buffer];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(signal_semaphores);

        device
            .queue_submit(submit_queue, &[submit_info.build()], submit_fence)
            .expect("queue submit failed.");
        device
            .wait_for_fences(&[submit_fence], true, std::u64::MAX)
            .expect("Wait for fence failed.");
        device.destroy_fence(submit_fence, None);
    }
}
