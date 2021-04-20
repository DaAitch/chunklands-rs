use super::{Error, Result};
use vk_sys as vk;
use vulkanic::{DevicePointers, InstancePointers};

#[derive(Debug)]
pub struct QueueFamilies {
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

pub fn get_device_queue_families(
    dp: &DevicePointers,
    device: vk::Device,
    queue_family_indices: &QueueFamilyIndices,
) -> QueueFamilies {
    QueueFamilies {
        graphics_queue: dp.get_device_queue(device, queue_family_indices.graphics, 0),
        present_queue: dp.get_device_queue(device, queue_family_indices.present, 0),
    }
}

#[derive(Debug)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
    pub present: u32,
}

pub fn find_queue_families(
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
