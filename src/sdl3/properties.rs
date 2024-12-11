use libc::c_char;
use libc::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::NulError;
use std::ptr;
use std::str::Utf8Error;
use sys::properties::SDL_PropertiesID;

use crate::get_error;
use crate::sys;

#[derive(Debug)]
pub enum PropertiesError {
    NameError(NulError),
    TypeError(NulError),
    DecodeError(Utf8Error),
    MissingKey,
    SdlError(String),
}

#[derive(Debug, Clone)]
pub struct Properties {
    internal: sys::properties::SDL_PropertiesID,
}

// Ideally this could be replaced by TryInto<CString> parameters (see: https://github.com/rust-lang/rust/issues/71448)
macro_rules! cstring {
    ($name:ident) => {
        let $name = match CString::new($name) {
            Ok(name) => name,
            Err(error) => return Err(PropertiesError::NameError(error)),
        };
    };
}

pub type EnumerateCallback = Box<dyn Fn(&Properties, Result<&str, PropertiesError>)>;
unsafe extern "C" fn enumerate(
    userdata: *mut c_void,
    props: SDL_PropertiesID,
    name: *const c_char,
) {
    let properties: &Properties = std::mem::transmute(&props);
    let callback_ptr = userdata as *mut EnumerateCallback;
    let name = CStr::from_ptr(name);
    match name.to_str() {
        Ok(name) => (*callback_ptr)(properties, Ok(name)),
        Err(error) => (*callback_ptr)(properties, Err(PropertiesError::DecodeError(error))),
    }
}

pub type CleanupCallback = fn(*mut c_void);
unsafe extern "C" fn cleanup_box(userdata: *mut c_void, value: *mut c_void) {
    let callback_ptr = userdata as *mut CleanupCallback;
    (*callback_ptr)(value);
}

pub use sys::properties::SDL_PropertyType as PropertyType;

impl Properties {
    #[doc(alias = "SDL_CreateProperties")]
    pub fn new() -> Result<Self, PropertiesError> {
        let internal = unsafe { sys::properties::SDL_CreateProperties() };
        if internal == 0 {
            Err(PropertiesError::SdlError(get_error()))
        } else {
            Ok(Self { internal })
        }
    }

    #[doc(alias = "SDL_GetGlobalProperties")]
    pub fn global() -> Result<Self, PropertiesError> {
        let internal = unsafe { sys::properties::SDL_GetGlobalProperties() };
        if internal == 0 {
            Err(PropertiesError::SdlError(get_error()))
        } else {
            Ok(Self { internal })
        }
    }

    #[doc(alias = "SDL_LockProperties")]
    pub fn lock(&mut self) -> Result<(), PropertiesError> {
        unsafe {
            if !sys::properties::SDL_LockProperties(self.internal) {
                return Err(PropertiesError::SdlError(get_error()));
            }
        }
        Ok(())
    }

    #[doc(alias = "SDL_UnlockProperties")]
    pub fn unlock(&mut self) {
        unsafe {
            sys::properties::SDL_UnlockProperties(self.internal);
        }
    }

    #[doc(alias = "SDL_HasProperty")]
    pub fn contains(&self, name: &str) -> Result<bool, PropertiesError> {
        cstring!(name);
        unsafe {
            Ok(sys::properties::SDL_HasProperty(
                self.internal,
                name.as_ptr(),
            ))
        }
    }

    #[doc(alias = "SDL_GetPropertyType")]
    pub fn get_type(&self, name: &str) -> Result<PropertyType, PropertiesError> {
        cstring!(name);
        unsafe {
            Ok(sys::properties::SDL_GetPropertyType(
                self.internal,
                name.as_ptr(),
            ))
        }
    }

