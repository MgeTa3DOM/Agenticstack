#!/usr/bin/env python3
"""
Apophy Sovereign — AZR-CTM AutoFineTune Loop
=============================================

Self-play data generation + QLoRA fine-tuning pipeline.
Uses Unsloth for 2x faster training with 80% less memory.

Requirements:
    uv add unsloth torch datasets accelerate bitsandbytes
    # OR
    pip install unsloth torch datasets accelerate bitsandbytes

Usage:
    # Generate self-play dataset from AZR episodes
    python trainer.py generate --episodes 1000 --domain math

    # Fine-tune with QLoRA
    python trainer.py train --base-model google/gemma-2-2b-it --epochs 3

    # Export to GGUF for Apophy Sovereign
    python trainer.py export --format gguf --output models/azr-ctm.gguf

    # Full pipeline (generate → train → export)
    python trainer.py pipeline --episodes 500 --output models/sovereign.gguf
"""

import argparse
import json
import os
import sys
from pathlib import Path
from datetime import datetime


# === AZR Self-Play Dataset Generator ===

def generate_azr_dataset(
    api_url: str = "http://localhost:8080",
    episodes: int = 1000,
    domain: str = "reasoning",
    difficulty: str = "medium",
    output: str = "data/azr_dataset.jsonl",
) -> Path:
    """Generate training data via AZR self-play episodes."""
    import requests

    output_path = Path(output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"Generating {episodes} AZR episodes (domain={domain}, difficulty={difficulty})")
    print(f"API: {api_url}")
    print(f"Output: {output_path}")

    successful = 0
    with open(output_path, "w") as f:
        for i in range(episodes):
            try:
                resp = requests.post(
                    f"{api_url}/api/v1/brain/self-play",
                    json={"domain": domain, "difficulty": difficulty},
                    timeout=30,
                )
                if resp.status_code == 200:
                    episode = resp.json()
                    # Convert to training format (instruction → response)
                    training_example = {
                        "instruction": episode["task"]["question"],
                        "output": episode["solution"],
                        "domain": episode["task"]["domain"],
                        "difficulty": episode["task"]["difficulty"],
                        "verified": episode["verification"]["correct"],
                        "confidence": episode["verification"]["confidence"],
                    }
                    # Only keep verified correct solutions
                    if training_example["verified"]:
                        f.write(json.dumps(training_example) + "\n")
                        successful += 1

                if (i + 1) % 100 == 0:
                    print(f"  [{i+1}/{episodes}] {successful} verified examples")
            except Exception as e:
                print(f"  Episode {i} failed: {e}")

    print(f"\nGenerated {successful}/{episodes} verified training examples")
    print(f"Saved to: {output_path}")
    return output_path


# === QLoRA Fine-Tuning ===

