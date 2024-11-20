use crate::{
    read_wattage, EnergyUnit, IOReport, IOReportChannel, IOReportChannelGroup,
    IOReportChannelRequest,
};
use oneshot::channel as oneshot_channel;
use oneshot::Sender as OneshotSender;

use std::{
    sync::mpsc::{channel, Receiver},
    thread::JoinHandle,
};

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
/// let sampler = Sampler::new(SamplerType::Power);
/// {
///     // Start sampling
///     let guard = sampler.subscribe(1000); //sample every 1000ms
///
///     // Do some work
///     for x in 0..1000000 {
///         let y = x * x;
///     }
/// }
pub struct Sampler {
    samples: Vec<PowerSample>,
}

pub enum SamplerType {
    Energy,
    Temp,
    All,
}

pub struct SamplerGuard<'a> {
    sampler: &'a mut Sampler,
    cancel_sender: Option<OneshotSender<()>>,
    receiver: Receiver<PowerSample>,
    thread_handle: Option<JoinHandle<()>>,
}

impl Drop for SamplerGuard<'_> {
    fn drop(&mut self) {
        if let Some(sender) = self.cancel_sender.take() {
            let _ = sender.send(());
        }

        while let Ok(s) = self.receiver.recv() {
            self.sampler.samples.push(s);
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

#[derive(Clone, Debug)]
struct PowerSample {
    cpu_power: f32,
    gpu_power: f32,
    ane_power: f32,
    timestamp: u64,
}

impl Sampler {
    pub fn new() -> Self {
        Sampler {
            samples: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, duration: u64, num_samples: usize) -> SamplerGuard {
        let (cancel_tx, cancel_rx) = oneshot_channel();
        let (sample_tx, sample_rx) = channel();

        let mut guard = SamplerGuard {
            sampler: self,
            cancel_sender: Some(cancel_tx),
            receiver: sample_rx,
            thread_handle: None,
        };

        let handle = std::thread::spawn(move || {
            let requests = vec![IOReportChannelRequest::new(
                IOReportChannelGroup::EnergyModel,
                None as Option<String>,
            )];
            let mut report = IOReport::new(requests).unwrap();

            loop {
                if cancel_rx.try_recv().is_ok() {
                    break;
                }

                let samples = report.get_samples(duration, num_samples);
                for mut sample in samples {
                    let duration = sample.duration();
                    let mut power_sample = PowerSample {
                        cpu_power: 0.0,
                        gpu_power: 0.0,
                        ane_power: 0.0,
                        timestamp: duration,
                    };

                    for entry in sample.iterator_mut() {
                        match entry.group {
                            IOReportChannelGroup::EnergyModel => {
                                let wattage = unsafe {
                                    read_wattage(
                                        entry.item,
                                        &EnergyUnit::from(entry.unit.as_str()),
                                        duration,
                                    )
                                    .unwrap()
                                };
                                match entry.channel {
                                    IOReportChannel::CPUEnergy => power_sample.cpu_power = wattage,
                                    IOReportChannel::GPUEnergy => power_sample.gpu_power = wattage,
                                    IOReportChannel::ANE => power_sample.ane_power = wattage,
                                    _ => continue,
                                };
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

        guard.thread_handle = Some(handle);

        guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sampler() {
        let mut sampler = Sampler::new();

        {
            let _guard = sampler.subscribe(100, 1);
            std::thread::sleep(std::time::Duration::from_secs(5));
            for i in 0..100 {
                let bingo = i * 2;
                println!("{}", bingo);
            }
        }

        assert!(!sampler.samples.is_empty());
        println!("Number of samples: {}", sampler.samples.len());
        for s in sampler.samples.iter() {
            println!(
                "CPU: {:.2}W, GPU: {:.2}W, ANE: {:.2}W, Time: {}",
                s.cpu_power, s.gpu_power, s.ane_power, s.timestamp
            );
        }
    }
}
