use super::error::{to_vulkan, Result};
use super::util::CStrings;
use log::{error, info, log, Level};
use vulkanic::{EntryPoints, InstancePointers};

use std::{
    ffi::{c_void, CString},
    ptr,
};
use vk_sys as vk;

use super::{error::Error, util::cchar_to_string, version::VulkanVersion};

pub fn create_instance(
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
        check_required_layers(ep, &req_dbg_layers)?;

        let mut req_dbg_ext = required_extensions.clone();
        req_dbg_ext.push("VK_EXT_debug_utils".to_owned());
        check_required_extensions(ep, &req_dbg_ext)?;

        (
            CStrings::new(&req_dbg_layers).unwrap(),
            CStrings::new(&req_dbg_ext).unwrap(),
        ) // TODO unwrap
    } else {
        check_required_extensions(ep, &required_extensions)?;

        (
            CStrings::new(&Vec::<String>::new()).unwrap(),
            CStrings::new(&required_extensions).unwrap(),
        ) // TODO unwrap
    };

    let mut debug_info = create_debugger_info();

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

fn check_required_extensions(ep: &EntryPoints, required_extensions: &Vec<String>) -> Result<()> {
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
        pfnUserCallback: debugger_callback,
        pUserData: ptr::null_mut(),
        pNext: ptr::null_mut(),
    }
}

pub fn create_debug_messenger(
    ip: &InstancePointers,
    instance: vk::Instance,
) -> Result<vk::DebugUtilsMessengerEXT> {
    let create_info = create_debugger_info();

    unsafe { ip.create_debug_utils_messenger_ext(instance, &create_info) }.map_err(to_vulkan)
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
                let log_level = if message_severity & vk::DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT
                    != 0
                {
                    Level::Error
                } else if message_severity & vk::DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT != 0 {
                    Level::Warn
                } else if message_severity & vk::DEBUG_UTILS_MESSAGE_SEVERITY_INFO_BIT_EXT != 0 {
                    Level::Info
                } else if message_severity & vk::DEBUG_UTILS_MESSAGE_SEVERITY_VERBOSE_BIT_EXT != 0 {
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
