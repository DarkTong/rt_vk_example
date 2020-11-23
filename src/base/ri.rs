extern crate ash;
extern crate winit;

use ash::extensions::{
    ext, khr
};
use ash::version::*;
use ash::vk;
use std::ffi::{CString, CStr};
use std::rc::Rc;

pub struct Backend {
    pub entry: ash::Entry, // vulkan函数入口
    pub instance: ash::Instance,
    pub debug_utils: ext::DebugUtils,
    pub debug_callback: vk::DebugUtilsMessengerEXT,
    pub physical_device: vk::PhysicalDevice,
    pub surface_khr: vk::SurfaceKHR,
    pub surface: khr::Surface,
    pub queue_family_index: u32,
    pub device: ash::Device,
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
        let surface_khr = unsafe {
            ash_window::create_surface(
                &entry, &instance, window, None
            ).unwrap()
        };
        let surface = khr::Surface::new(&entry, &instance);
        let graphic_queue_family_index = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
                .iter()
                .enumerate()
                .filter_map(|(index, ref info)|{
                    let supports_graphic_and_surface =
                        info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                            && surface
                                .get_physical_device_surface_support(
                                    physical_device,
                                    index as u32,
                                    surface_khr,
                                )
                                .unwrap();
                    if supports_graphic_and_surface {
                        Some(index)
                    } else {
                        None
                    }
                })
                .next()
                .unwrap()
                as u32
        };

        let device = {
            let device_extension_names_raw = [khr::Swapchain::name().as_ptr()];
            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };
            let priorities = [1.0];
            let queue_create_infos = [vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(graphic_queue_family_index)
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
            surface_khr,
            surface,
            queue_family_index: graphic_queue_family_index,
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
                // let ignore_flags = vk::QueueFlags::GRAPHICS & vk::QueueFlags::TRANSFER;
                f_get_queue(support_flags)
            };
            if queue_idx.is_some() {
                return queue_idx.unwrap() as u32;
            }
        }
        else if flags.contains(vk::QueueFlags::TRANSFER) {
            let queue_idx = {
                let support_flags = vk::QueueFlags::TRANSFER;
                // let ignore_flags = vk::QueueFlags::GRAPHICS & vk::QueueFlags::COMPUTE;
                f_get_queue(support_flags)
            };
            if queue_idx.is_some() {
                return queue_idx.unwrap() as u32;
            }
        }
        else if flags.contains(vk::QueueFlags::GRAPHICS) {
            let queue_idx = {
                let support_flags = vk::QueueFlags::GRAPHICS;
                // let ignore_flags = vk::QueueFlags::COMPUTE & vk::QueueFlags::TRANSFER;
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
            self.surface.destroy_surface(self.surface_khr, None)
        }
    }
}


