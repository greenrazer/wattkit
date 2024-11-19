from wattkit_py import PowerMonitorStream

def monitor_power():
    stream = PowerMonitorStream()
    
    try:
        for cpu_power, gpu_power, ane_power, timestamp in stream:
            print(f"Timestamp: {timestamp}")
            print(f"CPU Power: {cpu_power:.2f}W")
            print(f"GPU Power: {gpu_power:.2f}W")
            print(f"ANE Power: {ane_power:.2f}W")
            print("-" * 40)
            
    except KeyboardInterrupt:
        stream.close()
        print("\nMonitoring stopped")

if __name__ == "__main__":
    monitor_power()
