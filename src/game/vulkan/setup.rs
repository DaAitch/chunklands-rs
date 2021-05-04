use super::{
    error::{maybe_vulkan_error, to_vulkan},
    util::{cchar_to_string, CStrings},
    version::VulkanVersion,
    QueueFamilies, QueueFamilyIndices, Result, Vulkan, VulkanInit,
};
use crate::game::vulkan::{
    error::{to_other, Error},
    Context, InFlightFrame, MAX_FRAMES_IN_FLIGHT,
};
use log::{error, info, log, Level};
use std::{
    collections::HashSet,
    ffi::{c_void, CString},
    mem, ptr,
};
use vk_sys as vk;
use vulkanic::{DevicePointers, EntryPoints, InstancePointers};

impl Vulkan {
    pub fn new(init: VulkanInit) -> Result<Self> {
        let ep: EntryPoints = vk::EntryPoints::load(|procname| {
            init.window
                .get_instance_proc_address(0, procname.to_str().unwrap())
        })
        .into();

        let instance = Self::create_instance(&ep, init.req_layers, init.req_ext, init.debug)?;
        let ip: InstancePointers = vk::InstancePointers::load(|procname| {
            init.window
                .get_instance_proc_address(instance, procname.to_str().unwrap())
        })
        .into();
        let dp: DevicePointers = vk::DevicePointers::load(|procname| {
            init.window
                .get_instance_proc_address(instance, procname.to_str().unwrap())
        })
        .into();

        let debugger = if init.debug {
            Self::create_debug_messenger(&ip, instance)?
        } else {
            vk::NULL_HANDLE
        };

        let surface = Self::create_surface(init.window, instance)?;

        let req_dev_exts = vec!["VK_KHR_swapchain".to_owned()];

        let physical_device = Self::find_physical_device(&ip, instance, &req_dev_exts)?;
        let queue_family_indices = Self::find_queue_families(&ip, physical_device, surface)?;

        let device =
            Self::create_device(&ip, physical_device, &queue_family_indices, &req_dev_exts)?;
        let queues = Self::get_device_queue_families(&dp, device, &queue_family_indices);

        let command_pool = Self::create_command_pool(&dp, device, &queue_family_indices)?;
        let memory_properties = ip.get_physical_device_memory_properties(physical_device);

        let ctx = Context {
            instance,
            ip,
            debugger,
            dp,
            physical_device,
            device,
            queue_family_indices,
            queue_families: queues,
            surface,
            command_pool,
            memory_properties,
        };

        let mut inflight_frames = Vec::<InFlightFrame>::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let frame = InFlightFrame::new(&ctx)?;
            inflight_frames.push(frame);
        }

