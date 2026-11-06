from __future__ import annotations

import csv
import math
import re
import statistics
import threading
import time
from collections.abc import Callable
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

NETWORK_GENESIS_SLOT = {
    "preprod": 10_718_513,
    "mainnet": 71_482_583,
}

_MEMORY_PATTERN = re.compile(r"^\s*([0-9]+(?:\.[0-9]+)?)\s*([kmgt]?i?b)\s*$", re.I)
_MEMORY_MULTIPLIERS = {
    "b": 1,
    "kb": 1_000,
    "mb": 1_000**2,
    "gb": 1_000**3,
    "tb": 1_000**4,
    "kib": 1_024,
    "mib": 1_024**2,
    "gib": 1_024**3,
    "tib": 1_024**4,
}


@dataclass(frozen=True, slots=True)
class ResourceSample:
    elapsed_seconds: float
    service: str
    cpu_percent: float
    memory_bytes: int
    memory_percent: float
    pids: int


@dataclass(frozen=True, slots=True)
class CursorSample:
    elapsed_seconds: float
    baseline_cursor: int
    candidate_cursor: int


def parse_percent(value: str) -> float:
    return float(value.strip().removesuffix("%"))


def parse_memory_bytes(value: str) -> int:
    match = _MEMORY_PATTERN.fullmatch(value)
    if match is None:
        raise ValueError(f"unsupported Docker memory value: {value}")
    amount, unit = match.groups()
    return round(float(amount) * _MEMORY_MULTIPLIERS[unit.lower()])


def _nearest_rank(values: list[float], percentile: float) -> float:
    if not values:
        raise ValueError("cannot calculate a percentile without samples")
    ordered = sorted(values)
    index = max(0, math.ceil(percentile * len(ordered)) - 1)
    return ordered[index]


def _ratio(candidate: float | int | None, baseline: float | int | None) -> float | None:
    if candidate is None or baseline in (None, 0):
        return None
    return round(float(candidate) / float(baseline), 4)


