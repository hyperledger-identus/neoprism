from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, NoReturn

from .manifest import compare, read_manifest, write_manifest, write_reports

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

    def write_logs(self) -> None:
        logs = self.compose("logs", "--no-color", check=False)
        (self.options.output_dir / "containers.log").write_text(
            logs.stdout + logs.stderr,
            encoding="utf-8",
        )

    def cleanup(self) -> None:
        self.compose("down", "--volumes", "--remove-orphans", check=False)


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


def _read_cursors(harness: ComposeHarness) -> tuple[int, int]:
    return harness.cursor("db-baseline"), harness.cursor("db-candidate")


def _wait_for_initial_progress(
    harness: ComposeHarness, deadline: float
) -> tuple[int, int]:
    while time.monotonic() < deadline:
        cursors = _read_cursors(harness)
        print(f"cursor baseline={cursors[0]} candidate={cursors[1]}", flush=True)
        if min(cursors) > 0:
            return cursors
        time.sleep(POLL_SECONDS)
    raise HarnessError("timed out waiting for initial DLT cursor progress")


def _select_boundary(
    harness: ComposeHarness, options: RunOptions, deadline: float
) -> int:
    if options.target_slot is not None:
        return options.target_slot

    cursors = _wait_for_initial_progress(harness, deadline)
    sample_deadline = min(deadline, time.monotonic() + options.sample_seconds)
    while time.monotonic() < sample_deadline:
        time.sleep(min(POLL_SECONDS, max(0.0, sample_deadline - time.monotonic())))
        cursors = _read_cursors(harness)
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
    comparison_slot: int,
    boundary_lag: int,
    deadline: float,
) -> tuple[int, int]:
    required_cursor = comparison_slot + boundary_lag
    while time.monotonic() < deadline:
        cursors = _read_cursors(harness)
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
        harness.compose("up", "--detach")
        _wait_for_databases(harness, deadline)
        comparison_slot = _select_boundary(harness, options, deadline)
        cursors = _wait_for_boundary(
            harness, comparison_slot, options.boundary_lag, deadline
        )
        _wait_for_drain(harness, comparison_slot, deadline)
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
        write_reports(options.output_dir, comparison, metadata)
        succeeded = comparison.equal
        print(f"parity result: {'PASS' if comparison.equal else 'FAIL'}")
        print(f"report: {options.output_dir / 'report.md'}")
        return 0 if comparison.equal else 1
    finally:
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
