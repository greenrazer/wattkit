# WattKit - Measure the power usage of your code! (Rust / Python)

> [!WARNING]  
> This is a sandbox for active development! Do not use :) 

`wattkit` intends to provide a method for measuring the power consumption of your Rust or Python code.

Using RAII, we provide a nice Rust interface:
```rust
let mut sampler = Sampler::new();
{
    let _guard = sampler.subscribe(100, 1); # Will be sampled until drop/end of scope
    std::thread::sleep(std::time::Duration::from_secs(4));
}
assert!(!sampler.samples().is_empty());
sampler.print_summary();
```

We use `pyo3` to provide a Python interface.

```python
from wattkit import PowerProfiler
import time

with PowerProfiler(duration=100, num_samples=1) as profiler:
    for i in range(10):
        time.sleep(0.5)
    
profiler.print_summary()
```

# TODO
- [x] Surface ContextManager impl
- [ ] Code is very jank
- [ ] Improve measurements by determining the process in question, and computing
  what % of the time during the sample period it is running. Use that to compute
  the fraction of the power consumption that is due to the process.
- [ ] Determine Wh capacity of current battery, output what % of total bat cap
  was consumed by the scope 
- [ ] Add frequency measurements (ANE impossible :( )
- [ ] Add braindead method that does statistical sampling for you
- [ ] Convenience method to generate comparison report of power consumption between compute units? (CoreML specific, put in coremlprofiler)
- [ ] Publish on PyPi and crates.io

Lots of the reverse engineering work here was done by @vladkens with [macmon](https://github.com/vladkens/macmon).
