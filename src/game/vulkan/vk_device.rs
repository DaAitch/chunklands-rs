use super::error::{to_other, to_vulkan, Result};
use super::util::CStrings;
use super::vk_queue::QueueFamilyIndices;
use std::{collections::HashSet, mem};
use vk_sys as vk;
use vulkanic::InstancePointers;

pub fn create_device(
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
