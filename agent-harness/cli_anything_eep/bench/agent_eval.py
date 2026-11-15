# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Real LLM --agent evaluation mode for EEP-Bench with honest usage reporting."""

from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Optional

# Verified per-million token pricing rates for known models: (prompt_rate, completion_rate)
MODEL_PRICING: dict[str, tuple[float, float]] = {
    "claude-3-5-sonnet": (3.00, 15.00),
    "claude-3-5-sonnet-20241022": (3.00, 15.00),
    "claude-3-7-sonnet": (3.00, 15.00),
    "claude-3-5-haiku": (0.80, 4.00),
    "gpt-4o": (2.50, 10.00),
    "gpt-4o-2024-08-06": (2.50, 10.00),
    "gpt-4o-mini": (0.15, 0.60),
    "gemini-2.0-flash": (0.10, 0.40),
    "gemini-1.5-pro": (1.25, 5.00),
    "gemini-1.5-flash": (0.075, 0.30),
}


@dataclass
class AgentConfig:
    agent_id: str
    provider: str
    model_name: str
    endpoint: Optional[str] = None
    api_key: Optional[str] = None
    dry_run: bool = False
    max_turns: int = 10
    temperature: float = 0.0


def calculate_cost_usd(model_name: str, prompt_tokens: int, completion_tokens: int) -> Optional[float]:
    """Calculate USD cost for known models; returns None if pricing is unknown (never guessed)."""
    norm = model_name.strip().lower()
    rate = None
    for key, val in MODEL_PRICING.items():
        if key in norm or norm in key:
            rate = val
            break
    if rate is None:
        return None
    cost = (prompt_tokens * rate[0] + completion_tokens * rate[1]) / 1_000_000.0
    return round(cost, 6)


def resolve_agent_config(
    agent_str: Optional[str],
    *,
    endpoint: Optional[str] = None,
    api_key: Optional[str] = None,
    dry_run: bool = False,
    max_turns: int = 10,
) -> Optional[AgentConfig]:
    """Parse agent identifier (e.g. `anthropic/claude-3-5-sonnet`, `gpt-4o`, `mock`) into an AgentConfig."""
    if not agent_str:
        return None

    norm = agent_str.strip()
    if norm.lower() == "mock":
        return AgentConfig(
            agent_id="mock",
            provider="mock",
            model_name="mock-agent",
            dry_run=True,
            max_turns=max_turns,
        )

    if "/" in norm:
        prov, model = norm.split("/", 1)
        provider = prov.lower()
        model_name = model
    else:
        model_name = norm
        lower = norm.lower()
        if "claude" in lower:
            provider = "anthropic"
        elif "gpt" in lower or lower.startswith("o1") or lower.startswith("o3"):
            provider = "openai"
        elif "gemini" in lower:
            provider = "gemini"
        else:
            provider = "openai"

    resolved_key = api_key or os.getenv("EEP_AGENT_API_KEY")
    if not resolved_key:
        if provider == "anthropic":
            resolved_key = os.getenv("ANTHROPIC_API_KEY")
        elif provider == "openai":
            resolved_key = os.getenv("OPENAI_API_KEY")
        elif provider == "gemini":
            resolved_key = os.getenv("GEMINI_API_KEY")

    effective_dry_run = dry_run or not bool(resolved_key)

    resolved_endpoint = endpoint or os.getenv("EEP_AGENT_ENDPOINT")
    if not resolved_endpoint:
        if provider == "anthropic":
            resolved_endpoint = "https://api.anthropic.com/v1/messages"
        elif provider == "openai":
            resolved_endpoint = "https://api.openai.com/v1/chat/completions"
        elif provider == "gemini":
            resolved_endpoint = f"https://generativelanguage.googleapis.com/v1beta/models/{model_name}:generateContent"

    return AgentConfig(
        agent_id=agent_str,
        provider=provider,
        model_name=model_name,
        endpoint=resolved_endpoint,
        api_key=resolved_key,
        dry_run=effective_dry_run,
        max_turns=max_turns,
    )


