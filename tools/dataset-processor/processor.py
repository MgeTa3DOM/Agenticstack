#!/usr/bin/env python3
"""
Apophy Sovereign — Dataset Processor
=====================================

Converts PDF, CSV, MD files into specialized mini-datasets for agent fine-tuning.
Each agent gets its own mini-dataset (50-100 prompts) for Gemma3 270M fine-tuning.

Strategy:
  - 3000 micro-agents: Mini-datasets (~50-100 prompts each), fine-tuned Gemma3 270M
  - 200 business agents: Full datasets (~500-1000 prompts each), Gemini Pro/Claude

Requirements:
    uv pip install pandas pdfplumber markdown

Usage:
    # Process all files in a directory
    python processor.py scan --input data/raw/ --output data/datasets/

    # Generate mini-datasets for agent specialization
    python processor.py specialize --input data/datasets/ --agents 3000 --output data/mini/

    # Generate full datasets for business agents
    python processor.py business --input data/datasets/ --agents 200 --output data/business/

    # Full pipeline
    python processor.py pipeline --input data/raw/ --output data/
"""

import argparse
import csv
import json
import os
import re
import sys
from pathlib import Path
from datetime import datetime
from typing import Optional


# === PDF Processor ===

def extract_pdf(path: str) -> list[dict]:
    """Extract structured content from PDF files."""
    try:
        import pdfplumber
    except ImportError:
        print("  [WARN] pdfplumber not installed, skipping PDF. Install: uv pip install pdfplumber")
        return []

    records = []
    try:
        with pdfplumber.open(path) as pdf:
            for i, page in enumerate(pdf.pages):
                text = page.extract_text()
                if text and text.strip():
                    records.append({
                        "source": os.path.basename(path),
                        "type": "pdf",
                        "page": i + 1,
                        "content": text.strip(),
                        "length": len(text.strip()),
                    })
                # Extract tables if present
                tables = page.extract_tables()
                for j, table in enumerate(tables):
                    if table:
                        records.append({
                            "source": os.path.basename(path),
                            "type": "pdf_table",
                            "page": i + 1,
                            "table_index": j,
                            "content": json.dumps(table),
                            "length": len(str(table)),
                        })
    except Exception as e:
        print(f"  [ERROR] Failed to parse PDF {path}: {e}")

    return records


# === CSV Processor ===

def extract_csv(path: str) -> list[dict]:
    """Extract structured content from CSV files."""
    records = []
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            reader = csv.DictReader(f)
            headers = reader.fieldnames or []
            for i, row in enumerate(reader):
                content = " | ".join(f"{k}: {v}" for k, v in row.items() if v)
                if content.strip():
                    records.append({
                        "source": os.path.basename(path),
                        "type": "csv",
                        "row": i + 1,
                        "headers": headers,
                        "content": content.strip(),
                        "length": len(content.strip()),
                    })
    except Exception as e:
        print(f"  [ERROR] Failed to parse CSV {path}: {e}")

    return records


# === Markdown Processor ===

def extract_markdown(path: str) -> list[dict]:
    """Extract structured content from Markdown files, split by headers."""
    records = []
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        # Split by headers (## or ###)
        sections = re.split(r'\n(#{1,3}\s+.+)', content)

        current_header = "Introduction"
        current_body = ""

        for part in sections:
            if re.match(r'^#{1,3}\s+', part):
                # Save previous section
                if current_body.strip():
                    records.append({
                        "source": os.path.basename(path),
                        "type": "markdown",
                        "section": current_header.strip(),
                        "content": current_body.strip(),
                        "length": len(current_body.strip()),
                    })
                current_header = part.lstrip('#').strip()
                current_body = ""
            else:
                current_body += part

        # Last section
        if current_body.strip():
            records.append({
                "source": os.path.basename(path),
                "type": "markdown",
                "section": current_header.strip(),
                "content": current_body.strip(),
                "length": len(current_body.strip()),
            })
    except Exception as e:
        print(f"  [ERROR] Failed to parse MD {path}: {e}")

    return records


# === Scanner ===