def train_qlora(
    dataset_path: str = "data/azr_dataset.jsonl",
    base_model: str = "google/gemma-2-2b-it",
    output_dir: str = "checkpoints/azr-ctm",
    epochs: int = 3,
    batch_size: int = 4,
    lora_r: int = 16,
    lora_alpha: int = 32,
    learning_rate: float = 2e-4,
    max_seq_length: int = 4096,
):
    """Fine-tune with QLoRA using Unsloth for sovereign local model."""
    try:
        from unsloth import FastLanguageModel
        from datasets import Dataset
        from trl import SFTTrainer
        from transformers import TrainingArguments
    except ImportError:
        print("Missing dependencies. Install with:")
        print("  uv add unsloth torch datasets trl accelerate bitsandbytes")
        sys.exit(1)

    print(f"=== AZR-CTM QLoRA Fine-Tune ===")
    print(f"Base model: {base_model}")
    print(f"Dataset: {dataset_path}")
    print(f"LoRA r={lora_r}, alpha={lora_alpha}")
    print(f"Epochs: {epochs}, Batch: {batch_size}, LR: {learning_rate}")

    # Load dataset
    examples = []
    with open(dataset_path) as f:
        for line in f:
            ex = json.loads(line.strip())
            # Format as chat
            examples.append({
                "text": f"<bos><start_of_turn>user\n{ex['instruction']}<end_of_turn>\n"
                        f"<start_of_turn>model\n{ex['output']}<end_of_turn><eos>"
            })

    dataset = Dataset.from_list(examples)
    print(f"Training examples: {len(dataset)}")

    # Load model with Unsloth (2x faster, 80% less memory)
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=base_model,
        max_seq_length=max_seq_length,
        load_in_4bit=True,
    )

    # Apply QLoRA
    model = FastLanguageModel.get_peft_model(
        model,
        r=lora_r,
        lora_alpha=lora_alpha,
        target_modules=[
            "q_proj", "k_proj", "v_proj", "o_proj",
            "gate_proj", "up_proj", "down_proj",
        ],
        lora_dropout=0.05,
        bias="none",
        use_gradient_checkpointing="unsloth",
    )

    # Training
    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=dataset,
        dataset_text_field="text",
        max_seq_length=max_seq_length,
        args=TrainingArguments(
            output_dir=output_dir,
            num_train_epochs=epochs,
            per_device_train_batch_size=batch_size,
            gradient_accumulation_steps=4,
            learning_rate=learning_rate,
            weight_decay=0.01,
            warmup_steps=50,
            logging_steps=10,
            save_strategy="epoch",
            fp16=True,
            optim="adamw_8bit",
        ),
    )

    print("\nTraining started...")
    stats = trainer.train()
    print(f"\nTraining complete: {stats.metrics}")

    # Save
    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)
    print(f"Model saved to: {output_dir}")

    return output_dir


# === GGUF Export ===

def export_gguf(
    checkpoint_dir: str = "checkpoints/azr-ctm",
    output_path: str = "models/azr-ctm.gguf",
    quantization: str = "q4_k_m",
):
    """Export fine-tuned model to GGUF format for Apophy Sovereign."""
    try:
        from unsloth import FastLanguageModel
    except ImportError:
        print("Missing unsloth. Install with: uv add unsloth")
        sys.exit(1)

    print(f"=== GGUF Export ===")
    print(f"Checkpoint: {checkpoint_dir}")
    print(f"Output: {output_path}")
    print(f"Quantization: {quantization}")

    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=checkpoint_dir,
        max_seq_length=4096,
        load_in_4bit=True,
    )

    # Merge LoRA weights and export
    model.save_pretrained_gguf(
        output_path.replace(".gguf", ""),
        tokenizer,
        quantization_method=quantization,
    )

    print(f"\nGGUF exported to: {output_path}")
    print(f"Copy to Apophy Sovereign: cp {output_path} config/../models/default.gguf")


# === Full Pipeline ===

def full_pipeline(
    api_url: str = "http://localhost:8080",
    episodes: int = 500,
    base_model: str = "google/gemma-2-2b-it",
    output: str = "models/sovereign.gguf",
    domain: str = "reasoning",
):
    """Full pipeline: AZR generate → QLoRA train → GGUF export."""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M")
    dataset_path = f"data/azr_{domain}_{timestamp}.jsonl"
    checkpoint_dir = f"checkpoints/azr-ctm-{timestamp}"

    print("=" * 60)
    print("  APOPHY SOVEREIGN — AZR-CTM AutoFineTune Pipeline")
    print("=" * 60)
    print(f"  Episodes:   {episodes}")
    print(f"  Domain:     {domain}")
    print(f"  Base Model: {base_model}")
    print(f"  Output:     {output}")
    print("=" * 60)

    # Step 1: Generate
    print("\n[1/3] Generating AZR self-play dataset...")
    dataset_file = generate_azr_dataset(
        api_url=api_url,
        episodes=episodes,
        domain=domain,
        output=dataset_path,
    )

    # Step 2: Train
    print("\n[2/3] Fine-tuning with QLoRA...")
    train_qlora(
        dataset_path=str(dataset_file),
        base_model=base_model,
        output_dir=checkpoint_dir,
    )

    # Step 3: Export
    print("\n[3/3] Exporting to GGUF...")
    export_gguf(
        checkpoint_dir=checkpoint_dir,
        output_path=output,
    )

    print("\n" + "=" * 60)
    print("  PIPELINE COMPLETE")
    print(f"  Model: {output}")
    print(f"  Deploy: apophy-sovereign start --config config/sovereign.toml")
    print("=" * 60)


