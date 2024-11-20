use core_foundation::{
    base::{kCFAllocatorDefault, kCFAllocatorNull, CFRelease, CFTypeRef},
    dictionary::{CFDictionaryGetValue, CFDictionaryRef},
    string::{
        kCFStringEncodingUTF8, CFStringCreateWithBytesNoCopy, CFStringGetCString, CFStringRef,
    },
};

pub fn cfdict_get_val(dict: CFDictionaryRef, key: &str) -> Option<CFTypeRef> {
    unsafe {
        let key = cfstr(key);
        let val = CFDictionaryGetValue(dict, key as _);
        CFRelease(key as _);

        match val {
            _ if val.is_null() => None,
            _ => Some(val),
        }
    }
}

pub fn cfstr(val: &str) -> CFStringRef {
    // this creates broken objects if string len > 9
    // CFString::from_static_string(val).as_concrete_TypeRef()
    // CFString::new(val).as_concrete_TypeRef()

    unsafe {
        CFStringCreateWithBytesNoCopy(
            kCFAllocatorDefault,
            val.as_ptr(),
            val.len() as isize,
            kCFStringEncodingUTF8,
            0,
            kCFAllocatorNull,
        )
    }
}

pub fn from_cfstr(val: CFStringRef) -> String {
    unsafe {
        let mut buf = Vec::with_capacity(256); //128 here seems dumb
        if CFStringGetCString(val, buf.as_mut_ptr(), 256, kCFStringEncodingUTF8) == 0 {
            return String::new();
        }
        std::ffi::CStr::from_ptr(buf.as_ptr())
            .to_string_lossy()
            .to_string()
    }
}

pub fn get_cf_string<F>(getter: F) -> String
where
    F: FnOnce() -> CFStringRef,
{
    match getter() {
        x if x.is_null() => String::new(),
        x => from_cfstr(x),
    }
}