        Ok(Vulkan {
            ctx,
            inflight_frames,
            current_frame: 0,
            sc_ctx: None,
        })
    }

    pub fn destroy(mut self) -> Result<()> {
        for inflight_frame in self.inflight_frames.drain(..) {
            inflight_frame.destroy(&self.ctx);
        }

        self.sc_ctx.take().map(|sc| sc.destroy(&self.ctx));

        self.ctx
            .dp
            .destroy_command_pool(self.ctx.device, self.ctx.command_pool);
        self.ctx.command_pool = vk::NULL_HANDLE;

        self.ctx.dp.destroy_device(self.ctx.device);
        self.ctx.device = 0;

        self.ctx
            .ip
            .destroy_surface_khr(self.ctx.instance, self.ctx.surface);
        self.ctx.surface = vk::NULL_HANDLE;

        if self.ctx.debugger != vk::NULL_HANDLE {
            self.ctx
                .ip
                .destroy_debug_utils_messenger_ext(self.ctx.instance, self.ctx.debugger)
                .map_err(to_vulkan)?;
            self.ctx.debugger = vk::NULL_HANDLE;
        }

        self.ctx.ip.destroy_instance(self.ctx.instance);
        self.ctx.instance = 0;

        Ok(())
    }

    fn create_instance(
        ep: &EntryPoints,
        required_layers: &Vec<String>,
        required_extensions: &Vec<String>,
        debug: bool,
    ) -> Result<vk::Instance> {
        let app_name = CString::new("chunklands").unwrap();
        let engine_name = CString::new("crankshaft").unwrap();
        let app_info = vk::ApplicationInfo {
            sType: vk::STRUCTURE_TYPE_APPLICATION_INFO,
            pNext: std::ptr::null(),
            pApplicationName: app_name.as_ptr(),
            applicationVersion: VulkanVersion::new(0, 0, 1).get_compact(),
            pEngineName: engine_name.as_ptr(),
            engineVersion: VulkanVersion::new(0, 0, 1).get_compact(),
            apiVersion: VulkanVersion::new(1, 0, 0).get_compact(),
        };

        let (layers, extensions) = if debug {
            let mut req_dbg_layers = required_layers.clone();
            req_dbg_layers.push("VK_LAYER_KHRONOS_validation".to_owned());
            Self::check_required_layers(ep, &req_dbg_layers)?;

            let mut req_dbg_ext = required_extensions.clone();
            req_dbg_ext.push("VK_EXT_debug_utils".to_owned());
            Self::check_required_extensions(ep, &req_dbg_ext)?;

            (
                CStrings::new(&req_dbg_layers).unwrap(),
                CStrings::new(&req_dbg_ext).unwrap(),
            ) // TODO unwrap
        } else {
            Self::check_required_extensions(ep, &required_extensions)?;

            (
                CStrings::new(&Vec::<String>::new()).unwrap(),
                CStrings::new(&required_extensions).unwrap(),
            ) // TODO unwrap
        };

        let mut debug_info = Self::create_debugger_info();

        let instance_info = vk::InstanceCreateInfo {
            sType: vk::STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            flags: 0,
            pApplicationInfo: &app_info,
            enabledLayerCount: layers.len() as u32,
            ppEnabledLayerNames: layers.as_ptr(),
            enabledExtensionCount: extensions.len() as u32,
            ppEnabledExtensionNames: extensions.as_ptr(),
            pNext: if debug {
                &mut debug_info as *mut _ as *mut c_void
            } else {
                std::ptr::null()
            },
        };

        unsafe { ep.create_instance(&instance_info) }.map_err(to_vulkan)
    }

    fn check_required_layers(ep: &EntryPoints, required_layers: &Vec<String>) -> Result<()> {
        let layer_properties = ep
            .enumerate_instance_layer_properties()
            .map_err(to_vulkan)?;

        for required_layer in required_layers {
            let found_layer = layer_properties.iter().find(|layer_prop| {
                let layer_name = cchar_to_string(&layer_prop.layerName);
                layer_name == *required_layer
            });

            match found_layer {
                None => {
                    return Err(Error::Other(format!(
                        "cannot find layer: {}",
                        required_layer
                    )));
                }
                Some(layer) => {
                    let layer_name = cchar_to_string(&layer.layerName);
                    let version = VulkanVersion::from_compact(layer.specVersion);

                    info!("found layer: {}@{}", layer_name, version);
                }
            }
        }

        Ok(())
    }

    fn check_required_extensions(
        ep: &EntryPoints,
        required_extensions: &Vec<String>,
    ) -> Result<()> {
        let extension_properties = ep
            .enumerate_instance_extension_properties()
            .map_err(to_vulkan)?;
        for required_extension in required_extensions {
            let found_extension = extension_properties.iter().find(|extension_property| {
                let extension_name = cchar_to_string(&extension_property.extensionName);
                extension_name == *required_extension
            });

            match found_extension {
                None => {
                    return Err(Error::Other(format!(
                        "cannot find extension: {}",
                        required_extension
                    )));
                }
                Some(ext) => {
                    let extension_name = cchar_to_string(&ext.extensionName);
                    let version = VulkanVersion::from_compact(ext.specVersion);

                    info!("found extensions: {}@{}", extension_name, version);
                }
            }
        }

        Ok(())
    }

    fn create_debugger_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT {
            sType: vk::STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            flags: 0,
            messageSeverity: vk::DEBUG_UTILS_MESSAGE_SEVERITY_VERBOSE_BIT_EXT
                | vk::DEBUG_UTILS_MESSAGE_SEVERITY_INFO_BIT_EXT
                | vk::DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT
                | vk::DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT,
            messageType: vk::DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT
                | vk::DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT
                | vk::DEBUG_UTILS_MESSAGE_TYPE_PERFORMANCE_BIT_EXT,
            pfnUserCallback: Self::debugger_callback,
            pUserData: ptr::null_mut(),
            pNext: ptr::null(),
        }
    }

    extern "system" fn debugger_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagBitsEXT,
        _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _user_data: *mut c_void,
    ) -> vk::Bool32 {
        unsafe {
            let c_msg = std::ffi::CStr::from_ptr((*callback_data).pMessage);

            match c_msg.to_str() {
                Ok(s) => {
                    let log_level = if message_severity
                        & vk::DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT
                        != 0
                    {
                        Level::Error
                    } else if message_severity & vk::DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT
                        != 0
                    {
                        Level::Warn
                    } else if message_severity & vk::DEBUG_UTILS_MESSAGE_SEVERITY_INFO_BIT_EXT != 0
                    {
                        Level::Info
                    } else if message_severity & vk::DEBUG_UTILS_MESSAGE_SEVERITY_VERBOSE_BIT_EXT
                        != 0
                    {
                        Level::Debug
                    } else {
                        Level::Trace
                    };

                    log!(target: "vulkan", log_level, "vulkan | {}", s);
                }
                Err(_) => {
                    error!(target: "vulkan", "vulkan | debug utils cannot read message: {:?}", c_msg);
                }
            };
        };

        vk::FALSE
    }

    fn create_debug_messenger(
        ip: &InstancePointers,
        instance: vk::Instance,
    ) -> Result<vk::DebugUtilsMessengerEXT> {
        let create_info = Self::create_debugger_info();

        unsafe { ip.create_debug_utils_messenger_ext(instance, &create_info) }.map_err(to_vulkan)
    }

    fn create_surface(window: &glfw::Window, instance: vk::Instance) -> Result<vk::SurfaceKHR> {
        let mut surface = mem::MaybeUninit::<vk::SurfaceKHR>::uninit();
        let result = window.create_window_surface(instance, std::ptr::null(), surface.as_mut_ptr());
        maybe_vulkan_error(result)?;

        Ok(unsafe { surface.assume_init() })
    }

    fn find_physical_device(
        ip: &InstancePointers,
        instance: vk::Instance,
        required_device_extensions: &Vec<String>,
    ) -> Result<vk::PhysicalDevice> {
        let physical_devices = ip.enumerate_physical_devices(instance).map_err(to_vulkan)?;

        let maybe_good_physical_device = {
            let mut physical_devices_it = physical_devices.iter();
            loop {
                match physical_devices_it.next() {
                    Some(physical_device) => {
                        let properties = ip.get_physical_device_properties(*physical_device);
                        let name = cchar_to_string(&properties.deviceName);
                        info!("found physical device {}", name);

                        if properties.deviceType & vk::PHYSICAL_DEVICE_TYPE_DISCRETE_GPU != 0
                            && Self::check_physical_device_extensions(
                                ip,
                                *physical_device,
                                required_device_extensions,
                            )?
                        {
                            info!("found device and will use {}", name);
                            break Some(*physical_device);
                        }

                        info!("found device {}", name);
                    }
                    None => {
                        break None;
                    }
                }
            }
        };

        maybe_good_physical_device
            .ok_or_else(|| to_other(Error::Other("no discrete GPU found".to_owned())))
    }

    fn check_physical_device_extensions(
        ip: &InstancePointers,
        physical_device: vk::PhysicalDevice,
        req_dev_exts: &Vec<String>,
    ) -> Result<bool> {
        let props = ip
            .enumerate_device_extension_properties::<&str>(physical_device, None)
            .map_err(to_vulkan)?;

        let mut required_device_extensions: HashSet<&String> = req_dev_exts.iter().collect();

        for prop in &props {
            let ext_name = cchar_to_string(&prop.extensionName);
            info!("found device extension {}", ext_name);
            required_device_extensions.remove(&ext_name);
        }

        Ok(required_device_extensions.is_empty())
    }

    fn find_queue_families(
        ip: &InstancePointers,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Result<QueueFamilyIndices> {
        let props = ip.get_physical_device_queue_family_properties(physical_device);

        let graphics = props
            .iter()
            .enumerate()
            .find(|(_, prop)| prop.queueFlags & vk::QUEUE_GRAPHICS_BIT != 0)
            .map(|(index, _)| index as u32)
            .ok_or_else(|| Error::Other("graphics queue needed".to_owned()))?;

        let present = props
            .iter()
            .enumerate()
            .find(|(index, _)| {
                ip.get_physical_device_surface_support_khr(physical_device, *index as u32, surface)
                    .unwrap_or(false)
            })
            .map(|(index, _)| index as u32)
            .ok_or_else(|| Error::Other("present queue needed".to_owned()))?;

        Ok(QueueFamilyIndices { graphics, present })
    }

    fn create_device(
        ip: &InstancePointers,
        physical_device: vk::PhysicalDevice,
        queue_family_indices: &QueueFamilyIndices,
        required_device_extensions: &Vec<String>,
    ) -> Result<vk::Device> {
        let queue_priorities = [1f32];

        // There may be queues, which are graphics and present as well.
        // Vulkan does not allow to create multiple queues for the same index
        // so we need to dedupe them.
        let unique_queue_indices: HashSet<u32> =
            vec![queue_family_indices.graphics, queue_family_indices.present]
                .drain(..)
                .collect();

        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_queue_indices
            .into_iter()
            .map(|queue_index| vk::DeviceQueueCreateInfo {
                sType: vk::STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                queueFamilyIndex: queue_index as u32,
                queueCount: 1,
                pQueuePriorities: queue_priorities.as_ptr(),
            })
            .collect();

        let enabled_features: vk::PhysicalDeviceFeatures = unsafe { mem::zeroed() };
        let req_dev_exts = CStrings::new(&required_device_extensions).map_err(to_other)?;

        let create_info = vk::DeviceCreateInfo {
            sType: vk::STRUCTURE_TYPE_DEVICE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            queueCreateInfoCount: queue_create_infos.len() as u32,
            pQueueCreateInfos: queue_create_infos.as_ptr(),
            enabledLayerCount: 0,
            ppEnabledLayerNames: std::ptr::null(),
            enabledExtensionCount: req_dev_exts.len() as u32,
            ppEnabledExtensionNames: req_dev_exts.as_ptr(),
            pEnabledFeatures: &enabled_features,
        };

        unsafe { ip.create_device(physical_device, &create_info) }.map_err(to_vulkan)
    }

    fn get_device_queue_families(
        dp: &DevicePointers,
        device: vk::Device,
        queue_family_indices: &QueueFamilyIndices,
    ) -> QueueFamilies {
        QueueFamilies {
            graphics_queue: dp.get_device_queue(device, queue_family_indices.graphics, 0),
            present_queue: dp.get_device_queue(device, queue_family_indices.present, 0),
        }
    }

    fn create_command_pool(
        dp: &DevicePointers,
        device: vk::Device,
        queue_family_indices: &QueueFamilyIndices,
    ) -> Result<vk::CommandPool> {
        let info = vk::CommandPoolCreateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            queueFamilyIndex: queue_family_indices.graphics,
        };

        unsafe { dp.create_command_pool(device, &info) }.map_err(to_vulkan)
    }
}
