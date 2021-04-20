use std::fmt;

const VERSION_MAJOR_SHIFT: u32 = 22;
const VERSION_MINOR_SHIFT: u32 = 12;
const VERSION_MAJOR_MASK: u32 = 0b1111111111_0000000000_000000000000;
const VERSION_MINOR_MASK: u32 = 0b0000000000_1111111111_000000000000;
const VERSION_PATCH_MASK: u32 = 0b0000000000_0000000000_111111111111;

#[derive(Debug)]
pub struct VulkanVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl VulkanVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn from_compact(compact: u32) -> Self {
        let version = get_version(compact);
        Self {
            major: version.0,
            minor: version.1,
            patch: version.2,
        }
    }

    pub fn get_compact(&self) -> u32 {
        get_compact_version((self.major, self.minor, self.patch))
    }
}

impl fmt::Display for VulkanVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn get_compact_version(version: (u32, u32, u32)) -> u32 {
    (version.0 << VERSION_MAJOR_SHIFT) | (version.1 << VERSION_MINOR_SHIFT) | version.2
}

fn get_version(compact: u32) -> (u32, u32, u32) {
    (
        (compact & VERSION_MAJOR_MASK) >> VERSION_MAJOR_SHIFT,
        (compact & VERSION_MINOR_MASK) >> VERSION_MINOR_SHIFT,
        compact & VERSION_PATCH_MASK,
    )
}
