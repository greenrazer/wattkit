use pyo3::prelude::*;
use wattkit::*;

#[pyclass]
struct PowerProfiler {
    sampler: StartStopSampler,
    sample_duration: u64,
    num_samples: usize,
}

#[pymethods]
impl PowerProfiler {
    #[new]
    fn new(sample_duration: u64, num_samples: usize) -> PyResult<Self> {
        Ok(PowerProfiler {
            sampler: StartStopSampler::new(),
            sample_duration,
            num_samples,
        })
    }

    fn __enter__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let duration = slf.sample_duration;
        let num_samples = slf.num_samples;
        slf.sampler.start(duration, num_samples).unwrap();
        assert!(slf.sampler.is_sampling());
        Ok(slf)
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        mut slf: PyRefMut<'_, Self>,
        _exc_type: Option<PyObject>,
        _exc_value: Option<PyObject>,
        _traceback: Option<PyObject>,
    ) -> PyResult<bool> {
        slf.sampler.stop().unwrap();
        assert!(!slf.sampler.is_sampling());
        Ok(true)
    }

    fn print_summary(&self) {
        self.sampler.print_summary()
    }

    fn profile_duration(&self) {
        println!("Profiling duration: {:?}", self.sampler.duration());
    }
}

#[pymodule]
fn _wattkit_pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PowerProfiler>()?;
    Ok(())
}
