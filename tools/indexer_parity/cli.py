from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, NoReturn

from .manifest import compare, read_manifest, write_manifest, write_reports
from .metrics import MetricsRecorder, parse_memory_bytes, parse_percent

REPO_ROOT = Path(__file__).resolve().parents[2]
COMPOSE_FILE = REPO_ROOT / "docker" / "indexer-parity" / "compose.yml"
DEFAULT_BASELINE_IMAGE = "hyperledgeridentus/identus-neoprism:0.14.2"
DEFAULT_PREPROD_RELAY = "preprod-node.play.dev.cardano.org:3001"
DEFAULT_MAINNET_RELAY = "backbone.mainnet.cardanofoundation.org:3001"
PROJECT_NAME = "neoprism-indexer-parity"
POLL_SECONDS = 5

MANIFEST_QUERY = """
COPY (
    SELECT
        encode(ro.operation_id, 'hex') AS operation_id,
        encode(ro.tx_hash, 'hex') AS tx_hash,
        encode(ro.signed_operation_data, 'hex') AS signed_operation_data,
        ro.slot,
        ro.block_number,
        ro.absn,
        ro.osn,
        CASE WHEN ro.is_indexed THEN 'true' ELSE 'false' END AS indexed,
        CASE
            WHEN ssi.raw_operation_id IS NOT NULL THEN 'ssi'
            WHEN vdr.raw_operation_id IS NOT NULL THEN 'vdr'
            ELSE 'none'
        END AS operation_kind,
        COALESCE(encode(ssi.did, 'hex'), encode(vdr.did, 'hex'), '') AS did
    FROM raw_operation AS ro
    LEFT JOIN indexed_ssi_operation AS ssi ON ssi.raw_operation_id = ro.id
    LEFT JOIN indexed_vdr_operation AS vdr ON vdr.raw_operation_id = ro.id
    WHERE ro.slot <= {comparison_slot}
    ORDER BY ro.operation_id
) TO STDOUT WITH (FORMAT CSV, HEADER TRUE)
"""


class HarnessError(RuntimeError):
    """Raised when the live parity harness cannot produce a comparison."""


@dataclass(frozen=True, slots=True)
class RunOptions:
    baseline_image: str
    candidate_image: str
    network: str
    relay_address: str
    target_slot: int | None
    sample_seconds: int
    boundary_lag: int
    timeout_seconds: int
    metrics_sample_seconds: int
    output_dir: Path
    keep: bool


