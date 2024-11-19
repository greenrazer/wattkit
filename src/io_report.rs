use std::{
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
};

use crate::cf_utils::*;
use core_foundation::{
    array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef},
    base::{kCFAllocatorDefault, CFAllocatorRef, CFRelease, CFTypeRef},
    dictionary::{
        CFDictionaryCreateMutableCopy, CFDictionaryGetCount, CFDictionaryRef,
        CFMutableDictionaryRef,
    },
    string::CFStringRef,
};

pub type WithError<T> = Result<T, Box<dyn std::error::Error>>;
pub type CVoidRef = *const std::ffi::c_void;

#[repr(C)]
pub struct IOReportSubscription {
    _data: [u8; 0],
    _phantom: PhantomData<(*mut u8, PhantomPinned)>,
}

pub type IOReportSubscriptionRef = *const IOReportSubscription;

#[link(name = "IOKit", kind = "framework")]
#[rustfmt::skip]
extern "C" {
  pub fn IOServiceMatching(name: *const i8) -> CFMutableDictionaryRef;
  pub fn IOServiceGetMatchingServices(mainPort: u32, matching: CFDictionaryRef, existing: *mut u32) -> i32;
  pub fn IOIteratorNext(iterator: u32) -> u32;
  pub fn IORegistryEntryGetName(entry: u32, name: *mut i8) -> i32;
  pub fn IORegistryEntryCreateCFProperties(entry: u32, properties: *mut CFMutableDictionaryRef, allocator: CFAllocatorRef, options: u32) -> i32;
  pub fn IOObjectRelease(obj: u32) -> u32;
}

#[link(name = "IOReport", kind = "dylib")]
#[rustfmt::skip]
extern "C" {
  pub fn IOReportCopyAllChannels(a: u64, b: u64) -> CFMutableDictionaryRef;
  pub fn IOReportCopyChannelsInGroup(group: CFStringRef, subgroup: CFStringRef, c: u64, d: u64, e: u64) -> CFMutableDictionaryRef;
  pub fn IOReportMergeChannels(a: CFDictionaryRef, b: CFDictionaryRef, nil: CFTypeRef);
  pub fn IOReportCreateSubscription(a: CVoidRef, desired_channels: CFMutableDictionaryRef, subbed_channels: *mut CFMutableDictionaryRef, channel_id: u64, b: CFTypeRef) -> IOReportSubscriptionRef;
  pub fn IOReportCreateSamples(a: IOReportSubscriptionRef, b: CFMutableDictionaryRef, c: CFTypeRef) -> CFDictionaryRef;
  pub fn IOReportCreateSamplesDelta(a: CFDictionaryRef, b: CFDictionaryRef, c: CFTypeRef) -> CFDictionaryRef;
  pub fn IOReportChannelGetGroup(a: CFDictionaryRef) -> CFStringRef;
  pub fn IOReportChannelGetSubGroup(a: CFDictionaryRef) -> CFStringRef;
  pub fn IOReportChannelGetChannelName(a: CFDictionaryRef) -> CFStringRef;
  pub fn IOReportSimpleGetIntegerValue(a: CFDictionaryRef, b: i32) -> i64;
  pub fn IOReportChannelGetUnitLabel(a: CFDictionaryRef) -> CFStringRef;
  pub fn IOReportStateGetCount(a: CFDictionaryRef) -> i32;
  pub fn IOReportStateGetNameForIndex(a: CFDictionaryRef, b: i32) -> CFStringRef;
  pub fn IOReportStateGetResidency(a: CFDictionaryRef, b: i32) -> i64;
}

// const CPU_FREQ_DICE_SUBG: &str = "CPU Complex Performance States";
const CPU_FREQ_CORE_SUBG: &str = "CPU Core Performance States";
const GPU_FREQ_DICE_SUBG: &str = "GPU Performance States";

fn get_cf_string<F>(getter: F) -> String
where
    F: FnOnce() -> CFStringRef,
{
    match getter() {
        x if x.is_null() => String::new(),
        x => from_cfstr(x),
    }
}

pub fn cfio_get_group(item: CFDictionaryRef) -> String {
    get_cf_string(|| unsafe { IOReportChannelGetGroup(item) })
}

pub fn cfio_get_subgroup(item: CFDictionaryRef) -> String {
    get_cf_string(|| unsafe { IOReportChannelGetSubGroup(item) })
}

pub fn cfio_get_channel(item: CFDictionaryRef) -> String {
    get_cf_string(|| unsafe { IOReportChannelGetChannelName(item) })
}