def scan_directory(input_dir: str, output_dir: str) -> Path:
    """Scan directory for PDF/CSV/MD files and extract all content."""
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    all_records = []
    file_counts = {"pdf": 0, "csv": 0, "md": 0}

    print(f"Scanning: {input_path}")

    for ext, extractor in [(".pdf", extract_pdf), (".csv", extract_csv), (".md", extract_markdown)]:
        for fpath in input_path.rglob(f"*{ext}"):
            print(f"  Processing: {fpath}")
            records = extractor(str(fpath))
            all_records.extend(records)
            file_counts[ext.lstrip(".")] += 1

    # Save as JSONL
    output_file = output_path / "extracted_dataset.jsonl"
    with open(output_file, "w") as f:
        for record in all_records:
            f.write(json.dumps(record) + "\n")

    print(f"\nScan complete:")
    print(f"  Files: {sum(file_counts.values())} ({file_counts})")
    print(f"  Records: {len(all_records)}")
    print(f"  Output: {output_file}")

    return output_file


# === Mini-Dataset Specializer (3000 micro-agents) ===

def specialize_mini_datasets(
    input_path: str,
    num_agents: int = 3000,
    prompts_per_agent: int = 75,
    output_dir: str = "data/mini/",
) -> Path:
    """Create mini-datasets for each micro-agent specialization.

    Each agent gets 50-100 training prompts crafted from the extracted content.
    These are formatted for Gemma3 270M fine-tuning via LoRA.
    """
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    # Load extracted records
    records = []
    with open(input_path) as f:
        for line in f:
            records.append(json.loads(line.strip()))

    if not records:
        print("No records found in input. Run 'scan' first.")
        return output_path

    # Domains for specialization (matching the 9 Outskill domains)
    domains = [
        "startups", "tech_web_dev", "customer_support",
        "sales", "hr", "marketing",
        "ecommerce", "project_management", "legal"
    ]

    agents_per_domain = num_agents // len(domains)
    records_per_domain = len(records) // len(domains)

    manifest = {
        "created": datetime.now().isoformat(),
        "total_agents": num_agents,
        "prompts_per_agent": prompts_per_agent,
        "domains": {},
    }

    total_created = 0
    for i, domain in enumerate(domains):
        domain_dir = output_path / domain
        domain_dir.mkdir(exist_ok=True)

        # Get domain-relevant records (round-robin distribution)
        domain_records = records[i * records_per_domain: (i + 1) * records_per_domain]
        if not domain_records:
            domain_records = records  # fallback: use all

        for agent_idx in range(agents_per_domain):
            agent_id = f"{domain}-agent-{agent_idx:04d}"
            agent_file = domain_dir / f"{agent_id}.jsonl"

            # Generate training prompts from content chunks
            with open(agent_file, "w") as f:
                for p in range(min(prompts_per_agent, len(domain_records))):
                    record = domain_records[p % len(domain_records)]
                    content = record["content"][:500]  # Truncate for mini-dataset

                    training_example = {
                        "instruction": f"As a {domain} specialist, analyze: {content[:100]}...",
                        "output": f"Based on the {domain} context: {content}",
                        "domain": domain,
                        "agent_id": agent_id,
                        "source": record.get("source", "unknown"),
                    }
                    f.write(json.dumps(training_example) + "\n")

            total_created += 1

        manifest["domains"][domain] = {
            "agents": agents_per_domain,
            "records": len(domain_records),
        }

    # Save manifest
    with open(output_path / "manifest.json", "w") as f:
        json.dump(manifest, f, indent=2)

    print(f"\nMini-dataset specialization complete:")
    print(f"  Agents: {total_created}")
    print(f"  Prompts/agent: {prompts_per_agent}")
    print(f"  Domains: {len(domains)}")
    print(f"  Output: {output_path}")

    return output_path


# === Business Dataset Generator (200 agents) ===

