import subprocess
import re

def get_battery_info():
    """
    Get battery capacity and current state information using ioreg on macOS
    Returns capacity in Watt-hours and current state
    """
    try:
        # Run ioreg command to get battery information
        cmd = ["ioreg", "-r", "-c", "AppleSmartBattery"]
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        if result.returncode != 0:
            return f"Error running ioreg: {result.stderr}"

        output = result.stdout
        
        # Extract values using regex
        def extract_value(pattern):
            match = re.search(pattern + r'" = (\d+)', output)
            return int(match.group(1)) if match else 0
            
        def extract_string(pattern):
            match = re.search(pattern + r'" = \"([^"]+)\"', output)
            return match.group(1) if match else "Unknown"

        # Get design values
        max_capacity = extract_value('"AppleRawMaxCapacity')
        current_capacity = extract_value('"AppleRawCurrentCapacity')
        
        # Get current state
        voltage_now = extract_value('"Voltage') / 1000.0  # Current voltage in V
        temperature = extract_value('"Temperature') / 100.0  # Temperature in Celsius
        cycle_count = extract_value('"CycleCount')
        charging = extract_string('"IsCharging')
        
        max_capacity_wh = (max_capacity * voltage_now) / 1000
        current_wh = (current_capacity * voltage_now) / 1000
        
        return {
            'capacity': {
                'design_mah': max_capacity,
                'current_mah': current_capacity,
                'max_wh': round(max_capacity_wh, 2),
                'current_wh': round(current_wh, 2),
                'percentage': round(current_capacity / max_capacity * 100, 1),
            },
            'state': {
                'charging': charging == "Yes",
                'cycles': cycle_count,
                'temperature': temperature
            }
        }
        
    except Exception as e:
        return f"Error: {str(e)}"

if __name__ == "__main__":
    info = get_battery_info()
    if isinstance(info, dict):
        print("\nBattery Information:")
        print("\nCapacity:")
        print(f"Design Capacity: {info['capacity']['design_mah']} mAh")
        print(f"Current Max Capacity: {info['capacity']['current_max_mah']} mAh")
        print(f"Current Charge: {info['capacity']['current_mah']} mAh ({info['capacity']['percentage']}%)")
        print(f"Current Charge in Wh: {info['capacity']['current_wh']} Wh")
        print(f"Charging: {'Yes' if info['state']['charging'] else 'No'}")
        print(f"Cycle Count: {info['state']['cycles']}")
        print(f"Temperature: {info['state']['temperature']}°C")
    else:
        print(info)
