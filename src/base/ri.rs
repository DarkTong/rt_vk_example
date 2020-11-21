extern crate ash;
extern crate winit;

use ash::extensions::{
    ext, khr
};
use ash::version::*;
use ash::vk;
use std::ffi::{CString, CStr};
use std::cell::Cell;
use std::rc::Rc;

pub struct Backend {
    pub entry: ash::Entry, // vulkan函数入口
    pub instance: ash::Instance,
    pub debug_utils: ext::DebugUtils,
    pub debug_callback: vk::DebugUtilsMessengerEXT,
    pub physical_device: vk::PhysicalDevice,
    pub queue_family_index: u32,
    pub device: ash::Device,
}

pub struct Surface {
    pub surface: khr::Surface,
    pub surface_khr: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,
    pub swapchain: khr::Swapchain,
    pub swapchain_khr: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
    backend: Rc<Backend>,
}

impl Backend {
    pub fn new(window: &winit::Window, select_gpu_idx: usize) -> Self
    {
        let entry = ash::Entry::new().unwrap();
        let instance = {
            let app_name = CString::new("rt_vt_exp").unwrap();
            let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
            let layer_names_raw = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect::<Vec<*const i8>>();

            let extension_name_raw;
            let mut surface_extensions = ash_window::enumerate_required_extensions(window).unwrap();
            surface_extensions.push(&ext::DebugUtils::name());
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
                entry.create_instance(&create_info, None)
                    .expect("Instance creation error")
            }
        };
        let debug_utils = ext::DebugUtils::new(&entry, &instance);
        let debug_callback = {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR |
                        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                        vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::all()
                )
                .pfn_user_callback(Some(Backend::vulkan_debug_callback));

            unsafe {
                debug_utils.create_debug_utils_messenger(&debug_info, None)
                    .unwrap()
            }
        };
        let physical_devices = unsafe {
            let _p_d = instance.enumerate_physical_devices()
                .expect("Physical device error");
            assert!(_p_d.len() > 0, "Get physical device number is zero");
            _p_d
        };
        let physical_device = {
            assert!(select_gpu_idx < physical_devices.len(),
                    format!("Select physical device is error. sum is {}, select is {}",
                        physical_devices.len(), select_gpu_idx)
            );
            physical_devices[select_gpu_idx]
        };
        let queue_family_index = select_gpu_idx as u32;
        let device = {
            let device_extension_names_raw = [khr::Swapchain::name().as_ptr()];
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
                instance
                    .create_device(physical_device, &device_create_info, None)
                    .unwrap()
            }
        };

        Backend {
            entry,
            instance,
            debug_utils,
            debug_callback,
            physical_device,
            queue_family_index,
            device
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

    pub fn get_queue_family_index(&self, flags: vk::QueueFlags)
        -> u32
    {
        let props = unsafe {
            self.instance.get_physical_device_queue_family_properties(self.physical_device)
        };
        let f_get_queue =
            |support_flags: vk::QueueFlags| {
                props.iter()
                    .enumerate()
                    .filter_map(|(idx, prop)| {
                        let prop_flags = &prop.queue_flags;
                        match prop_flags.contains(support_flags) {
                            true => Some(idx),
                            false => None,
                        }
                    })
                    .next()
        };
        if flags.contains(vk::QueueFlags::COMPUTE) {
            let queue_idx = {
                let support_flags = vk::QueueFlags::COMPUTE;
                let ignore_flags = vk::QueueFlags::GRAPHICS & vk::QueueFlags::TRANSFER;
                f_get_queue(support_flags)
            };
            if queue_idx.is_some() {
                return queue_idx.unwrap() as u32;
            }
        }
        else if flags.contains(vk::QueueFlags::TRANSFER) {
            let queue_idx = {
                let support_flags = vk::QueueFlags::TRANSFER;
                let ignore_flags = vk::QueueFlags::GRAPHICS & vk::QueueFlags::COMPUTE;
                f_get_queue(support_flags)
            };
            if queue_idx.is_some() {
                return queue_idx.unwrap() as u32;
            }
        }
        else if flags.contains(vk::QueueFlags::GRAPHICS) {
            let queue_idx = {
                let support_flags = vk::QueueFlags::GRAPHICS;
                let ignore_flags = vk::QueueFlags::COMPUTE & vk::QueueFlags::TRANSFER;
                f_get_queue(support_flags)
            };
            if queue_idx.is_some() {
                return queue_idx.unwrap() as u32;
            }
        }
        if flags.contains(vk::QueueFlags::GRAPHICS) {
            let queue_idx = {
                let support_flags = vk::QueueFlags::GRAPHICS;
                f_get_queue(support_flags)
            };
            if queue_idx.is_some() {
                return queue_idx.unwrap() as u32;
            }
        }
        panic!("wrong queue flags");
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_device(None);
            self.debug_utils.destroy_debug_utils_messenger(self.debug_callback, None);
            self.instance.destroy_instance(None);
        }
    }
}

impl Surface {
    pub fn new(backend: Rc<Backend>, window: &winit::Window)
        -> Self
    {
        let surface_khr = unsafe {
            ash_window::create_surface(
                &backend.entry, &backend.instance, window, None
            ).unwrap()
        };
        let surface = khr::Surface::new(&backend.entry, &backend.instance);
        let surface_format = {
            let surface_formats = unsafe {
                surface
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
                image_extent: surface_resolution,
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

        Surface {
            surface,
            surface_khr,
            surface_format,
            surface_resolution,
            swapchain,
            swapchain_khr,
            present_images,
            present_image_views,
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
            self.surface.destroy_surface(self.surface_khr, None)
        }
    }
}