def generate_business_datasets(
    input_path: str,
    num_agents: int = 200,
    prompts_per_agent: int = 750,
    output_dir: str = "data/business/",
) -> Path:
    """Create full datasets for 200 business agents.

    These agents use Gemini Pro / Claude via RouteLLM routing.
    Larger datasets (500-1000 prompts) for better generalization.
    """
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    records = []
    with open(input_path) as f:
        for line in f:
            records.append(json.loads(line.strip()))

    if not records:
        print("No records found. Run 'scan' first.")
        return output_path

    domains = [
        "startups", "tech_web_dev", "customer_support",
        "sales", "hr", "marketing",
        "ecommerce", "project_management", "legal"
    ]

    agents_per_domain = num_agents // len(domains)
    total_created = 0

    for i, domain in enumerate(domains):
        domain_dir = output_path / domain
        domain_dir.mkdir(exist_ok=True)

        for agent_idx in range(agents_per_domain):
            agent_id = f"biz-{domain}-{agent_idx:04d}"
            agent_file = domain_dir / f"{agent_id}.jsonl"

            with open(agent_file, "w") as f:
                for p in range(min(prompts_per_agent, len(records))):
                    record = records[p % len(records)]
                    content = record["content"][:1500]

                    training_example = {
                        "instruction": f"[{domain.upper()}] Provide expert analysis: {content[:200]}",
                        "output": f"Expert {domain} analysis:\n\n{content}",
                        "domain": domain,
                        "agent_id": agent_id,
                        "tier": "business",
                    }
                    f.write(json.dumps(training_example) + "\n")

            total_created += 1

    print(f"\nBusiness datasets complete: {total_created} agents, {prompts_per_agent} prompts each")
    return output_path


# === Full Pipeline ===

def full_pipeline(input_dir: str, output_dir: str):
    """Full pipeline: scan → specialize → business datasets."""
    print("=" * 60)
    print("  APOPHY SOVEREIGN — Dataset Processor Pipeline")
    print("=" * 60)

    out = Path(output_dir)

    # Step 1: Scan
    print("\n[1/3] Scanning input files (PDF, CSV, MD)...")
    dataset_file = scan_directory(input_dir, str(out / "extracted"))

    # Step 2: Mini-datasets for 3000 micro-agents
    print("\n[2/3] Generating mini-datasets for 3000 micro-agents...")
    specialize_mini_datasets(str(dataset_file), 3000, 75, str(out / "mini"))

    # Step 3: Business datasets for 200 agents
    print("\n[3/3] Generating business datasets for 200 agents...")
    generate_business_datasets(str(dataset_file), 200, 750, str(out / "business"))

    print("\n" + "=" * 60)
    print("  PIPELINE COMPLETE")
    print(f"  Output: {out}")
    print("=" * 60)


# === CLI ===

def main():
    parser = argparse.ArgumentParser(description="Apophy Sovereign — Dataset Processor")
    subparsers = parser.add_subparsers(dest="command")

    scan = subparsers.add_parser("scan", help="Scan PDF/CSV/MD files")
    scan.add_argument("--input", required=True, help="Input directory")
    scan.add_argument("--output", default="data/datasets/", help="Output directory")

    spec = subparsers.add_parser("specialize", help="Generate mini-datasets for micro-agents")
    spec.add_argument("--input", required=True, help="Path to extracted JSONL")
    spec.add_argument("--agents", type=int, default=3000)
    spec.add_argument("--prompts", type=int, default=75)
    spec.add_argument("--output", default="data/mini/")

    biz = subparsers.add_parser("business", help="Generate business agent datasets")
    biz.add_argument("--input", required=True, help="Path to extracted JSONL")
    biz.add_argument("--agents", type=int, default=200)
    biz.add_argument("--prompts", type=int, default=750)
    biz.add_argument("--output", default="data/business/")

    pipe = subparsers.add_parser("pipeline", help="Full pipeline")
    pipe.add_argument("--input", required=True, help="Input directory with raw files")
    pipe.add_argument("--output", default="data/", help="Output root directory")

    args = parser.parse_args()

    if args.command == "scan":
        scan_directory(args.input, args.output)
    elif args.command == "specialize":
        specialize_mini_datasets(args.input, args.agents, args.prompts, args.output)
    elif args.command == "business":
        generate_business_datasets(args.input, args.agents, args.prompts, args.output)
    elif args.command == "pipeline":
        full_pipeline(args.input, args.output)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
