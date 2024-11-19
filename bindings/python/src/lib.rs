use pyo3::prelude::*;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use wattkit::*;

#[derive(Clone)]
struct PowerSample {
    cpu_power: f32,
    gpu_power: f32,
    ane_power: f32,
    timestamp: u64,
}

#[pyclass]
struct PowerProfiler {
    receiver: Option<Receiver<PowerSample>>,
    samples: Vec<PowerSample>,
    start_time: u64,
    end_time: u64,
}

#[pymethods]
impl PowerProfiler {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(PowerProfiler {
            receiver: None,
            samples: Vec::new(),
            start_time: 0,
            end_time: 0,
        })
    }

    fn __enter__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let (tx, rx) = channel();
        slf.receiver = Some(rx);

        thread::spawn(move || {
            let requests = vec![IOReportChannelRequest::new(
                IOReportChannelGroup::EnergyModel,
                None as Option<String>,
            )];
            let mut report = IOReport::new(requests).unwrap();

            loop {
                let samples = report.get_samples(100, 1);
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
                    if tx.send(power_sample).is_err() {
                        break;
                    }
                }
            }
        });

        slf.start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Ok(slf)
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        mut slf: PyRefMut<'_, Self>,
        _exc_type: Option<PyObject>,
        _exc_value: Option<PyObject>,
        _traceback: Option<PyObject>,
    ) -> PyResult<bool> {
        slf.end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if let Some(receiver) = slf.receiver.take() {
            while let Ok(sample) = receiver.try_recv() {
                slf.samples.push(sample);
            }
        }

        Ok(false)
    }

    #[getter]
    fn get_average_power(&self) -> PyResult<(f32, f32, f32)> {
        if self.samples.is_empty() {
            return Ok((0.0, 0.0, 0.0));
        }

        let sum = self.samples.iter().fold((0.0, 0.0, 0.0), |acc, sample| {
            (
                acc.0 + sample.cpu_power,
                acc.1 + sample.gpu_power,
                acc.2 + sample.ane_power,
            )
        });

        let count = self.samples.len() as f32;
        Ok((sum.0 / count, sum.1 / count, sum.2 / count))
    }

    #[getter]
    fn get_total_energy(&self) -> PyResult<(f32, f32, f32)> {
        let duration_secs = (self.end_time - self.start_time) as f32 / 1000.0;
        let (avg_cpu, avg_gpu, avg_ane) = self.get_average_power()?;

        Ok((
            avg_cpu * duration_secs,
            avg_gpu * duration_secs,
            avg_ane * duration_secs,
        ))
    }

    #[getter]
    fn get_duration_seconds(&self) -> PyResult<f32> {
        Ok((self.end_time - self.start_time) as f32 / 1000.0)
    }
}

#[pymodule]
fn _wattkit_pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PowerProfiler>()?;
    Ok(())
}