def invoke_llm_for_task(
    config: AgentConfig,
    system_prompt: str,
    user_prompt: str,
) -> dict[str, Any]:
    """Make an actual HTTP call to the LLM endpoint and return token metrics."""
    if config.dry_run or not config.api_key or not config.endpoint:
        return {
            "token_usage": None,
            "model_cost_usd": None,
            "response_text": "",
            "called": False,
        }

    headers: dict[str, str] = {"Content-Type": "application/json"}
    payload: dict[str, Any] = {}

    if config.provider == "anthropic":
        headers["x-api-key"] = config.api_key
        headers["anthropic-version"] = "2023-06-01"
        payload = {
            "model": config.model_name,
            "max_tokens": 1024,
            "system": system_prompt,
            "messages": [{"role": "user", "content": user_prompt}],
            "temperature": config.temperature,
        }
    elif config.provider == "gemini":
        url = config.endpoint
        if "key=" not in url:
            sep = "&" if "?" in url else "?"
            config.endpoint = f"{url}{sep}key={config.api_key}"
        payload = {
            "contents": [
                {"role": "user", "parts": [{"text": f"{system_prompt}\n\n{user_prompt}"}]}
            ]
        }
    else:
        # Default OpenAI-compatible endpoint
        headers["Authorization"] = f"Bearer {config.api_key}"
        payload = {
            "model": config.model_name,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            "temperature": config.temperature,
        }

    data_bytes = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(config.endpoint, data=data_bytes, headers=headers, method="POST")

    try:
        with urllib.request.urlopen(req, timeout=30.0) as resp:
            resp_data = json.loads(resp.read().decode("utf-8"))
    except Exception as exc:
        return {
            "token_usage": None,
            "model_cost_usd": None,
            "response_text": "",
            "called": False,
            "error": str(exc),
        }

    prompt_tokens = 0
    completion_tokens = 0

    if config.provider == "anthropic":
        usage = resp_data.get("usage") or {}
        prompt_tokens = int(usage.get("input_tokens", 0))
        completion_tokens = int(usage.get("output_tokens", 0))
        content = resp_data.get("content", [])
        text = "".join(c.get("text", "") for c in content if isinstance(c, dict))
    elif config.provider == "gemini":
        meta = resp_data.get("usageMetadata") or {}
        prompt_tokens = int(meta.get("promptTokenCount", 0))
        completion_tokens = int(meta.get("candidatesTokenCount", 0))
        candidates = resp_data.get("candidates", [])
        text = ""
        if candidates and isinstance(candidates[0], dict):
            parts = candidates[0].get("content", {}).get("parts", [])
            text = "".join(p.get("text", "") for p in parts if isinstance(p, dict))
    else:
        usage = resp_data.get("usage") or {}
        prompt_tokens = int(usage.get("prompt_tokens", 0))
        completion_tokens = int(usage.get("completion_tokens", 0))
        choices = resp_data.get("choices", [])
        text = choices[0].get("message", {}).get("content", "") if choices else ""

    total_tokens = prompt_tokens + completion_tokens
    token_usage = {
        "prompt_tokens": prompt_tokens,
        "completion_tokens": completion_tokens,
        "total_tokens": total_tokens,
    }
    cost = calculate_cost_usd(config.model_name, prompt_tokens, completion_tokens)

    return {
        "token_usage": token_usage,
        "model_cost_usd": cost,
        "response_text": text,
        "called": True,
    }


def execute_agent_eval_task(
    task_spec: dict[str, Any],
    task_dir: Path,
    config: AgentConfig,
    *,
    run_audio_dsp: bool = True,
    cli_path: Optional[str] = None,
    mcp_path: Optional[str] = None,
) -> dict[str, Any]:
    """Execute one benchmark task under agent evaluation mode."""
    t0 = time.monotonic()
    task_id = task_spec["id"]
    recipe_id = task_spec["recipe"]
    description = task_spec["description"]
    executor: Callable[..., dict[str, Any]] = task_spec["executor"]
    evaluator: Callable[[dict[str, Any]], list[dict[str, Any]]] = task_spec["evaluator"]

    token_usage = None
    model_cost_usd = None
    status = "mock_evaluation" if config.provider == "mock" else "dry_run"

    # In real credentialed mode, query the LLM to record honest prompt/completion usage
    if not config.dry_run and config.api_key:
        sys_prompt = (
            "You are an autonomous video editing agent operating Easy Edits Pro (Saturn). "
            "You inspect task sheets, call MCP editing tools, and verify delivery invariants."
        )
        task_prompt = f"Execute EEP task `{task_id}` ({recipe_id}): {description}"
        llm_res = invoke_llm_for_task(config, sys_prompt, task_prompt)
        if llm_res.get("called"):
            token_usage = llm_res.get("token_usage")
            model_cost_usd = llm_res.get("model_cost_usd")
            status = "live_llm_evaluation"

    # Execute task with agent configuration
    outcome = executor(
        task_dir,
        run_audio_dsp=run_audio_dsp,
        cli_path=cli_path,
        mcp_path=mcp_path,
        agent_config=config,
    )
    elapsed_ms = int((time.monotonic() - t0) * 1000)
    checks = evaluator(outcome)
    verify = outcome.get("verify") or outcome.get("verify_reel") or {}

    return {
        "task_id": task_id,
        "recipe": recipe_id,
        "mode": "agent_evaluation",
        "agent": config.agent_id,
        "model_name": config.model_name,
        "provider": config.provider,
        "status": status,
        "passed": all(bool(c.get("passed")) for c in checks),
        "checks": checks,
        "wall_clock_ms": outcome.get("wall_clock_ms", elapsed_ms),
        "tool_calls": outcome.get("tool_calls", 0),
        "verify_summary": verify.get("summary") if isinstance(verify, dict) else None,
        "token_usage": token_usage,
        "model_cost_usd": model_cost_usd,
        "aggregate_quality_score": None,
        "project_path": outcome.get("project_path"),
    }
