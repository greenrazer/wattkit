use pyo3::prelude::*;
use wattkit::*;

#[pyclass]
struct PowerProfiler {
    sampler: Sampler,
}

#[pymodule]
fn _wattkit_pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PowerProfiler>()?;
    Ok(())
}
