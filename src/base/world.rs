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
use std::cell::RefCell;
use std::borrow::Cow;
use super::pso::*;
use super::loader;

pub struct InstanceBase {
    pub events_loop: RefCell<winit::EventsLoop>,
    pub entry: Entry,
    pub instance: Instance,
    pub window: winit::Window,
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
}

pub struct InstanceCreateInfo {
    pub window_width: u32,
    pub window_height: u32,
    pub app_name: String
}

impl InstanceBase {
    pub fn render_loop<F: Fn()>(&self, f:F) {
        use winit::*;
        self.events_loop.borrow_mut().run_forever(|event| {
            f();
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput {input, ..} => {
                        if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                            ControlFlow::Break
                        } else {
                            ControlFlow::Continue
                        }
                    },
                    WindowEvent::CloseRequested => ControlFlow::Break,
                    _ => ControlFlow::Continue,
                },
                _ => ControlFlow::Continue,
            }
        });

    }

    pub fn new(create_info: InstanceCreateInfo) -> Self {
        let events_loop = winit::EventsLoop::new();
        let entry = Entry::new().unwrap();
       
        let window; 
        {
            window = winit::WindowBuilder::new()
            .with_title("window-title")
            .with_dimensions(winit::dpi::LogicalSize::new(
                f64::from(create_info.window_width),
                f64::from(create_info.window_height)))
            .build(&events_loop)
            .unwrap();
        }
        let instance: Instance;
        {
            let app_name = CString::new(create_info.app_name).unwrap();
            let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
            // let layer_names_raw: Vec<*const i8>;
            let layer_names_raw = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect::<Vec<*const i8>>();

            let extension_name_raw;
            let mut surface_extensions = ash_window::enumerate_required_extensions(&window).unwrap();
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
                &entry, &instance, &window, None)
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
                    width: create_info.window_width,
                    height: create_info.window_height,
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

                device.bind_image_memory(depth_image, depth_image_memory, 0);

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
                    new_layout: vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
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

        InstanceBase {
            events_loop: RefCell::new(events_loop),
            entry: entry,
            instance: instance,
            window: window,
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
        }

    }

    // pub fn create_pipeline_state_object(&self, desc: PipelineStateObjectDescriptor
    // ) -> Box<PipelineStateObject>
    // {
    //     Box::new(PipelineStateObject {

    //     })
    // }

}

impl Drop for InstanceBase {
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
    let message_id_name = match callback_data.p_message_id_name.is_null() {
        false => Cow::from(""),
        true => CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy(),
    };
    let message = match callback_data.p_message.is_null() {
        false => Cow::from(""),
        true => CStr::from_ptr(callback_data.p_message).to_string_lossy(),
    };

    println!(
        "{:?}:{:?} [{} {}] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
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
