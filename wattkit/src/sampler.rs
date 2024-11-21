use oneshot::channel as oneshot_channel;
use oneshot::Sender as OneshotSender;
use std::collections::HashSet;
use std::{
    sync::mpsc::{channel, Receiver},
    thread::JoinHandle,
};

use crate::cf_utils::get_cf_string;
use crate::io_report::read_residencies;
use crate::io_report::IOReportSampleCopyDescription;
use crate::io_report::IOReportSimpleGetIntegerValue;
use crate::io_report::{
    read_wattage, EnergyUnit, IOReport, IOReportChannelGroup, IOReportChannelName,
};

#[derive(Clone, Debug, Default)]
pub struct PowerSample {
    cpu_power: f32,
    gpu_power: f32,
    ane_power: f32,
    duration: u64,
}

#[derive(Debug)]
struct SampleManager {
    cancel_sender: OneshotSender<()>,
    sample_receiver: Receiver<PowerSample>,
    thread_handle: JoinHandle<()>,
}

impl SampleManager {
    fn new(duration: u64, num_samples: usize) -> Self {
        let (cancel_tx, cancel_rx) = oneshot_channel();
        let (sample_tx, sample_rx) = channel();

        let handle = std::thread::spawn(move || {
            let requests = vec![];
            let mut report = IOReport::new(requests).unwrap();
            let mut unique_channel_groups = HashSet::new();
            let mut unique_channel_names = HashSet::new();

            loop {
                if cancel_rx.try_recv().is_ok() {
                    //println!("Cancelling sampling");
                    //for uc in unique_channel_names.iter() {
                    //    println!("unique chan name: {}", uc);
                    //}
                    //for uc in unique_channel_groups.iter() {
                    //    println!("Unique chan grp: {}", uc);
                    //}
                    break;
                }

                let samples = report.get_samples(duration, num_samples);
                for mut sample in samples {
                    let duration = sample.duration();
                    let mut power_sample = PowerSample {
                        duration,
                        ..Default::default()
                    };

                    for entry in sample.iterator_mut() {
                        println!("{:?}", entry);
                        if let IOReportChannelName::Unknown(ref u) = entry.channel_name {
                            unique_channel_names.insert(u.clone());
                        }

                        match entry.group {
                            IOReportChannelGroup::EnergyModel => {
                                let u = EnergyUnit::from(entry.unit);
                                let raw_joules = unsafe {
                                    IOReportSimpleGetIntegerValue(entry.item, std::ptr::null_mut())
                                } as f32;
                                println!("Raw joules: {} {}{}", entry.channel_name, raw_joules, u);
                                let w = read_wattage(entry.item, &u, duration).unwrap();
                                match entry.channel_name {
                                    IOReportChannelName::CPUEnergy => power_sample.cpu_power += w,
                                    IOReportChannelName::GPUEnergy => power_sample.gpu_power += w,
                                    IOReportChannelName::ANE => power_sample.ane_power += w,
                                    _ => {}
                                };
                            }
                            IOReportChannelGroup::SoCStats => match entry.channel_name {
                                IOReportChannelName::ANE => {
                                    let y = read_residencies(entry.item);
                                    println!("ANE: {:?}", y);
                                }
                                _ => {}
                            },
                            IOReportChannelGroup::H11ANE => {
                                println!("H11ANE: {:?}", entry.item);
                                let desc = get_cf_string(|| unsafe {
                                    IOReportSampleCopyDescription(entry.item, 0)
                                });
                                let raw_value = unsafe {
                                    IOReportSimpleGetIntegerValue(entry.item, std::ptr::null_mut())
                                } as f32;
                                println!("{:?}", desc);
                                println!("Raw value: {}", raw_value);
                            }
                            IOReportChannelGroup::Unknown(u) => {
                                unique_channel_groups.insert(u);
                            }
                            _ => continue,
                        }
                    }
                    if sample_tx.send(power_sample).is_err() {
                        break;
                    }
                }
            }
        });

        SampleManager {
            cancel_sender: cancel_tx,
            sample_receiver: sample_rx,
            thread_handle: handle,
        }
    }

