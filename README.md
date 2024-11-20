# WattKit - Measure the power usage of your code! (Rust / Python)

> [!WARNING]  
> This is a sandbox for active development! Do not use :) 

`wattkit` intends to provide a method for measuring the power consumption of your Rust or Python code.

```rust
let mut sampler = Sampler::new();
{
    let _guard = sampler.subscribe(100, 1);
    std::thread::sleep(std::time::Duration::from_secs(4));
}
assert!(!sampler.samples().is_empty());
sampler.print_summary();
```


```python
from wattkit import PowerProfiler
import time

with PowerProfiler(duration=100, num_samples=1) as profiler:
    for i in range(10):
        time.sleep(0.5)
    
profiler.print_summary()
```

# TODO
- [ ] Code is pretty jank
- [ ] Surface ContextManager impl
- [ ] Add frequency measurements
- [ ] Add braindead method that does statistical sampling for you
- [ ] Convenience method to generate comparison report of power consumption between compute units? (Seems CoreML specific)

Lots of the reverse engineering work here was done by @vladkens with [macmon](https://github.com/vladkens/macmon).
