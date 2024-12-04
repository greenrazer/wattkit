from wattkit import Profiler 
import coremltools as ct
from PIL import Image
from torchvision import transforms
from torchvision.transforms.functional import InterpolationMode
from urllib.request import urlopen
import numpy as np

def random_inputs_for_model(model):
    inputs = {}
    
    try:
        spec = model.get_spec()
    except AttributeError:
        spec = model.spec if hasattr(model, 'spec') else model
    
    for input_desc in spec.description.input:
        input_name = input_desc.name
        
        if hasattr(input_desc, 'type'):
            if input_desc.type.HasField('imageType'):
                image_type = input_desc.type.imageType
                height = image_type.height
                width = image_type.width
                
                if image_type.colorSpace == 0: # GRAYSCALE
                    channels = 1
                else: # Default to 3
                    channels = 3

                noise_array = np.random.randint(0, 256, (height, width, channels), dtype=np.uint8)
                inputs[input_name] = Image.fromarray(noise_array)
            elif input_desc.type.HasField('multiArrayType'):
                shape = tuple(input_desc.type.multiArrayType.shape)
                inputs[input_name] = np.random.randn(*shape)
            else:
                raise Exception(f"Could not determine input type for {input_name}")
        else:
            raise Exception(f"Could not determine input type for {input_name}")
    
    return inputs

def validation_image():
    input = Image.open(urlopen(
    'http://images.cocodataset.org/val2017/000000281759.jpg'
    ))
    transform = transforms.Compose(
        [
            transforms.Resize(
                size=284,
                interpolation=InterpolationMode.BICUBIC,
                max_size=None,
                antialias=True,
            ),
            transforms.CenterCrop(size=(256, 256)),
        ]
    )
    return transform(input)

compute_units = ct.ComputeUnit.ALL
cml_model = ct.models.MLModel("FastViTMA36F16.mlpackage", compute_units=compute_units)
img = validation_image()

model_inputs = random_inputs_for_model(cml_model)

model_iterations = 1000
cml_model.predict(model_inputs) # Once before to "warm up" hardware
with Profiler(sample_duration=100, num_samples=2) as profiler:
    for i in range(model_iterations):
        cml_model.predict(model_inputs)

profile = profiler.get_profile()
print(profile)
print(",".join([str(x) for x in [
    model_iterations,
    profile.total_cpu_energy,
    profile.total_gpu_energy,
    profile.total_ane_energy,
    profile.average_cpu_power,
    profile.average_gpu_power,
    profile.average_ane_power,
    profile.total_duration_milliseconds
]]))