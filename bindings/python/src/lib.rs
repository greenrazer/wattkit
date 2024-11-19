use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use wattkit::*;

#[pyclass]
struct PowerMonitorStream {
    receiver: Receiver<PowerResult>,
    is_running: bool,
}

#[derive(Clone)]
struct PowerResult {
    cpu_power: f32,
    gpu_power: f32,
    ane_power: f32,
    timestamp: u64,
}

#[pymethods]
impl PowerMonitorStream {
    #[new]
    fn new() -> PyResult<Self> {
        let (tx, rx) = channel();

        thread::spawn(move || {
            let requests = vec![IOReportChannelRequest::new("Energy Model", None)];
            let mut report = IOReport::new(requests).unwrap();

            loop {
                let samples = report.get_samples(1000, 1);
                for mut sample in samples {
                    let duration = sample.duration();
                    let mut result = PowerResult {
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
                                    IOReportChannel::CPUEnergy => result.cpu_power = wattage,
                                    IOReportChannel::GPUEnergy => {
                                        result.gpu_power = wattage;
                                    }
                                    IOReportChannel::ANE => {
                                        result.ane_power = wattage;
                                    }
                                    _ => continue,
                                };
                            }
                            _ => continue,
                        }
                    }

                    if tx.send(result).is_err() {
                        // Channel closed, stop sampling
                        break;
                    }
                }
            }
        });

        Ok(PowerMonitorStream {
            receiver: rx,
            is_running: true,
        })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<(f32, f32, f32, u64)>> {
        if !slf.is_running {
            return Err(PyStopIteration::new_err("Stream closed"));
        }

        match slf.receiver.recv() {
            Ok(sample) => Ok(Some((
                sample.cpu_power,
                sample.gpu_power,
                sample.ane_power,
                sample.timestamp,
            ))),
            Err(_) => {
                slf.is_running = false;
                Err(PyStopIteration::new_err("Stream ended"))
            }
        }
    }

    fn close(&mut self) {
        self.is_running = false;
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn _wattkit_pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PowerMonitorStream>()?;
    Ok(())
}