# === ONNX Export for Browser Inference (Gemma3 270M) ===

def export_onnx(
    checkpoint_dir: str = "checkpoints/azr-ctm",
    output_dir: str = "models/onnx/",
    quantize: bool = True,
):
    """Export fine-tuned model to ONNX format for browser inference via Transformers.js/WebGPU.

    Target: Gemma3 270M parameters, optimized for in-browser execution.
    """
    try:
        from optimum.exporters.onnx import main_export
    except ImportError:
        print("Missing optimum. Install with: uv pip install optimum onnx onnxruntime")
        sys.exit(1)

    print(f"=== ONNX Export for Browser AI ===")
    print(f"Checkpoint: {checkpoint_dir}")
    print(f"Output: {output_dir}")
    print(f"Quantize: {quantize}")

    os.makedirs(output_dir, exist_ok=True)

    # Export to ONNX
    main_export(
        model_name_or_path=checkpoint_dir,
        output=output_dir,
        task="text-generation",
    )

    # Quantize for smaller browser payload
    if quantize:
        try:
            from onnxruntime.quantization import quantize_dynamic, QuantType
            import glob as globmod

            for onnx_file in globmod.glob(os.path.join(output_dir, "*.onnx")):
                quantized = onnx_file.replace(".onnx", "_q4.onnx")
                quantize_dynamic(
                    onnx_file,
                    quantized,
                    weight_type=QuantType.QUInt8,
                )
                print(f"  Quantized: {quantized}")
        except ImportError:
            print("  [WARN] onnxruntime not available for quantization")

    print(f"\nONNX export complete: {output_dir}")
    print("  Deploy to browser via Transformers.js:")
    print("    import { pipeline } from '@xenova/transformers';")
    print(f"    const generator = await pipeline('text-generation', '{output_dir}');")


# === Gemma3 270M Specialization Pipeline ===

