use std::ffi::{CString, NulError};

use libc::c_char;

pub fn option_to_ptr<T>(opt: Option<&T>) -> *const T {
    opt.map_or(std::ptr::null(), |v| v as *const _)
}

// Offer a flexible parameter taking String and &str but allowing CString for optimization
pub struct StringParam {
    internal: CString,
}

impl StringParam {
    pub fn as_ptr(&self) -> *const c_char {
        self.internal.as_ptr()
    }
}

impl TryFrom<&str> for StringParam {
    type Error = NulError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self {
            internal: CString::new(value)?,
        })
    }
}

impl TryFrom<String> for StringParam {
    type Error = NulError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self {
            internal: CString::new(value.as_str())?,
        })
    }
}

impl TryFrom<CString> for StringParam {
    type Error = NulError;
    fn try_from(value: CString) -> Result<Self, Self::Error> {
        Ok(Self { internal: value })
    }
}
