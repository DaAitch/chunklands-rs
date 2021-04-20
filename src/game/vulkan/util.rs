use std::ffi::{CString, NulError};
use vk_sys as vk;

pub struct CStrings {
    cstrings: Vec<CString>,
    cchars: Vec<*const i8>,
}

impl CStrings {
    pub fn new(strings: &Vec<String>) -> Result<Self, NulError> {
        let len = strings.len();

        let mut cstrings = Vec::<CString>::with_capacity(len);
        let mut cchars = Vec::<*const i8>::with_capacity(len);

        for string in strings {
            let cstring = CString::new(string.clone())?;
            cchars.push(cstring.as_ptr());
            cstrings.push(cstring);
        }

        Ok(CStrings { cstrings, cchars })
    }

    pub fn as_ptr(&self) -> *const *const i8 {
        self.cchars.as_ptr()
    }

    pub fn len(&self) -> usize {
        self.cstrings.len()
    }
}

pub fn cchar_to_string(c: &[i8]) -> String {
    c.iter()
        .take_while(|x| (**x) != 0i8)
        .map(|x| (*x) as u8 as char)
        .collect()
}

macro_rules! impl_copy {
    ($t:ty, $fn_name:ident) => {
        pub fn $fn_name(data: &$t) -> $t {
            unsafe { std::mem::transmute_copy(data) }
        }
    };
}

impl_copy!(vk::Extent2D, copy_extent_2d);
impl_copy!(vk::SurfaceFormatKHR, copy_surface_format_khr);
