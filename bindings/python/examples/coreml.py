from wattkit import Profiler 
import coremltools as ct
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

compute_units = ct.ComputeUnit.ALL
cml_model = ct.models.MLModel("FastViTMA36F16.mlpackage", compute_units=compute_units)
img = validation_image()

with Profiler(sample_duration=100, num_samples=2) as profiler:
    for i in range(1000):
        cml_model.predict({"image": img})

profile = profiler.get_profile()
print(profile)
    