class MetricsRecorder:
    def __init__(
        self,
        scan_started_monotonic: float,
        clock: Callable[[], float] = time.monotonic,
    ) -> None:
        self.scan_started_monotonic = scan_started_monotonic
        self._clock = clock
        self._resources: list[ResourceSample] = []
        self._cursors: list[CursorSample] = []
        self._lock = threading.Lock()

    def elapsed(self) -> float:
        return self._clock() - self.scan_started_monotonic

    def record_resource(
        self,
        service: str,
        cpu_percent: float,
        memory_bytes: int,
        memory_percent: float,
        pids: int,
    ) -> None:
        sample = ResourceSample(
            elapsed_seconds=round(self.elapsed(), 3),
            service=service,
            cpu_percent=cpu_percent,
            memory_bytes=memory_bytes,
            memory_percent=memory_percent,
            pids=pids,
        )
        with self._lock:
            self._resources.append(sample)

    def record_cursors(self, baseline_cursor: int, candidate_cursor: int) -> None:
        sample = CursorSample(
            elapsed_seconds=round(self.elapsed(), 3),
            baseline_cursor=baseline_cursor,
            candidate_cursor=candidate_cursor,
        )
        with self._lock:
            self._cursors.append(sample)

    def write_samples(self, output_dir: Path) -> None:
        with self._lock:
            resources = list(self._resources)
            cursors = list(self._cursors)
        self._write_csv(
            output_dir / "resource-samples.csv",
            resources,
            tuple(ResourceSample.__dataclass_fields__),
        )
        self._write_csv(
            output_dir / "cursor-samples.csv",
            cursors,
            tuple(CursorSample.__dataclass_fields__),
        )

    @staticmethod
    def _write_csv(
        path: Path,
        rows: list[ResourceSample] | list[CursorSample],
        fields: tuple[str, ...],
    ) -> None:
        with path.open("w", encoding="utf-8", newline="") as output:
            writer = csv.DictWriter(output, fieldnames=fields, lineterminator="\n")
            writer.writeheader()
            for row in rows:
                writer.writerow(asdict(row))

    def summarize(
        self,
        *,
        network: str,
        required_cursor: int,
        total_duration_seconds: float,
        scan_duration_seconds: float,
        restart_counts: dict[str, int],
    ) -> dict[str, Any]:
        with self._lock:
            resources = list(self._resources)
            cursors = list(self._cursors)
        cursor_metrics = {
            service: self._cursor_summary(
                service, cursors, NETWORK_GENESIS_SLOT[network], required_cursor
            )
            for service in ("baseline", "candidate")
        }
        resource_metrics = {
            service: self._resource_summary(
                [sample for sample in resources if sample.service == service],
                restart_counts.get(service),
            )
            for service in ("baseline", "candidate")
        }
        return {
            "measurement_scope": (
                "diagnostic; resource values are host- and Docker-runtime-dependent"
            ),
            "durations": {
                "total_run_seconds": round(total_duration_seconds, 3),
                "scan_seconds": round(scan_duration_seconds, 3),
            },
            "cursor": {
                "required_cursor": required_cursor,
                "network_genesis_slot": NETWORK_GENESIS_SLOT[network],
                **cursor_metrics,
                "candidate_to_baseline": {
                    "time_to_boundary_ratio": _ratio(
                        cursor_metrics["candidate"]["time_to_boundary_seconds"],
                        cursor_metrics["baseline"]["time_to_boundary_seconds"],
                    ),
                    "effective_slots_per_second_ratio": _ratio(
                        cursor_metrics["candidate"]["effective_slots_per_second"],
                        cursor_metrics["baseline"]["effective_slots_per_second"],
                    ),
                },
            },
            "resources": {
                **resource_metrics,
                "candidate_to_baseline": {
                    "mean_cpu_ratio": _ratio(
                        resource_metrics["candidate"]["mean_cpu_percent"],
                        resource_metrics["baseline"]["mean_cpu_percent"],
                    ),
                    "peak_cpu_ratio": _ratio(
                        resource_metrics["candidate"]["peak_cpu_percent"],
                        resource_metrics["baseline"]["peak_cpu_percent"],
                    ),
                    "mean_memory_ratio": _ratio(
                        resource_metrics["candidate"]["mean_memory_bytes"],
                        resource_metrics["baseline"]["mean_memory_bytes"],
                    ),
                    "peak_memory_ratio": _ratio(
                        resource_metrics["candidate"]["peak_memory_bytes"],
                        resource_metrics["baseline"]["peak_memory_bytes"],
                    ),
                },
            },
        }

    @staticmethod
    def _cursor_summary(
        service: str,
        samples: list[CursorSample],
        genesis_slot: int,
        required_cursor: int,
    ) -> dict[str, float | None]:
        values = [
            (sample.elapsed_seconds, getattr(sample, f"{service}_cursor"))
            for sample in samples
        ]
        first = next((elapsed for elapsed, cursor in values if cursor > 0), None)
        boundary = next(
            (elapsed for elapsed, cursor in values if cursor >= required_cursor), None
        )
        scanned_slots = max(0, required_cursor - genesis_slot)
        throughput = (
            round(scanned_slots / boundary, 3)
            if boundary is not None and boundary > 0
            else None
        )
        return {
            "time_to_initial_cursor_seconds": first,
            "time_to_boundary_seconds": boundary,
            "effective_slots_per_second": throughput,
        }

    @staticmethod
    def _resource_summary(
        samples: list[ResourceSample], restart_count: int | None
    ) -> dict[str, float | int | None]:
        if not samples:
            return {
                "sample_count": 0,
                "mean_cpu_percent": None,
                "p95_cpu_percent": None,
                "peak_cpu_percent": None,
                "mean_memory_bytes": None,
                "p95_memory_bytes": None,
                "peak_memory_bytes": None,
                "peak_memory_percent": None,
                "peak_pids": None,
                "restart_count": restart_count,
            }
        cpu = [sample.cpu_percent for sample in samples]
        memory = [sample.memory_bytes for sample in samples]
        return {
            "sample_count": len(samples),
            "mean_cpu_percent": round(statistics.fmean(cpu), 3),
            "p95_cpu_percent": round(_nearest_rank(cpu, 0.95), 3),
            "peak_cpu_percent": round(max(cpu), 3),
            "mean_memory_bytes": round(statistics.fmean(memory)),
            "p95_memory_bytes": round(_nearest_rank([float(v) for v in memory], 0.95)),
            "peak_memory_bytes": max(memory),
            "peak_memory_percent": round(
                max(sample.memory_percent for sample in samples), 3
            ),
            "peak_pids": max(sample.pids for sample in samples),
            "restart_count": restart_count,
        }