pub fn cfio_get_props(entry: u32, name: String) -> WithError<CFDictionaryRef> {
    unsafe {
        let mut props: MaybeUninit<CFMutableDictionaryRef> = MaybeUninit::uninit();
        if IORegistryEntryCreateCFProperties(entry, props.as_mut_ptr(), kCFAllocatorDefault, 0) != 0
        {
            return Err(format!("Failed to get properties for {}", name).into());
        }

        Ok(props.assume_init())
    }
}

pub fn cfio_get_residencies(item: CFDictionaryRef) -> Vec<(String, i64)> {
    let count = unsafe { IOReportStateGetCount(item) };
    let mut res = vec![];

    for i in 0..count {
        let name = unsafe { IOReportStateGetNameForIndex(item, i) };
        let val = unsafe { IOReportStateGetResidency(item, i) };
        res.push((from_cfstr(name), val));
    }

    res
}

pub fn cfio_watts(item: CFDictionaryRef, unit: &String, duration: u64) -> WithError<f32> {
    let val = unsafe { IOReportSimpleGetIntegerValue(item, 0) } as f32;
    let val = val / (duration as f32 / 1000.0);
    match unit.as_str() {
        "mJ" => Ok(val / 1e3f32),
        "uJ" => Ok(val / 1e6f32),
        "nJ" => Ok(val / 1e9f32),
        _ => Err(format!("Invalid energy unit: {}", unit).into()),
    }
}

pub struct IOReportIterator {
    sample: CFDictionaryRef,
    index: isize,
    items: CFArrayRef,
    items_size: isize,
}

impl IOReportIterator {
    pub fn new(data: CFDictionaryRef) -> Self {
        let channels = cfdict_get_val(data, "IOReportChannels").unwrap() as CFArrayRef;
        let items_size = unsafe { CFArrayGetCount(channels) } as isize;
        Self {
            sample: data,
            items: channels,
            items_size,
            index: 0,
        }
    }
}

impl Drop for IOReportIterator {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.sample as _);
        }
    }
}

#[derive(Debug)]
pub struct IOReportIteratorItem {
    pub group: String,
    pub subgroup: String,
    pub channel: String,
    pub unit: String,
    pub item: CFDictionaryRef,
}

impl Iterator for IOReportIterator {
    type Item = IOReportIteratorItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.items_size {
            return None;
        }

        let item = unsafe { CFArrayGetValueAtIndex(self.items, self.index) } as CFDictionaryRef;

        let group = cfio_get_group(item);
        let subgroup = cfio_get_subgroup(item);
        let channel = cfio_get_channel(item);
        let unit = from_cfstr(unsafe { IOReportChannelGetUnitLabel(item) })
            .trim()
            .to_string();