    #[doc(alias = "SDL_CopyProperties")]
    pub fn copy(&self, destination: &mut Self) -> Result<(), PropertiesError> {
        if unsafe { sys::properties::SDL_CopyProperties(self.internal, destination.internal) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }

    #[doc(alias = "SDL_EnumerateProperties")]
    pub fn enumerate(&self, callback: EnumerateCallback) -> Result<(), PropertiesError> {
        let callback_ptr = Box::into_raw(Box::new(callback)) as *mut c_void;
        if unsafe {
            sys::properties::SDL_EnumerateProperties(self.internal, Some(enumerate), callback_ptr)
        } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }

    #[doc(alias = "SDL_ClearProperty")]
    pub fn clear(&mut self, name: &str) -> Result<(), PropertiesError> {
        cstring!(name);
        if unsafe { sys::properties::SDL_ClearProperty(self.internal, name.as_ptr()) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }

    #[doc(alias = "SDL_GetPointerProperty")]
    pub fn with<T>(&mut self, name: &str, with: fn(&T)) -> Result<(), PropertiesError> {
        self.lock()?;
        let pointer = self.get(name, None::<*mut T>)?;
        let reference = unsafe { &mut *pointer };
        with(reference);
        self.unlock();
        Ok(())
    }
}

pub trait PropertySetter<T> {
    fn set(&self, name: &str, value: T) -> Result<(), PropertiesError>;
}

impl PropertySetter<bool> for Properties {
    #[doc(alias = "SDL_SetBooleanProperty")]
    fn set(&self, name: &str, value: bool) -> Result<(), PropertiesError> {
        cstring!(name);
        if unsafe { sys::properties::SDL_SetBooleanProperty(self.internal, name.as_ptr(), value) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

impl PropertySetter<f32> for Properties {
    #[doc(alias = "SDL_SetFloatProperty")]
    fn set(&self, name: &str, value: f32) -> Result<(), PropertiesError> {
        cstring!(name);
        if unsafe { sys::properties::SDL_SetFloatProperty(self.internal, name.as_ptr(), value) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

impl PropertySetter<i64> for Properties {
    #[doc(alias = "SDL_SetNumberProperty")]
    fn set(&self, name: &str, value: i64) -> Result<(), PropertiesError> {
        cstring!(name);
        if unsafe { sys::properties::SDL_SetNumberProperty(self.internal, name.as_ptr(), value) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

impl PropertySetter<&str> for Properties {
    #[doc(alias = "SDL_SetStringProperty")]
    fn set(&self, name: &str, value: &str) -> Result<(), PropertiesError> {
        cstring!(name);
        // Have to transform the value into a cstring, SDL makes an internal copy
        cstring!(value);
        if unsafe {
            sys::properties::SDL_SetStringProperty(self.internal, name.as_ptr(), value.as_ptr())
        } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

impl<T> PropertySetter<*mut T> for Properties {
    #[doc(alias = "SDL_SetPointerProperty")]
    fn set(&self, name: &str, value: *mut T) -> Result<(), PropertiesError> {
        cstring!(name);
        if unsafe {
            sys::properties::SDL_SetPointerProperty(
                self.internal,
                name.as_ptr(),
                value as *mut c_void,
            )
        } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

impl<T> PropertySetter<Box<T>> for Properties {
    #[doc(alias = "SDL_SetPointerPropertyWithCleanup")]
    fn set(&self, name: &str, value: Box<T>) -> Result<(), PropertiesError> {
        cstring!(name);
        let value_ptr = Box::into_raw(value) as *mut c_void;
        let cleanup: CleanupCallback = |value: *mut c_void| {
            let value = value as *mut T;
            unsafe {
                drop(Box::from_raw(value));
            }
        };
        let cleanup_ptr = Box::into_raw(Box::new(cleanup)) as *mut c_void;
        if unsafe {
            sys::properties::SDL_SetPointerPropertyWithCleanup(
                self.internal,
                name.as_ptr(),
                value_ptr,
                Some(cleanup_box),
                cleanup_ptr,
            )
        } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

pub trait PropertyGetter<T> {
    fn get(&self, name: &str, default: Option<T>) -> Result<T, PropertiesError>;
}

/// If the consumer passes no default value, error out if the key does not exist
/// newdefault is just a dummy value to pass to the function
macro_rules! nodefault {
    ($self:ident, $name:ident, $default:ident, $newdefault:expr) => {
        let $default = if $default.is_none() {
            if !sys::properties::SDL_HasProperty($self.internal, $name.as_ptr()) {
                return Err(PropertiesError::MissingKey);
            } else {
                $newdefault
            }
        } else {
            $default.unwrap()
        };
    };
}

impl PropertyGetter<bool> for Properties {
    #[doc(alias = "SDL_GetBooleanProperty")]
    fn get(&self, name: &str, default: Option<bool>) -> Result<bool, PropertiesError> {
        cstring!(name);
        unsafe {
            nodefault!(self, name, default, false);
            Ok(sys::properties::SDL_GetBooleanProperty(
                self.internal,
                name.as_ptr(),
                default,
            ))
        }
    }
}

impl PropertyGetter<f32> for Properties {
    #[doc(alias = "SDL_GetFloatProperty")]
    fn get(&self, name: &str, default: Option<f32>) -> Result<f32, PropertiesError> {
        cstring!(name);
        unsafe {
            nodefault!(self, name, default, 0.0);
            Ok(sys::properties::SDL_GetFloatProperty(
                self.internal,
                name.as_ptr(),
                default,
            ))
        }
    }
}

impl PropertyGetter<i64> for Properties {
    #[doc(alias = "SDL_GetNumberProperty")]
    fn get(&self, name: &str, default: Option<i64>) -> Result<i64, PropertiesError> {
        cstring!(name);
        unsafe {
            nodefault!(self, name, default, 0);
            Ok(sys::properties::SDL_GetNumberProperty(
                self.internal,
                name.as_ptr(),
                default,
            ))
        }
    }
}

impl PropertyGetter<String> for Properties {
    #[doc(alias = "SDL_GetStringProperty")]
    fn get(&self, name: &str, default: Option<String>) -> Result<String, PropertiesError> {
        cstring!(name);
        let default_ptr = if default.is_none() {
            unsafe {
                if !sys::properties::SDL_HasProperty(self.internal, name.as_ptr()) {
                    return Err(PropertiesError::MissingKey);
                } else {
                    ptr::null()
                }
            }
        } else {
            // This is not evaluated by SDL, only the value is returned
            default.as_ref().unwrap().as_ptr() as *const i8
        };
        let value = unsafe {
            sys::properties::SDL_GetStringProperty(self.internal, name.as_ptr(), default_ptr)
        };

        // Default value, just return our default
        let value = if value == default_ptr {
            return Ok(default.unwrap());
        } else {
            unsafe { CStr::from_ptr(value) }
        };

        match value.to_str() {
            Ok(value) => Ok(String::from(value)),
            Err(error) => Err(PropertiesError::DecodeError(error)),
        }
    }
}

impl<T> PropertyGetter<*mut T> for Properties {
    #[doc(alias = "SDL_GetPointerProperty")]
    fn get(&self, name: &str, default: Option<*mut T>) -> Result<*mut T, PropertiesError> {
        cstring!(name);
        let default = if default.is_none() {
            unsafe {
                if !sys::properties::SDL_HasProperty(self.internal, name.as_ptr()) {
                    return Err(PropertiesError::MissingKey);
                } else {
                    ptr::null_mut()
                }
            }
        } else {
            default.unwrap() as *mut c_void
        };
        let pointer = unsafe {
            sys::properties::SDL_GetPointerProperty(self.internal, name.as_ptr(), default)
        };
        Ok(pointer as *mut T)
    }
}

impl Drop for Properties {
    fn drop(&mut self) {
        unsafe {
            sys::properties::SDL_DestroyProperties(self.internal);
        }
    }
}
