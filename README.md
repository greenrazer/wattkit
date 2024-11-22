# WattKit - Measure the power usage of your code on MacOS! (Rust / Python)

> [!WARNING]
> This only works on MacOS!

`wattkit` intends to provide a method for measuring the power consumption of your Rust or Python code.

Currently `wattkit` records the power consumption of the **entire compute unit**
(CPU, GPU, etc) over a period of time. It does not specifically measure your process, use with caution.

Using RAII, we provide a nice Rust interface:
```rust
let mut sampler = Sampler::new();
{
    let _guard = sampler.subscribe(100, 1); # Will be sampled until drop/end of scope
    # Do intensive work here
    # Sleep will measure background processing of your machine :)
    std::thread::sleep(std::time::Duration::from_secs(4));
}

let profile = sampler.profile();
println!("{}", profile);
```

We use `pyo3` to provide a Python interface:
```python
from wattkit import Profiler 
import time

with Profiler(duration=100, num_samples=2) as profiler:
    # Do intensive work here
    for i in range(10):
        time.sleep(0.5)
    
profile = profiler.profile()
print(profile)
```

# TODO
- [x] Surface ContextManager impl
- [x] `num_samples` (sampling multiple times within a sample duration doesn't work)
- [x] Code is very jank
- [ ] Publish on PyPi and crates.io
- [ ] Determine baseline energy consumption of ANE
- [ ] Improve measurements by determining the process in question, and computing
  what % of the time during the sample period it is running. Use that to compute
  the fraction of the power consumption that is due to the process.
- [ ] Determine Wh capacity of current battery, output what % of total bat cap
  was consumed by the scope (check query_maxcap.py for more) 
- [ ] Add frequency measurements (ANE impossible :( )
- [ ] Add braindead method that does statistical sampling for you
- [ ] Convenience method to generate comparison report of power consumption between compute units? (CoreML specific, put in coremlprofiler)


Lots of the reverse engineering work here was done by @vladkens with [macmon](https://github.com/vladkens/macmon).
