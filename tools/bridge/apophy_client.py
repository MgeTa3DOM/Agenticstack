#!/usr/bin/env python3
"""
Apophy Memory Bridge — Python SDK Client
=========================================

Python ↔ Rust bridge for the Apophy Sovereign Memory Palace.
Wraps the REST API endpoints exposed by apophy-sovereign to provide
bidirectional data flow between the Python neural pipeline and the
Rust memory engine.

Usage:
    from apophy_client import ApophyMemoryClient

    client = ApophyMemoryClient("http://localhost:8080")

    # Identity
    identity = client.get_identity("Apophy")

    # Semantic key-value beliefs
    client.learn("preferred_language", "Rust", confidence=1.0)
    result = client.recall("preferred_language")

    # Episodic memories
    client.remember("First training run completed", valence=0.8, importance=0.9)
    episodes = client.episodes(limit=10)

    # Semantic facts with embeddings (for vector search)
    client.store_fact("tech", "Rust is memory-safe", embedding=[0.1, 0.2, ...])
    results = client.search_facts(embedding=[0.1, 0.2, ...], domain="tech")

    # Bulk operations for training pipelines
    client.sync_training_context(domain="reasoning", facts=["..."])
"""

import json
import time
from typing import Any

import requests


class ApophyMemoryClient:
    """Python SDK for the Apophy Sovereign Memory Palace REST API."""

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        identity_name: str = "Apophy",
        timeout: int = 30,
        max_retries: int = 3,
    ):
        self.base_url = base_url.rstrip("/")
        self.identity_name = identity_name
        self.timeout = timeout
        self.max_retries = max_retries
        self.session = requests.Session()
        self.session.headers.update({"Content-Type": "application/json"})

    def _request(self, method: str, path: str, **kwargs) -> Any:
        """Make an HTTP request with retry logic."""
        url = f"{self.base_url}{path}"
        for attempt in range(self.max_retries):
            try:
                resp = self.session.request(method, url, timeout=self.timeout, **kwargs)
                resp.raise_for_status()
                return resp.json() if resp.content else None
            except requests.exceptions.ConnectionError:
                if attempt < self.max_retries - 1:
                    time.sleep(2 ** attempt)
                else:
                    raise
            except requests.exceptions.HTTPError as e:
                if resp.status_code == 404:
                    return None
                raise

    # === Identity ===

    def get_identity(self, name: str | None = None) -> dict | None:
        """Get or auto-create an identity by name."""
        params = {"name": name or self.identity_name}
        return self._request("GET", "/api/v1/memory/identity", params=params)

    # === Semantic Key-Value Beliefs ===

    def learn(self, key: str, value: str, confidence: float = 1.0, identity_name: str | None = None) -> dict:
        """Store or update a semantic belief (key → value with confidence)."""
        return self._request("POST", "/api/v1/memory/learn", json={
            "identity_name": identity_name or self.identity_name,
            "key": key,
            "value": value,
            "confidence": confidence,
        })

    def recall(self, key: str, identity_name: str | None = None) -> dict | None:
        """Recall a semantic belief by key."""
        return self._request("POST", "/api/v1/memory/recall", json={
            "identity_name": identity_name or self.identity_name,
            "key": key,
        })

    def list_semantic(self, limit: int = 100, identity_name: str | None = None) -> list[dict]:
        """List all semantic memories for an identity."""
        return self._request("POST", "/api/v1/memory/semantic/list", json={
            "identity_name": identity_name or self.identity_name,
            "limit": limit,
        }) or []

    # === Episodic Memories ===

    def remember(
        self,
        content: str,
        valence: float = 0.0,
        importance: float = 0.5,
        tags: list[str] | None = None,
        identity_name: str | None = None,
    ) -> dict:
        """Store an episodic memory with emotional context."""
        return self._request("POST", "/api/v1/memory/episodes", json={
            "identity_name": identity_name or self.identity_name,
            "content": content,
            "valence": valence,
            "importance": importance,
            "tags": tags or [],
        })

    def episodes(self, limit: int = 20, identity_name: str | None = None) -> list[dict]:
        """Recall the most important episodic memories."""
        return self._request("POST", "/api/v1/memory/episodes", json={
            "identity_name": identity_name or self.identity_name,
            "limit": limit,
        }) or []

    # === Semantic Facts with Embeddings ===

    def store_fact(
        self,
        domain: str,
        content: str,
        embedding: list[float] | None = None,
        source: str = "python",
        identity_name: str | None = None,
    ) -> dict:
        """Store a semantic fact with optional embedding vector."""
        return self._request("POST", "/api/v1/memory/facts/store", json={
            "identity_name": identity_name or self.identity_name,
            "domain": domain,
            "content": content,
            "embedding": embedding,
            "source": source,
        })

    def search_facts(
        self,
        embedding: list[float],
        domain: str | None = None,
        limit: int = 10,
        min_similarity: float = 0.5,
        identity_name: str | None = None,
    ) -> dict:
        """Search semantic facts by cosine similarity against an embedding."""
        return self._request("POST", "/api/v1/memory/facts/search", json={
            "identity_name": identity_name or self.identity_name,
            "embedding": embedding,
            "domain": domain,
            "limit": limit,
            "min_similarity": min_similarity,
        })

    def list_facts(
        self,
        domain: str | None = None,
        limit: int = 50,
        identity_name: str | None = None,
    ) -> dict:
        """List semantic facts, optionally filtered by domain."""
        return self._request("POST", "/api/v1/memory/facts/list", json={
            "identity_name": identity_name or self.identity_name,
            "domain": domain,
            "limit": limit,
        })

    # === Bulk Operations for Training Pipeline ===

    def sync_training_context(
        self,
        domain: str,
        facts: list[str],
        embeddings: list[list[float]] | None = None,
        source: str = "trainer",
    ) -> dict:
        """Bulk-store training facts into the memory palace.

        If embeddings are provided, they must match facts 1:1.
        Returns summary of stored facts.
        """
        stored = 0
        errors = 0
        for i, content in enumerate(facts):
            emb = embeddings[i] if embeddings and i < len(embeddings) else None
            try:
                self.store_fact(domain, content, embedding=emb, source=source)
                stored += 1
            except Exception:
                errors += 1
        return {"stored": stored, "errors": errors, "total": len(facts)}

    def export_training_data(
        self,
        domain: str | None = None,
        limit: int = 10000,
        output_path: str | None = None,
    ) -> list[dict]:
        """Export memory facts as training data (JSONL-compatible dicts).

        Combines semantic beliefs + episodic memories + semantic facts
        into a unified training format.
        """
        entries = []

        # Semantic beliefs → instruction/output pairs
        beliefs = self.list_semantic(limit=limit)
        for belief in beliefs:
            entries.append({
                "instruction": f"What is {belief.get('key', '')}?",
                "output": belief.get("value", ""),
                "domain": "beliefs",
                "source": "memory_palace",
                "confidence": belief.get("confidence", 1.0),
            })

        # Semantic facts
        facts_resp = self.list_facts(domain=domain, limit=limit)
        if facts_resp and "facts" in facts_resp:
            for fact in facts_resp["facts"]:
                entries.append({
                    "instruction": f"Tell me about: {fact.get('domain', 'general')}",
                    "output": fact.get("content", ""),
                    "domain": fact.get("domain", "general"),
                    "source": fact.get("source", "memory_palace"),
                    "confidence": 1.0,
                })

        # Episodic memories
        eps = self.episodes(limit=limit)
        for ep in eps:
            if isinstance(ep, dict):
                entries.append({
                    "instruction": "Recall an important memory.",
                    "output": ep.get("content", ""),
                    "domain": "episodic",
                    "source": "memory_palace",
                    "confidence": ep.get("importance", 0.5),
                })

        # Write to file if requested
        if output_path:
            with open(output_path, "w") as f:
                for entry in entries:
                    f.write(json.dumps(entry) + "\n")

        return entries

    # === Health Check ===

    def health(self) -> dict:
        """Check if the Sovereign server is running."""
        return self._request("GET", "/health")

    def is_alive(self) -> bool:
        """Quick connectivity check."""
        try:
            resp = self.health()
            return resp is not None
        except Exception:
            return False


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Apophy Memory Bridge CLI")
    parser.add_argument("--url", default="http://localhost:8080", help="Sovereign API URL")
    parser.add_argument("--identity", default="Apophy", help="Identity name")
    sub = parser.add_subparsers(dest="command")

    sub.add_parser("health", help="Check server health")
    sub.add_parser("identity", help="Get identity info")

    learn_cmd = sub.add_parser("learn", help="Store a belief")
    learn_cmd.add_argument("key")
    learn_cmd.add_argument("value")
    learn_cmd.add_argument("--confidence", type=float, default=1.0)

    recall_cmd = sub.add_parser("recall", help="Recall a belief")
    recall_cmd.add_argument("key")

    remember_cmd = sub.add_parser("remember", help="Store an episode")
    remember_cmd.add_argument("content")
    remember_cmd.add_argument("--valence", type=float, default=0.5)
    remember_cmd.add_argument("--importance", type=float, default=0.7)

    episodes_cmd = sub.add_parser("episodes", help="List episodes")
    episodes_cmd.add_argument("--limit", type=int, default=10)

    export_cmd = sub.add_parser("export", help="Export training data")
    export_cmd.add_argument("--output", default="data/memory_export.jsonl")
    export_cmd.add_argument("--domain", default=None)

    args = parser.parse_args()
    client = ApophyMemoryClient(args.url, args.identity)

    if args.command == "health":
        print(json.dumps(client.health(), indent=2))
    elif args.command == "identity":
        print(json.dumps(client.get_identity(), indent=2))
    elif args.command == "learn":
        result = client.learn(args.key, args.value, args.confidence)
        print(json.dumps(result, indent=2))
    elif args.command == "recall":
        result = client.recall(args.key)
        print(json.dumps(result, indent=2))
    elif args.command == "remember":
        result = client.remember(args.content, args.valence, args.importance)
        print(json.dumps(result, indent=2))
    elif args.command == "episodes":
        result = client.episodes(args.limit)
        print(json.dumps(result, indent=2))
    elif args.command == "export":
        entries = client.export_training_data(domain=args.domain, output_path=args.output)
        print(f"Exported {len(entries)} training entries to {args.output}")
    else:
        parser.print_help()