def _run(
    command: list[str],
    *,
    env: dict[str, str] | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=REPO_ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    if check and result.returncode != 0:
        rendered = shlex.join(command)
        raise HarnessError(
            f"command failed ({result.returncode}): {rendered}\n{result.stderr.strip()}"
        )
    return result


class ComposeHarness:
    def __init__(self, options: RunOptions) -> None:
        self.options = options
        self.environment = os.environ.copy()
        self.environment.update(
            {
                "NEOPRISM_BASELINE_IMAGE": options.baseline_image,
                "NEOPRISM_CANDIDATE_IMAGE": options.candidate_image,
                "NEOPRISM_PARITY_NETWORK": options.network,
                "NEOPRISM_PARITY_RELAY_ADDR": options.relay_address,
            }
        )
        self.command_prefix = [
            "docker",
            "compose",
            "--project-name",
            PROJECT_NAME,
            "--file",
            str(COMPOSE_FILE),
        ]

    def compose(
        self, *arguments: str, check: bool = True
    ) -> subprocess.CompletedProcess[str]:
        return _run(
            [*self.command_prefix, *arguments],
            env=self.environment,
            check=check,
        )

    def query(self, database_service: str, sql: str) -> str:
        result = self.compose(
            "exec",
            "-T",
            database_service,
            "psql",
            "--username",
            "postgres",
            "--dbname",
            "postgres",
            "--tuples-only",
            "--no-align",
            "--command",
            sql,
        )
        return result.stdout.strip()

    def cursor(self, database_service: str) -> int:
        value = self.query(
            database_service, "SELECT COALESCE(MAX(slot), 0) FROM dlt_cursor"
        )
        return int(value)

    def operation_state(
        self, database_service: str, comparison_slot: int
    ) -> tuple[int, int]:
        value = self.query(
            database_service,
            "SELECT COUNT(*), COUNT(*) FILTER (WHERE is_indexed = false) "
            f"FROM raw_operation WHERE slot <= {comparison_slot}",
        )
        count, unindexed = value.split("|")
        return int(count), int(unindexed)

    def export_manifest(self, database_service: str, comparison_slot: int) -> str:
        return self.query(
            database_service,
            MANIFEST_QUERY.format(comparison_slot=comparison_slot),
        )

    def image_identity(self, image: str) -> str:
        result = _run(
            [
                "docker",
                "image",
                "inspect",
                "--format",
                '{{.Id}} {{join .RepoDigests ","}}',
                image,
            ],
            check=False,
        )
        if result.returncode != 0:
            _run(["docker", "pull", image])
            result = _run(
                [
                    "docker",
                    "image",
                    "inspect",
                    "--format",
                    '{{.Id}} {{join .RepoDigests ","}}',
                    image,
                ]
            )
        return result.stdout.strip()

    def container_id(self, service: str) -> str:
        result = self.compose("ps", "--quiet", service)
        container_id = result.stdout.strip()
        if not container_id:
            raise HarnessError(f"unable to resolve container for service {service}")
        return container_id

    def restart_count(self, container_id: str) -> int:
        result = _run(
            ["docker", "inspect", "--format", "{{.RestartCount}}", container_id]
        )
        return int(result.stdout.strip())

    def write_logs(self) -> None:
        logs = self.compose("logs", "--no-color", check=False)
        (self.options.output_dir / "containers.log").write_text(
            logs.stdout + logs.stderr,
            encoding="utf-8",
        )

    def cleanup(self) -> None:
        self.compose("down", "--volumes", "--remove-orphans", check=False)


class ResourceSampler:
    def __init__(
        self,
        harness: ComposeHarness,
        recorder: MetricsRecorder,
        interval_seconds: int,
    ) -> None:
        self.harness = harness
        self.recorder = recorder
        self.interval_seconds = interval_seconds
        self.container_ids: dict[str, str] = {}
        self.warnings: list[str] = []
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        self.container_ids = {
            service: self.harness.container_id(service)
            for service in ("baseline", "candidate")
        }
        self._thread = threading.Thread(
            target=self._run,
            name="indexer-parity-resource-sampler",
            daemon=True,
        )
        self._thread.start()

    def _run(self) -> None:
        while not self._stop.is_set():
            sample_started = time.monotonic()
            self._sample_all()
            sample_duration = time.monotonic() - sample_started
            self._stop.wait(max(0.0, self.interval_seconds - sample_duration))

    def _sample_all(self) -> None:
        result = _run(
            [
                "docker",
                "stats",
                "--no-stream",
                "--format",
                "{{.ID}}|{{.CPUPerc}}|{{.MemUsage}}|{{.MemPerc}}|{{.PIDs}}",
                *self.container_ids.values(),
            ],
            check=False,
        )
        if result.returncode != 0:
            warning = f"Docker stats failed: {result.stderr.strip()}"
            if warning not in self.warnings:
                self.warnings.append(warning)
            return
        for line in result.stdout.splitlines():
            self._record_line(line)

    def _record_line(self, line: str) -> None:
        parts = line.strip().split("|")
        if len(parts) != 5:
            warning = "unexpected Docker stats output"
            if warning not in self.warnings:
                self.warnings.append(warning)
            return
        identifier = parts[0]
        service = next(
            (
                name
                for name, container_id in self.container_ids.items()
                if container_id.startswith(identifier)
                or identifier.startswith(container_id)
            ),
            None,
        )
        if service is None:
            warning = f"Docker stats returned unknown container {identifier}"
            if warning not in self.warnings:
                self.warnings.append(warning)
            return
        try:
            self.recorder.record_resource(
                service=service,
                cpu_percent=parse_percent(parts[1]),
                memory_bytes=parse_memory_bytes(parts[2].split("/", maxsplit=1)[0]),
                memory_percent=parse_percent(parts[3]),
                pids=int(parts[4]),
            )
        except ValueError as error:
            warning = f"unable to parse Docker stats for {service}: {error}"
            if warning not in self.warnings:
                self.warnings.append(warning)

    def stop(self) -> None:
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=self.interval_seconds + 10)
            self._thread = None

    def restart_counts(self) -> dict[str, int]:
        return {
            service: self.harness.restart_count(container_id)
            for service, container_id in self.container_ids.items()
        }


def _wait_for_databases(harness: ComposeHarness, deadline: float) -> None:
    while time.monotonic() < deadline:
        ready: list[bool] = []
        for service in ("db-baseline", "db-candidate"):
            result = harness.compose(
                "exec",
                "-T",
                service,
                "psql",
                "--username",
                "postgres",
                "--dbname",
                "postgres",
                "--tuples-only",
                "--no-align",
                "--command",
                "SELECT to_regclass('public.dlt_cursor') IS NOT NULL",
                check=False,
            )
            ready.append(result.returncode == 0 and result.stdout.strip() == "t")
        if all(ready):
            return
        time.sleep(POLL_SECONDS)
    raise HarnessError("timed out waiting for both database schemas")