def specialize_gemma3(
    mini_dataset_dir: str = "data/mini/",
    base_model: str = "google/gemma-3-270m",
    output_dir: str = "models/specialized/",
    max_agents: int = 100,
):
    """Fine-tune Gemma3 270M for each agent specialization.

    Creates one LoRA adapter per agent domain, exported to ONNX for browser inference.
    Designed for the 3000 micro-agent architecture where each agent runs in-browser.
    """
    print(f"=== Gemma3 270M Agent Specialization ===")
    print(f"Dataset dir: {mini_dataset_dir}")
    print(f"Base model: {base_model}")
    print(f"Max agents: {max_agents}")

    dataset_path = Path(mini_dataset_dir)
    if not dataset_path.exists():
        print(f"  [ERROR] Dataset directory not found: {mini_dataset_dir}")
        print("  Run: python tools/dataset-processor/processor.py pipeline --input data/raw/")
        return

    domains = [d for d in dataset_path.iterdir() if d.is_dir()]
    print(f"  Found {len(domains)} domains")

    trained = 0
    for domain_dir in domains:
        domain = domain_dir.name
        agent_files = sorted(domain_dir.glob("*.jsonl"))[:max_agents // len(domains)]

        for agent_file in agent_files:
            agent_id = agent_file.stem
            agent_output = os.path.join(output_dir, domain, agent_id)
            os.makedirs(agent_output, exist_ok=True)

            print(f"  [{trained + 1}] Training {agent_id} (domain: {domain})")

            try:
                train_qlora(
                    dataset_path=str(agent_file),
                    base_model=base_model,
                    output_dir=agent_output,
                    epochs=2,
                    batch_size=2,
                    lora_r=8,
                    lora_alpha=16,
                    max_seq_length=1024,
                )
                trained += 1
            except Exception as e:
                print(f"    [SKIP] {agent_id}: {e}")

    print(f"\nSpecialization complete: {trained} agents trained")
    print(f"Output: {output_dir}")


# === CLI ===

def main():
    parser = argparse.ArgumentParser(
        description="Apophy Sovereign — AZR-CTM AutoFineTune Loop"
    )
    subparsers = parser.add_subparsers(dest="command")

    # Generate
    gen = subparsers.add_parser("generate", help="Generate AZR self-play dataset")
    gen.add_argument("--api-url", default="http://localhost:8080")
    gen.add_argument("--episodes", type=int, default=1000)
    gen.add_argument("--domain", default="reasoning")
    gen.add_argument("--difficulty", default="medium")
    gen.add_argument("--output", default="data/azr_dataset.jsonl")

    # Train
    trn = subparsers.add_parser("train", help="Fine-tune with QLoRA")
    trn.add_argument("--dataset", default="data/azr_dataset.jsonl")
    trn.add_argument("--base-model", default="google/gemma-2-2b-it")
    trn.add_argument("--output-dir", default="checkpoints/azr-ctm")
    trn.add_argument("--epochs", type=int, default=3)
    trn.add_argument("--batch-size", type=int, default=4)
    trn.add_argument("--lora-r", type=int, default=16)

    # Export
    exp = subparsers.add_parser("export", help="Export to GGUF")
    exp.add_argument("--checkpoint", default="checkpoints/azr-ctm")
    exp.add_argument("--output", default="models/azr-ctm.gguf")
    exp.add_argument("--quantization", default="q4_k_m")

    # Pipeline
    pipe = subparsers.add_parser("pipeline", help="Full pipeline")
    pipe.add_argument("--api-url", default="http://localhost:8080")
    pipe.add_argument("--episodes", type=int, default=500)
    pipe.add_argument("--base-model", default="google/gemma-2-2b-it")
    pipe.add_argument("--output", default="models/sovereign.gguf")
    pipe.add_argument("--domain", default="reasoning")

    # ONNX export for browser
    onnx = subparsers.add_parser("export-onnx", help="Export to ONNX for browser inference")
    onnx.add_argument("--checkpoint", default="checkpoints/azr-ctm")
    onnx.add_argument("--output", default="models/onnx/")
    onnx.add_argument("--no-quantize", action="store_true")

    # Gemma3 specialization
    spec = subparsers.add_parser("specialize", help="Fine-tune Gemma3 270M per-agent")
    spec.add_argument("--dataset-dir", default="data/mini/")
    spec.add_argument("--base-model", default="google/gemma-3-270m")
    spec.add_argument("--output", default="models/specialized/")
    spec.add_argument("--max-agents", type=int, default=100)

    args = parser.parse_args()

    if args.command == "generate":
        generate_azr_dataset(
            api_url=args.api_url,
            episodes=args.episodes,
            domain=args.domain,
            difficulty=args.difficulty,
            output=args.output,
        )
    elif args.command == "train":
        train_qlora(
            dataset_path=args.dataset,
            base_model=args.base_model,
            output_dir=args.output_dir,
            epochs=args.epochs,
            batch_size=args.batch_size,
            lora_r=args.lora_r,
        )
    elif args.command == "export":
        export_gguf(
            checkpoint_dir=args.checkpoint,
            output_path=args.output,
            quantization=args.quantization,
        )
    elif args.command == "export-onnx":
        export_onnx(
            checkpoint_dir=args.checkpoint,
            output_dir=args.output,
            quantize=not args.no_quantize,
        )
    elif args.command == "specialize":
        specialize_gemma3(
            mini_dataset_dir=args.dataset_dir,
            base_model=args.base_model,
            output_dir=args.output,
            max_agents=args.max_agents,
        )
    elif args.command == "pipeline":
        full_pipeline(
            api_url=args.api_url,
            episodes=args.episodes,
            base_model=args.base_model,
            output=args.output,
            domain=args.domain,
        )
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
