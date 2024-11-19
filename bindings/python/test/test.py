from wattkit import PowerProfiler
import time

with PowerProfiler() as profiler:
    for i in range(5):
        time.sleep(1)
    
avg_power = profiler.average_power      
total_energy = profiler.total_energy    
duration = profiler.duration_seconds

print(f"Average power: {avg_power}")
print(f"Total energy: {total_energy}")
print(f"Duration: {duration} seconds")