    fn stop(self) -> Vec<PowerSample> {
        let _ = self.cancel_sender.send(());
        let mut samples = Vec::new();
        while let Ok(sample) = self.sample_receiver.recv() {
            samples.push(sample);
        }
        let _ = self.thread_handle.join();
        samples
    }
}

/// # Sampler
///
/// The `Sampler` struct is used to sample the power consumption of the device.
/// When sampling begins, the `Sampler` will `subscribe` to the underlying IOReport
/// C API, and will begin to receive power samples.
///
/// These values are placed onto a queue, which can then be accessed by the user.
///
/// ## Example
/// ```rust
/// use wattkit::*;
///
/// let sampler = Sampler::new(SamplerType::Energy);
/// {
///     // Start sampling
///     let guard = sampler.subscribe(1000); //sample every 1000ms
///
///     // Do some work
///     for x in 0..1000000 {
///         let y = x * x;
///     }
/// }
/// sampler.print_summary();
#[derive(Debug, Default)]
pub struct Sampler {
    samples: Vec<PowerSample>,
}

pub struct SamplerGuard<'a> {
    sampler: &'a mut Sampler,
    manager: Option<SampleManager>,
}

impl<'a> Drop for SamplerGuard<'a> {
    fn drop(&mut self) {
        if let Some(manager) = self.manager.take() {
            self.sampler.samples.extend(manager.stop());
        }
    }
}

impl Sampler {
    pub fn new() -> Self {
        Sampler {
            samples: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, duration: u64, num_samples: usize) -> SamplerGuard {
        SamplerGuard {
            sampler: self,
            manager: Some(SampleManager::new(duration, num_samples)),
        }
    }

    pub fn samples(&self) -> &Vec<PowerSample> {
        &self.samples
    }

    pub fn print_summary(&self) {
        for s in self.samples.iter() {
            println!(
                "CPU: {:.2}W, GPU: {:.2}W, ANE: {:.2}W, Time: {}",
                s.cpu_power, s.gpu_power, s.ane_power, s.duration
            );
        }
    }
}

/// # StartStopSampler
///
/// Exclusively for use with pyo3, use `Sampler` from Rust instead.
#[derive(Debug, Default)]
pub struct StartStopSampler {
    samples: Vec<PowerSample>,
    manager: Option<SampleManager>,
}

impl StartStopSampler {
    pub fn new() -> Self {
        StartStopSampler {
            samples: Vec::new(),
            manager: None,
        }
    }

    pub fn start(&mut self, duration: u64, num_samples: usize) -> Result<(), &'static str> {
        if self.manager.is_some() {
            return Err("Sampling is already in progress");
        }
        self.manager = Some(SampleManager::new(duration, num_samples));
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), &'static str> {
        println!("Stopping sampling");
        if let Some(core) = self.manager.take() {
            self.samples.extend(core.stop());
            Ok(())
        } else {
            Err("No sampling in progress")
        }
    }

    pub fn is_sampling(&self) -> bool {
        self.manager.is_some()
    }

    pub fn samples(&self) -> &Vec<PowerSample> {
        &self.samples
    }

    pub fn print_summary(&self) {
        println!("SAMPLES: {}", self.samples.len());
        for s in self.samples.iter() {
            println!(
                "CPU: {:.2}W, GPU: {:.2}W, ANE: {:.2}W, Time: {}",
                s.cpu_power, s.gpu_power, s.ane_power, s.duration
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_sampler() {
        let mut sampler = Sampler::new();
        {
            let _guard = sampler.subscribe(100, 1);
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        assert!(!sampler.samples().is_empty());
        sampler.print_summary();
    }

    #[test]
    fn test_start_stop_sampler() {
        let mut sampler = StartStopSampler::new();

        assert!(!sampler.is_sampling());
        sampler.start(100, 1).unwrap();
        assert!(sampler.is_sampling());

        std::thread::sleep(std::time::Duration::from_secs(4));

        sampler.stop().unwrap();
        assert!(!sampler.is_sampling());
        assert!(!sampler.samples().is_empty());
        sampler.print_summary();
    }
}