        self.index += 1;
        Some(IOReportIteratorItem {
            group,
            subgroup,
            channel,
            unit,
            item,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IOReportError {
    #[error("Failed to get properties for {0}")]
    PropertyError(String),
    #[error("Failed to get channels")]
    ChannelError,
    #[error("Failed to create subscription")]
    SubscriptionError,
    #[error("Invalid energy unit: {0}")]
    InvalidEnergyUnit(String),
}

type Result<T> = std::result::Result<T, IOReportError>;

pub struct IOReport {
    subscription: IOReportSubscriptionRef,
    channels: CFMutableDictionaryRef,
    previous: Option<(CFDictionaryRef, std::time::Instant)>,
}

impl IOReport {
    pub fn new(channels: Vec<(&str, Option<&str>)>) -> WithError<Self> {
        let channels = Self::create_channels(channels)?;
        let subscription = Self::subscribe(channels)?;

        Ok(Self {
            subscription,
            channels,
            previous: None,
        })
    }

    fn subscribe(channel: CFMutableDictionaryRef) -> Result<IOReportSubscriptionRef> {
        let mut s: MaybeUninit<CFMutableDictionaryRef> = MaybeUninit::uninit();
        let rs = unsafe {
            IOReportCreateSubscription(
                std::ptr::null(),
                channel,
                s.as_mut_ptr(),
                0,
                std::ptr::null(),
            )
        };
        if rs.is_null() {
            return Err(IOReportError::SubscriptionError);
        }

        unsafe { s.assume_init() };
        Ok(rs)
    }

    fn create_channels(channel_reqs: Vec<(&str, Option<&str>)>) -> Result<CFMutableDictionaryRef> {
        // if no items are provided, return all channels
        if channel_reqs.is_empty() {
            unsafe {
                let c = IOReportCopyAllChannels(0, 0);
                let r =
                    CFDictionaryCreateMutableCopy(kCFAllocatorDefault, CFDictionaryGetCount(c), c);
                CFRelease(c as _);
                return Ok(r);
            }
        }

        let mut channels = Vec::with_capacity(channel_reqs.len());
        for (group, subgroup) in channel_reqs {
            let gname = cfstr(group);
            let sname = subgroup.map_or(std::ptr::null(), cfstr);
            let chan = unsafe { IOReportCopyChannelsInGroup(gname, sname, 0, 0, 0) };
            channels.push(chan);

            unsafe { CFRelease(gname as _) };
            if subgroup.is_some() {
                unsafe { CFRelease(sname as _) };
            }
        }

        let base_channel = channels[0];
        for channel in channels.iter().skip(1) {
            unsafe { IOReportMergeChannels(base_channel, *channel, std::ptr::null()) };
        }

        let size = unsafe { CFDictionaryGetCount(base_channel) };
        let chan =
            unsafe { CFDictionaryCreateMutableCopy(kCFAllocatorDefault, size, base_channel) };

        for channel in channels {
            unsafe { CFRelease(channel as _) };
        }

        if cfdict_get_val(chan, "IOReportChannels").is_none() {
            return Err(IOReportError::ChannelError);
        }

        Ok(chan)
    }

    pub fn get_sample(&self, duration: u64) -> IOReportIterator {
        unsafe {
            let sample1 = IOReportCreateSamples(self.subscription, self.channels, std::ptr::null());
            std::thread::sleep(std::time::Duration::from_millis(duration));
            let sample2 = IOReportCreateSamples(self.subscription, self.channels, std::ptr::null());

            let sample3 = IOReportCreateSamplesDelta(sample1, sample2, std::ptr::null());
            CFRelease(sample1 as _);
            CFRelease(sample2 as _);
            IOReportIterator::new(sample3)
        }
    }

    fn raw_sample(&self) -> (CFDictionaryRef, std::time::Instant) {
        (
            unsafe { IOReportCreateSamples(self.subscription, self.channels, std::ptr::null()) },
            std::time::Instant::now(),
        )
    }

    pub fn get_samples(&mut self, duration: u64, count: usize) -> Vec<(IOReportIterator, u64)> {
        let count = count.clamp(1, 32);
        let mut samples: Vec<(IOReportIterator, u64)> = Vec::with_capacity(count);
        let step_msec = duration / count as u64;

        let mut prev = match self.previous {
            Some(x) => x,
            None => self.raw_sample(),
        };

        for _ in 0..count {
            std::thread::sleep(std::time::Duration::from_millis(step_msec));

            let next = self.raw_sample();
            let diff = unsafe { IOReportCreateSamplesDelta(prev.0, next.0, std::ptr::null()) };
            unsafe { CFRelease(prev.0 as _) };

            let elapsed = next.1.duration_since(prev.1).as_millis() as u64;
            prev = next;

            samples.push((IOReportIterator::new(diff), elapsed.max(1)));
        }

        self.previous = Some(prev);
        samples
    }
}

impl Drop for IOReport {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.channels as _);
            CFRelease(self.subscription as _);
            if self.previous.is_some() {
                CFRelease(self.previous.unwrap().0 as _);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_report() -> WithError<()> {
        let channels = [
            ("Energy Model", None), // cpu/gpu/ane power
            // ("CPU Stats", Some(CPU_FREQ_DICE_SUBG)), // cpu freq by cluster
            ("CPU Stats", Some(CPU_FREQ_CORE_SUBG)), // cpu freq per core
            ("GPU Stats", Some(GPU_FREQ_DICE_SUBG)),
        ];
        let measures: usize = 4;
        let mut report = IOReport::new(channels.to_vec()).unwrap();
        let samples = report.get_samples(1000, measures);

        for (report_it, sample_dt) in samples {
            for entry in report_it {
                if entry.group == "Energy Model" {
                    let pwr = match entry.channel.as_str() {
                        "CPU Energy" => cfio_watts(entry.item, &entry.unit, sample_dt)?,
                        "GPU Energy" => cfio_watts(entry.item, &entry.unit, sample_dt)?,
                        c if c.starts_with("ANE") => {
                            cfio_watts(entry.item, &entry.unit, sample_dt)?
                        }
                        _ => continue,
                    };
                    println!("POWER {}: {:.2}W", entry.channel, pwr);
                }
            }
        }

        Ok(())
    }
}
