use libc::c_char;
use libc::c_void;
use std::ffi::CStr;
use std::ffi::NulError;
use std::ptr;
use std::str::Utf8Error;
use sys::properties::SDL_PropertiesID;

use crate::get_error;
use crate::sys;
use crate::util::StringParam;

#[derive(Debug)]
pub enum PropertiesError {
    ParamError(NulError),
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

// Wrap try_into conversion/errors for StringParam
macro_rules! stringparam {
    ($name:ident) => {
        let $name = match $name.try_into() {
            Ok(name) => name,
            Err(error) => return Err(PropertiesError::ParamError(error)),
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
    pub fn contains<S>(&self, name: S) -> Result<bool, PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
        unsafe {
            Ok(sys::properties::SDL_HasProperty(
                self.internal,
                name.as_ptr(),
            ))
        }
    }

    #[doc(alias = "SDL_GetPropertyType")]
    pub fn get_type<S>(&self, name: S) -> Result<PropertyType, PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
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
    pub fn clear<S>(&mut self, name: S) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
        if unsafe { sys::properties::SDL_ClearProperty(self.internal, name.as_ptr()) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }

    #[doc(alias = "SDL_GetPointerProperty")]
    pub fn with<S, T>(&mut self, name: S, with: fn(&T)) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        self.lock()?;
        let pointer = self.get(name, None::<*mut T>)?;
        let reference = unsafe { &mut *pointer };
        with(reference);
        self.unlock();
        Ok(())
    }
}

pub trait PropertySetter<T> {
    fn set<S>(&self, name: S, value: T) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>;
}

impl PropertySetter<bool> for Properties {
    #[doc(alias = "SDL_SetBooleanProperty")]
    fn set<S>(&self, name: S, value: bool) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
        if unsafe { sys::properties::SDL_SetBooleanProperty(self.internal, name.as_ptr(), value) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

impl PropertySetter<f32> for Properties {
    #[doc(alias = "SDL_SetFloatProperty")]
    fn set<S>(&self, name: S, value: f32) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
        if unsafe { sys::properties::SDL_SetFloatProperty(self.internal, name.as_ptr(), value) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

impl PropertySetter<i64> for Properties {
    #[doc(alias = "SDL_SetNumberProperty")]
    fn set<S>(&self, name: S, value: i64) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
        if unsafe { sys::properties::SDL_SetNumberProperty(self.internal, name.as_ptr(), value) } {
            Ok(())
        } else {
            Err(PropertiesError::SdlError(get_error()))
        }
    }
}

impl PropertySetter<&str> for Properties {
    #[doc(alias = "SDL_SetStringProperty")]
    fn set<S>(&self, name: S, value: &str) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
        // Have to transform the value into a cstring, SDL makes an internal copy
        let value: StringParam = match value.try_into() {
            Ok(value) => value,
            Err(error) => return Err(PropertiesError::ParamError(error)),
        };

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
    fn set<S>(&self, name: S, value: *mut T) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
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
    fn set<S>(&self, name: S, value: Box<T>) -> Result<(), PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
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
    fn get<S>(&self, name: S, default: Option<T>) -> Result<T, PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>;
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
    fn get<S>(&self, name: S, default: Option<bool>) -> Result<bool, PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
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
    fn get<S>(&self, name: S, default: Option<f32>) -> Result<f32, PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
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
    fn get<S>(&self, name: S, default: Option<i64>) -> Result<i64, PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
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
    fn get<S>(&self, name: S, default: Option<String>) -> Result<String, PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
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
    fn get<S>(&self, name: S, default: Option<*mut T>) -> Result<*mut T, PropertiesError>
    where
        S: TryInto<StringParam, Error = NulError>,
    {
        stringparam!(name);
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
