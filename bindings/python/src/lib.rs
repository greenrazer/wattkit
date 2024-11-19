use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use wattkit::*;

#[pyclass]
struct PowerMonitorStream {
    receiver: Receiver<PowerSample>,
    is_running: bool,
}

#[derive(Clone)]
struct PowerSample {
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
            let report = IOReport::new(requests).unwrap();

            loop {
                let samples = report.get_samples(1000, 1);
                for sample in samples {
                    let mut sample = PowerSample {
                        cpu_power: 0.0,
                        gpu_power: 0.0,
                        ane_power: 0.0,
                        timestamp: sample.duration(),
                    };

                    for entry in report_it {
                        if entry.group == "Energy Model" {
                            let unit = EnergyUnit::from(entry.unit.as_str());
                            match entry.channel.as_str() {
                                "CPU Energy" => {
                                    sample.cpu_power =
                                        read_wattage(entry.item, &unit, sample_dt).unwrap_or(0.0);
                                }
                                "GPU Energy" => {
                                    sample.gpu_power =
                                        read_wattage(entry.item, &unit, sample_dt).unwrap_or(0.0);
                                }
                                c if c.starts_with("ANE") => {
                                    sample.ane_power =
                                        read_wattage(entry.item, &unit, sample_dt).unwrap_or(0.0);
                                }
                                _ => continue,
                            }
                        }
                    }

                    if tx.send(sample).is_err() {
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
