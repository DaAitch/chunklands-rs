use super::error::to_vulkan;
use super::error::{to_other, Error, Result};
use super::util::cchar_to_string;
use log::info;
use std::collections::HashSet;
use vk_sys as vk;
use vulkanic::InstancePointers;

pub fn find_physical_device(
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
                        && check_device_extensions(
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

fn check_device_extensions(
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
