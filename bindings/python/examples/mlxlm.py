from mlx_lm import load, generate
from wattkit import Profiler 

model, tokenizer = load("mlx-community/Llama-3.2-1B-Instruct-4bit")

prompt="Tell me about Napoleon's life, birthplace and death"

if hasattr(tokenizer, "apply_chat_template") and tokenizer.chat_template is not None:
    messages = [{"role": "user", "content": prompt}]
    prompt = tokenizer.apply_chat_template(
        messages, tokenize=False, add_generation_prompt=True
    )

with Profiler(sample_duration=100, num_samples=2) as profiler:
    response = generate(model, tokenizer, prompt=prompt, verbose=True)

profile = profiler.get_profile()
print(profile)
    