def _read_cursors(
    harness: ComposeHarness, metrics: MetricsRecorder
) -> tuple[int, int]:
    cursors = harness.cursor("db-baseline"), harness.cursor("db-candidate")
    metrics.record_cursors(*cursors)
    return cursors


def _wait_for_initial_progress(
    harness: ComposeHarness, metrics: MetricsRecorder, deadline: float
) -> tuple[int, int]:
    while time.monotonic() < deadline:
        cursors = _read_cursors(harness, metrics)
        print(f"cursor baseline={cursors[0]} candidate={cursors[1]}", flush=True)
        if min(cursors) > 0:
            return cursors
        time.sleep(POLL_SECONDS)
    raise HarnessError("timed out waiting for initial DLT cursor progress")


def _select_boundary(
    harness: ComposeHarness,
    metrics: MetricsRecorder,
    options: RunOptions,
    deadline: float,
) -> int:
    if options.target_slot is not None:
        return options.target_slot

    cursors = _wait_for_initial_progress(harness, metrics, deadline)
    sample_deadline = min(deadline, time.monotonic() + options.sample_seconds)
    while time.monotonic() < sample_deadline:
        time.sleep(min(POLL_SECONDS, max(0.0, sample_deadline - time.monotonic())))
        cursors = _read_cursors(harness, metrics)
        print(f"sampling baseline={cursors[0]} candidate={cursors[1]}", flush=True)
    boundary = min(cursors) - options.boundary_lag
    if boundary <= 0:
        raise HarnessError(
            "unable to derive a positive comparison slot; "
            "increase sample time or reduce boundary lag"
        )
    return boundary


def _wait_for_boundary(
    harness: ComposeHarness,
    metrics: MetricsRecorder,
    comparison_slot: int,
    boundary_lag: int,
    deadline: float,
) -> tuple[int, int]:
    required_cursor = comparison_slot + boundary_lag
    while time.monotonic() < deadline:
        cursors = _read_cursors(harness, metrics)
        print(
            f"target={comparison_slot} required={required_cursor} "
            f"baseline={cursors[0]} candidate={cursors[1]}",
            flush=True,
        )
        if min(cursors) >= required_cursor:
            return cursors
        time.sleep(POLL_SECONDS)
    raise HarnessError(f"timed out before both indexers passed slot {required_cursor}")


def _wait_for_drain(
    harness: ComposeHarness, comparison_slot: int, deadline: float
) -> tuple[tuple[int, int], tuple[int, int]]:
    previous_counts: tuple[int, int] | None = None
    stable_polls = 0
    while time.monotonic() < deadline:
        baseline = harness.operation_state("db-baseline", comparison_slot)
        candidate = harness.operation_state("db-candidate", comparison_slot)
        counts = (baseline[0], candidate[0])
        print(
            f"drain baseline={baseline[0]}/{baseline[1]}-pending "
            f"candidate={candidate[0]}/{candidate[1]}-pending",
            flush=True,
        )
        if baseline[1] == 0 and candidate[1] == 0 and counts == previous_counts:
            stable_polls += 1
            if stable_polls >= 2:
                return baseline, candidate
        else:
            stable_polls = 0
        previous_counts = counts
        time.sleep(POLL_SECONDS)
    raise HarnessError("timed out waiting for operation indexing queues to drain")


