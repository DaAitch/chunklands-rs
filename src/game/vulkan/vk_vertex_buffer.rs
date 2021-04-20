use glm::{Vec2, Vec3};
use vk_sys as vk;
use vulkanic::{DevicePointers, InstancePointers};
use std::{mem::size_of, ptr};
use super::{error::{Result, to_other, to_vulkan}, vertex::Vertex};


pub fn create_vertex_buffer(ip: &InstancePointers, dp: &DevicePointers, physical_device: vk::PhysicalDevice, device: vk::Device) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let vertices = [
        Vertex {
            pos: Vec2::new(0.0, -0.5),
            color: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: Vec2::new(0.5, 0.5),
            color: Vec3::new(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: Vec2::new(-0.5, 0.5),
            color: Vec3::new(0.0, 0.0, 1.0),
        },
    ];

    let buffer_info = vk::BufferCreateInfo {
        sType: vk::STRUCTURE_TYPE_BUFFER_CREATE_INFO,
        pNext: ptr::null(),
        flags: 0,
        size: (size_of::<Vertex>() * vertices.len()) as u64,
        usage: vk::BUFFER_USAGE_VERTEX_BUFFER_BIT,
        sharingMode: vk::SHARING_MODE_EXCLUSIVE,
        queueFamilyIndexCount: 0,
        pQueueFamilyIndices: ptr::null(),
    };

    let buffer = unsafe { dp.create_buffer(device, &buffer_info) }.map_err(to_vulkan)?;

    let memory_requirements = dp.get_buffer_memory_requirements(device, buffer);

    let allocate_info = vk::MemoryAllocateInfo {
        sType: vk::STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
        pNext: ptr::null(),
        allocationSize: memory_requirements.size,
        memoryTypeIndex: find_memory_type(ip, physical_device, memory_requirements.memoryTypeBits, vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT | vk::MEMORY_PROPERTY_HOST_COHERENT_BIT)?,
    };

    let device_memory = unsafe { dp.allocate_memory(device, &allocate_info) }.map_err(to_vulkan)?;

    dp.bind_buffer_memory(device, buffer, device_memory, 0).map_err(to_vulkan)?;

    let data = dp.map_memory(device, device_memory, 0, buffer_info.size, 0).map_err(to_vulkan)?;
    unsafe { std::ptr::copy_nonoverlapping(vertices.as_ptr(), data as *mut Vertex, buffer_info.size as usize) };
    dp.unmap_memory(device, device_memory);

    Ok((buffer, device_memory))
}

fn find_memory_type(ip: &InstancePointers, physical_device: vk::PhysicalDevice, type_filter: u32, flags: vk::MemoryPropertyFlags) -> Result<u32> {
    let properties = ip.get_physical_device_memory_properties(physical_device);
    for i in 0..properties.memoryTypeCount {
        if (type_filter & (1 << i)) != 0 && (properties.memoryTypes[i as usize].propertyFlags & flags) != 0 {
            return Ok(i);
        }
    }

    Err(to_other("could not find memory type"))
}