from wattkit import PowerProfiler
import coremltools as ct
import numpy as np
from PIL import Image
from torchvision import transforms
from torchvision.transforms.functional import InterpolationMode
from urllib.request import urlopen

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

# Load the CoreML model
cml_model = ct.models.MLModel("FastViTMA36F16.mlmodel")
img = validation_image()

with PowerProfiler() as profiler:
    cml_model.predict({"image": img})
    
profiler.print_summary()