def run(options: RunOptions) -> int:
    if options.network == "mainnet" and options.relay_address == DEFAULT_PREPROD_RELAY:
        raise HarnessError("mainnet requires an explicit mainnet relay address")
    options.output_dir.mkdir(parents=True, exist_ok=False)
    harness = ComposeHarness(options)
    deadline = time.monotonic() + options.timeout_seconds
    started_at = datetime.now(UTC)
    run_started_monotonic = time.monotonic()
    metrics: MetricsRecorder | None = None
    sampler: ResourceSampler | None = None
    succeeded = False
    try:
        _run(["docker", "info"])
        baseline_identity = harness.image_identity(options.baseline_image)
        candidate_identity = harness.image_identity(options.candidate_image)
        if baseline_identity.split(maxsplit=1)[0] == candidate_identity.split(
            maxsplit=1
        )[0]:
            raise HarnessError(
                "baseline and candidate resolve to the same image ID; "
                "a differential run requires two distinct images"
            )
        scan_started_monotonic = time.monotonic()
        metrics = MetricsRecorder(scan_started_monotonic)
        harness.compose("up", "--detach")
        sampler = ResourceSampler(harness, metrics, options.metrics_sample_seconds)
        sampler.start()
        _wait_for_databases(harness, deadline)
        comparison_slot = _select_boundary(harness, metrics, options, deadline)
        cursors = _wait_for_boundary(
            harness, metrics, comparison_slot, options.boundary_lag, deadline
        )
        _wait_for_drain(harness, comparison_slot, deadline)
        sampler.stop()
        scan_duration_seconds = time.monotonic() - scan_started_monotonic
        restart_counts = sampler.restart_counts()
        harness.compose("stop", "baseline", "candidate")

        baseline = read_manifest(
            harness.export_manifest("db-baseline", comparison_slot)
        )
        candidate = read_manifest(
            harness.export_manifest("db-candidate", comparison_slot)
        )
        write_manifest(options.output_dir / "baseline.csv", baseline)
        write_manifest(options.output_dir / "candidate.csv", candidate)
        comparison = compare(baseline, candidate)
        total_duration_seconds = time.monotonic() - run_started_monotonic
        metrics.write_samples(options.output_dir)
        metrics_summary = metrics.summarize(
            network=options.network,
            required_cursor=comparison_slot + options.boundary_lag,
            total_duration_seconds=total_duration_seconds,
            scan_duration_seconds=scan_duration_seconds,
            restart_counts=restart_counts,
        )
        metrics_summary["warnings"] = sampler.warnings
        metadata: dict[str, Any] = {
            "network": options.network,
            "relay_address": options.relay_address,
            "comparison_slot": comparison_slot,
            "baseline_cursor": cursors[0],
            "candidate_cursor": cursors[1],
            "baseline_image": options.baseline_image,
            "baseline_identity": baseline_identity,
            "candidate_image": options.candidate_image,
            "candidate_identity": candidate_identity,
            "started_at": started_at.isoformat(),
            "completed_at": datetime.now(UTC).isoformat(),
        }
        write_reports(options.output_dir, comparison, metadata, metrics_summary)
        succeeded = comparison.equal
        print(f"parity result: {'PASS' if comparison.equal else 'FAIL'}")
        print(f"report: {options.output_dir / 'report.md'}")
        return 0 if comparison.equal else 1
    finally:
        if sampler is not None:
            sampler.stop()
        if metrics is not None:
            metrics.write_samples(options.output_dir)
        harness.write_logs()
        if not options.keep:
            harness.cleanup()
        elif not succeeded:
            print("containers retained for diagnostics", file=sys.stderr)


def _positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def parse_args(arguments: list[str] | None = None) -> RunOptions:
    parser = argparse.ArgumentParser(
        description="Compare two NeoPRISM indexers on a shared live-network prefix."
    )
    parser.add_argument("--candidate-image", required=True)
    parser.add_argument("--baseline-image", default=DEFAULT_BASELINE_IMAGE)
    parser.add_argument("--network", choices=("preprod", "mainnet"), default="preprod")
    parser.add_argument("--relay-address")
    parser.add_argument("--target-slot", type=_positive_int)
    parser.add_argument("--sample-seconds", type=_positive_int, default=300)
    parser.add_argument("--boundary-lag", type=_positive_int, default=1_000)
    parser.add_argument("--timeout-seconds", type=_positive_int, default=7_200)
    parser.add_argument("--metrics-sample-seconds", type=_positive_int, default=5)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--allow-mainnet", action="store_true")
    parser.add_argument("--keep", action="store_true")
    args = parser.parse_args(arguments)
    if args.network == "mainnet" and not args.allow_mainnet:
        parser.error("mainnet execution requires --allow-mainnet")
    relay = args.relay_address
    if relay is None:
        relay = (
            DEFAULT_MAINNET_RELAY
            if args.network == "mainnet"
            else DEFAULT_PREPROD_RELAY
        )
    output_dir = args.output_dir
    if output_dir is None:
        stamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
        output_dir = REPO_ROOT / "artifacts" / "indexer-parity" / stamp
    return RunOptions(
        baseline_image=args.baseline_image,
        candidate_image=args.candidate_image,
        network=args.network,
        relay_address=relay,
        target_slot=args.target_slot,
        sample_seconds=args.sample_seconds,
        boundary_lag=args.boundary_lag,
        timeout_seconds=args.timeout_seconds,
        metrics_sample_seconds=args.metrics_sample_seconds,
        output_dir=output_dir.resolve(),
        keep=args.keep,
    )


def main() -> NoReturn:
    try:
        raise SystemExit(run(parse_args()))
    except (HarnessError, ValueError) as error:
        print(f"indexer parity failed: {error}", file=sys.stderr)
        raise SystemExit(2) from error


if __name__ == "__main__":
    main()
