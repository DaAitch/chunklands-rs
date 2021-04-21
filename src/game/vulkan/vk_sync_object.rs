use super::error::{to_vulkan, Result};
use vk_sys as vk;
use vulkanic::DevicePointers;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub fn create_sync_objects(
    dp: &DevicePointers,
    device: vk::Device,
) -> Result<Vec<(vk::Semaphore, vk::Semaphore, vk::Fence)>> {
    let semaphore_info = vk::SemaphoreCreateInfo {
        sType: vk::STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
    };

    let fence_info = vk::FenceCreateInfo {
        sType: vk::STRUCTURE_TYPE_FENCE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: vk::FENCE_CREATE_SIGNALED_BIT,
    };

    let mut sync_objects =
        Vec::<(vk::Semaphore, vk::Semaphore, vk::Fence)>::with_capacity(MAX_FRAMES_IN_FLIGHT);

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        let image_available_sem =
            unsafe { dp.create_semaphore(device, &semaphore_info) }.map_err(to_vulkan)?;
        let render_finished_sem =
            unsafe { dp.create_semaphore(device, &semaphore_info) }.map_err(to_vulkan)?;
        let in_flight_fence = unsafe { dp.create_fence(device, &fence_info) }.map_err(to_vulkan)?;

        sync_objects.push((image_available_sem, render_finished_sem, in_flight_fence));
    }

    Ok(sync_objects)
}
