#!/usr/bin/env python3
"""Run the NeoPRISM conformance matrix with isolated, recoverable state."""

from __future__ import annotations

import argparse
import fcntl
import http.client
import json
import os
import platform
import re
import shutil
import signal
import subprocess
import sys
import time
from collections import deque
from collections.abc import Generator, Mapping, Sequence
from contextlib import contextmanager, suppress
from dataclasses import asdict, dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, TextIO, cast

PROFILES = (
    "sqlite-dev",
    "ci",
    "ci-sqlite",
    "ci-blockfrost",
    "ci-embedded-wallet",
)
PROJECT_PREFIX = "neoprism-e2e"
DEFAULT_MIN_FREE_GIB = 10
EMBEDDED_WALLET_READY_TIMEOUT_SECONDS = 120
EMBEDDED_WALLET_READY_POLL_SECONDS = 1
STREAMED_EXECUTABLES = frozenset({"docker", "docker-compose", "nix", "sbt"})
REQUIRED_EXECUTABLES = ("docker", "docker-compose", "nix", "sbt")
_termination_defer_depth = 0
_pending_termination: int | None = None


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def generate_project_name(started_at: datetime, process_id: int) -> str:
    timestamp = started_at.astimezone(UTC).strftime("%Y%m%d%H%M%S")
    return f"{PROJECT_PREFIX}-{timestamp}-{process_id}"


def compose_command(project: str, compose_file: Path, *arguments: str) -> list[str]:
    return ["docker-compose", "-p", project, "-f", str(compose_file), *arguments]


def classify_test_networks(
    networks: Sequence[Mapping[str, Any]],
) -> list[tuple[str, str]]:
    stale: list[tuple[str, str]] = []
    for network in networks:
        labels_value = network.get("Labels")
        labels = (
            cast(dict[str, Any], labels_value) if isinstance(labels_value, dict) else {}
        )
        project_value = labels.get("com.docker.compose.project")
        project = project_value if isinstance(project_value, str) else None
        containers = network.get("Containers")
        is_unused = isinstance(containers, dict) and not containers
        is_test_owned = project is not None and (
            project.startswith(f"{PROJECT_PREFIX}-") or project == "prism-test"
        )
        network_id = network.get("Id")
        name = network.get("Name")
        if (
            is_unused
            and is_test_owned
            and isinstance(network_id, str)
            and isinstance(name, str)
        ):
            stale.append((network_id, name))
    return stale


@dataclass
class ProfileReceipt:
    name: str
    compose_file: str
    project: str
    started_at: str
    ended_at: str | None = None
    duration_seconds: float | None = None
    readiness_duration_seconds: float | None = None
    cold_cache: bool = False
    missing_images: list[str] = field(default_factory=lambda: [])
    status: str = "running"
    cleanup_status: str = "pending"
    tests_passed: int | None = None
    log: str | None = None


@dataclass
class RunReceipt:
    schema_version: int
    started_at: str
    git_revision: str
    git_dirty: bool
    host_platform: str
    docker_architecture: str
    docker_version: str
    project_prefix: str
    receipt_path: str
    profiles: list[ProfileReceipt] = field(default_factory=lambda: [])
    ended_at: str | None = None
    duration_seconds: float | None = None
    image_build_duration_seconds: float | None = None
    neoprism_image_id: str | None = None
    status: str = "running"


class CommandFailure(RuntimeError):
    def __init__(self, command: Sequence[str], return_code: int, output: str) -> None:
        super().__init__(
            f"command failed with exit code {return_code}: {' '.join(command)}"
        )
        self.command = list(command)
        self.return_code = return_code
        self.output = output


class TerminationRequested(Exception):
    def __init__(self, signum: int) -> None:
        super().__init__(f"received {signal.Signals(signum).name}")
        self.signum = signum


def captured(command: Sequence[str], cwd: Path) -> str:
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as error:
        executable = command[0] if command else "command"
        raise RuntimeError(
            f"failed to start required executable {executable}: {error}"
        ) from error
    if completed.returncode != 0:
        raise CommandFailure(command, completed.returncode, completed.stderr)
    return completed.stdout.strip()


def trusted_streamed_command(command: Sequence[str]) -> list[str]:
    if not command or command[0] not in STREAMED_EXECUTABLES:
        raise ValueError("streamed command does not use an approved executable")
    if any("\0" in argument for argument in command):
        raise ValueError("streamed command contains a null byte")
    return list(command)


def streamed(
    command: Sequence[str],
    cwd: Path,
    log: TextIO,
    extra_environment: Mapping[str, str] | None = None,
) -> str:
    trusted_command = trusted_streamed_command(command)

    environment = os.environ.copy()
    environment["COMPOSE_PROGRESS"] = "plain"
    if extra_environment:
        environment.update(extra_environment)

    # Every executable is allowlisted above, arguments are constructed internally
    # from argparse choices, and shell expansion is disabled. This is not an LLM
    # command sink.
    try:
        process = subprocess.Popen(  # NOSONAR
            trusted_command,
            cwd=cwd,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
            start_new_session=True,
            shell=False,
        )
    except OSError as error:
        raise RuntimeError(
            f"failed to start required executable {trusted_command[0]}: {error}"
        ) from error
    output: deque[str] = deque(maxlen=2_000)
    assert process.stdout is not None
    try:
        for line in process.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()
            log.write(line)
            log.flush()
            output.append(line)
        return_code = process.wait()
    except BaseException:
        terminate_process_group(process)
        raise
    joined_output = "".join(output)
    if return_code != 0:
        raise CommandFailure(command, return_code, joined_output)
    return joined_output


def terminate_process_group(process: subprocess.Popen[str]) -> None:
    """Stop and reap a streamed command and any children it spawned."""
    if process.poll() is not None:
        return
    with suppress(ProcessLookupError):
        os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        with suppress(ProcessLookupError):
            os.killpg(process.pid, signal.SIGKILL)
        process.wait()


@contextmanager
def termination_signal_handlers() -> Generator[None]:
    """Turn termination signals into exceptions so profile cleanup can run."""
    global _pending_termination, _termination_defer_depth
    previous: dict[signal.Signals, Any] = {}

    def request_termination(signum: int, _frame: object) -> None:
        global _pending_termination
        if _termination_defer_depth > 0:
            _pending_termination = signum
            return
        raise TerminationRequested(signum)

    _termination_defer_depth = 0
    _pending_termination = None
    for handled_signal in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        previous[handled_signal] = signal.signal(handled_signal, request_termination)
    try:
        yield
    finally:
        for handled_signal, handler in previous.items():
            signal.signal(handled_signal, handler)
        _termination_defer_depth = 0
        _pending_termination = None


@contextmanager
def defer_termination_signals() -> Generator[None]:
    """Record termination while an atomic cleanup section completes."""
    global _pending_termination, _termination_defer_depth
    _termination_defer_depth += 1
    try:
        yield
    finally:
        _termination_defer_depth -= 1
        if _termination_defer_depth == 0 and _pending_termination is not None:
            signum = _pending_termination
            _pending_termination = None
            raise TerminationRequested(signum)


def atomic_write_json(path: Path, receipt: RunReceipt) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(f"{path.suffix}.tmp")
    temporary.write_text(json.dumps(asdict(receipt), indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def missing_required_executables() -> list[str]:
    """Return host tools that must exist before the matrix touches Docker."""
    return [
        executable
        for executable in REQUIRED_EXECUTABLES
        if shutil.which(executable) is None
    ]


def parse_json_object(value: str) -> dict[str, Any]:
    parsed: object = json.loads(value)
    if not isinstance(parsed, dict):
        raise ValueError("expected a JSON object")
    return cast(dict[str, Any], parsed)


def parse_json_array(value: str) -> list[dict[str, Any]]:
    parsed: object = json.loads(value)
    if not isinstance(parsed, list):
        raise ValueError("expected an array of JSON objects")
    objects: list[dict[str, Any]] = []
    for item in cast(list[object], parsed):
        if not isinstance(item, dict):
            raise ValueError("expected an array of JSON objects")
        objects.append(cast(dict[str, Any], item))
    return objects


def required_mapping(
    mapping: Mapping[str, Any], key: str, error_message: str
) -> dict[str, Any]:
    value = mapping.get(key)
    if not isinstance(value, dict):
        raise RuntimeError(error_message)
    return cast(dict[str, Any], value)


def published_port(service: Mapping[str, Any], target: int) -> int:
    ports = service.get("ports")
    if not isinstance(ports, list):
        raise RuntimeError("Blockfrost proxy has no published ports")
    for value in cast(list[Any], ports):
        if not isinstance(value, dict):
            continue
        port = cast(dict[str, Any], value)
        published = port.get("published")
        if port.get("target") == target and isinstance(published, (str, int)):
            return int(published)
    raise RuntimeError(f"Blockfrost proxy port {target} is not published")


def embedded_wallet_funding_target(config: dict[str, Any]) -> tuple[str, int, str]:
    """Read the funding address and published Blockfrost port from config."""
    services = required_mapping(
        config, "services", "Compose config has no services object"
    )

    bootstrap = required_mapping(
        services, "bootstrap-testnet", "embedded-wallet Compose services are missing"
    )
    proxy = required_mapping(
        services, "bf-proxy", "embedded-wallet Compose services are missing"
    )

    environment = required_mapping(
        bootstrap, "environment", "embedded-wallet funding address is missing"
    )
    address = environment.get("GENESIS_PAYMENT_ADDR")
    if not isinstance(address, str) or not address:
        raise RuntimeError("embedded-wallet funding address is missing")

    return "127.0.0.1", published_port(proxy, 3000), address


def compose_embedded_wallet_funding_target(
    repository: Path, compose_file: Path
) -> tuple[str, int, str]:
    """Resolve the embedded-wallet funding target from rendered Compose JSON."""
    config = parse_json_object(
        captured(
            [
                "docker-compose",
                "-f",
                str(compose_file),
                "config",
                "--format",
                "json",
            ],
            repository,
        )
    )
    return embedded_wallet_funding_target(config)


def blockfrost_utxos(host: str, port: int, address: str) -> list[dict[str, Any]]:
    """Fetch the deterministic wallet's UTXOs from the local Blockfrost proxy."""
    connection = http.client.HTTPConnection(host, port, timeout=5)
    try:
        connection.request("GET", f"/addresses/{address}/utxos")
        response = connection.getresponse()
        body = response.read().decode("utf-8")
        if response.status != 200:
            raise RuntimeError(
                f"Blockfrost readiness returned HTTP {response.status}: {body[:200]}"
            )
        return parse_json_array(body)
    finally:
        connection.close()


def wait_for_embedded_wallet_funding(
    repository: Path,
    compose_file: Path,
    timeout_seconds: int = EMBEDDED_WALLET_READY_TIMEOUT_SECONDS,
) -> float:
    """Wait until the deterministic embedded wallet has a spendable UTXO."""
    host, port, address = compose_embedded_wallet_funding_target(
        repository, compose_file
    )
    started = time.monotonic()
    deadline = started + timeout_seconds
    last_error = "no UTXOs returned"
    while True:
        try:
            if blockfrost_utxos(host, port, address):
                return round(time.monotonic() - started, 3)
            last_error = "no UTXOs returned"
        except (http.client.HTTPException, OSError, RuntimeError, ValueError) as error:
            last_error = str(error)
        if time.monotonic() >= deadline:
            raise RuntimeError(
                "embedded-wallet funding did not become ready within "
                f"{timeout_seconds}s ({last_error})"
            )
        time.sleep(EMBEDDED_WALLET_READY_POLL_SECONDS)


def conformance_test_count(output: str) -> int:
    matches = re.findall(r"(?m)^(?:\[info\]\s*)?(\d+) tests passed\.", output)
    if not matches:
        raise RuntimeError("SBT output did not contain a conformance test receipt")
    count = sum(int(value) for value in matches)
    if count == 0:
        raise RuntimeError("SBT reported zero conformance tests")
    return count


def host_lock_path() -> Path:
    """Return the per-user host lock shared by every repository worktree."""
    return Path.home() / ".cache" / "neoprism" / "e2e.lock"


@contextmanager
def exclusive_run_lock(lock_path: Path | None = None) -> Generator[None]:
    lock_path = lock_path if lock_path is not None else host_lock_path()
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+", encoding="utf-8") as lock_file:
        try:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise RuntimeError(
                f"another E2E matrix owns {lock_path}; fixed host ports require "
                "serialized runs"
            ) from error
        try:
            yield
        finally:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)


def inspect_test_networks(repository: Path) -> tuple[int, list[tuple[str, str]]]:
    dangling_ids = captured(
        ["docker", "network", "ls", "-q", "--filter", "dangling=true"], repository
    ).splitlines()
    all_ids = captured(["docker", "network", "ls", "-q"], repository).splitlines()
    if not all_ids:
        return len(dangling_ids), []
    networks = parse_json_array(
        captured(["docker", "network", "inspect", *all_ids], repository)
    )
    return len(dangling_ids), classify_test_networks(networks)


def preflight(
    repository: Path, profiles: Sequence[str], min_free_gib: int
) -> dict[str, str]:
    missing_executables = missing_required_executables()
    if missing_executables:
        raise RuntimeError(
            "required E2E executables are unavailable: "
            f"{', '.join(missing_executables)}; enter the Nix development shell"
        )

    docker_info = parse_json_object(
        captured(["docker", "info", "--format", "{{json .}}"], repository)
    )
    for profile in profiles:
        compose_file = repository / "docker" / "prism-test" / f"compose-{profile}.yml"
        if not compose_file.is_file():
            raise RuntimeError(f"missing generated Compose file: {compose_file}")

    free_bytes = shutil.disk_usage(repository).free
    required_bytes = min_free_gib * 1024**3
    if free_bytes < required_bytes:
        free_gib = free_bytes / 1024**3
        raise RuntimeError(
            f"only {free_gib:.1f} GiB is free; the E2E matrix requires at least "
            f"{min_free_gib} GiB"
        )

    dangling_count, stale_test_networks = inspect_test_networks(repository)
    if dangling_count >= 16:
        print(
            f"warning: Docker has {dangling_count} unused networks; address-pool "
            "exhaustion is likely",
            file=sys.stderr,
        )
    if stale_test_networks:
        names = ", ".join(name for _, name in stale_test_networks)
        identifiers = " ".join(network_id for network_id, _ in stale_test_networks)
        print(
            f"warning: unused NeoPRISM E2E networks detected: {names}", file=sys.stderr
        )
        print(
            f"recovery (not run automatically): docker network rm {identifiers}",
            file=sys.stderr,
        )

    architecture = docker_info.get("Architecture")
    server_version = docker_info.get("ServerVersion")
    return {
        "architecture": architecture if isinstance(architecture, str) else "unknown",
        "server_version": server_version
        if isinstance(server_version, str)
        else "unknown",
    }


def missing_images(repository: Path, compose_file: Path) -> list[str]:
    images = captured(
        ["docker-compose", "-f", str(compose_file), "config", "--images"], repository
    )
    missing: list[str] = []
    for image in sorted(set(images.splitlines())):
        if not image:
            continue
        try:
            completed = subprocess.run(
                ["docker", "image", "inspect", image],
                cwd=repository,
                check=False,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except OSError as error:
            raise RuntimeError(
                f"failed to start required executable docker: {error}"
            ) from error
        if completed.returncode != 0:
            missing.append(image)
    return missing


def cleanup(repository: Path, project: str, compose_file: Path, log: TextIO) -> str:
    command = compose_command(
        project, compose_file, "down", "--volumes", "--remove-orphans"
    )
    try:
        streamed(command, repository, log)
    except RuntimeError as error:
        print(f"cleanup failed for {project}: {error}", file=sys.stderr)
        return "failed"
    return "passed"


def print_address_pool_recovery(error: CommandFailure, repository: Path) -> None:
    if "all predefined address pools have been fully subnetted" not in error.output:
        return
    _, stale_test_networks = inspect_test_networks(repository)
    if stale_test_networks:
        identifiers = " ".join(network_id for network_id, _ in stale_test_networks)
        print("Docker address pools are exhausted.", file=sys.stderr)
        print(
            "Remove only stale NeoPRISM test networks, then retry: "
            f"docker network rm {identifiers}",
            file=sys.stderr,
        )
    else:
        print(
            "Docker address pools are exhausted, but no stale NeoPRISM test "
            "networks were found. "
            "Inspect `docker network ls` before removing unrelated networks.",
            file=sys.stderr,
        )


def run_profile(
    repository: Path,
    profile: str,
    run_id: str,
    log_directory: Path,
    receipt_path: Path,
    receipt: RunReceipt,
) -> None:
    compose_file = repository / "docker" / "prism-test" / f"compose-{profile}.yml"
    project = f"{run_id}-{profile}"
    log_path = log_directory / f"{profile}.log"
    profile_started = time.monotonic()
    profile_receipt = ProfileReceipt(
        name=profile,
        compose_file=str(compose_file.relative_to(repository)),
        project=project,
        started_at=utc_now(),
        log=str(log_path.relative_to(repository)),
    )
    receipt.profiles.append(profile_receipt)
    atomic_write_json(receipt_path, receipt)

    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("w", encoding="utf-8") as profile_log:
        try:
            images = missing_images(repository, compose_file)
            profile_receipt.cold_cache = bool(images)
            profile_receipt.missing_images = images
            atomic_write_json(receipt_path, receipt)
            print(f"\n=== Testing {profile} ({project}) ===")
            streamed(
                compose_command(project, compose_file, "up", "-d", "--wait"),
                repository,
                profile_log,
            )
            if profile == "ci-embedded-wallet":
                profile_receipt.readiness_duration_seconds = (
                    wait_for_embedded_wallet_funding(repository, compose_file)
                )
                print(
                    "embedded-wallet funding ready after "
                    f"{profile_receipt.readiness_duration_seconds:.3f}s"
                )
            test_output = streamed(
                ["sbt", "clean;Test / testOnly *"],
                repository / "tests" / "prism-test",
                profile_log,
            )
            profile_receipt.tests_passed = conformance_test_count(test_output)
            profile_receipt.status = "passed"
        except (KeyboardInterrupt, RuntimeError, TerminationRequested) as error:
            profile_receipt.status = (
                "interrupted"
                if isinstance(error, (KeyboardInterrupt, TerminationRequested))
                else "failed"
            )
            if isinstance(error, CommandFailure):
                print_address_pool_recovery(error, repository)
            with suppress(CommandFailure):
                streamed(
                    compose_command(project, compose_file, "logs", "--no-color"),
                    repository,
                    profile_log,
                )
            raise
        finally:
            with defer_termination_signals():
                profile_receipt.cleanup_status = cleanup(
                    repository, project, compose_file, profile_log
                )
                if profile_receipt.cleanup_status != "passed":
                    profile_receipt.status = "failed"
                profile_receipt.ended_at = utc_now()
                profile_receipt.duration_seconds = round(
                    time.monotonic() - profile_started, 3
                )
                atomic_write_json(receipt_path, receipt)
    if profile_receipt.cleanup_status != "passed":
        raise RuntimeError(f"cleanup failed for E2E profile {profile}")


def run_matrix(repository: Path, profiles: Sequence[str], min_free_gib: int) -> int:
    started_datetime = datetime.now(UTC)
    started_monotonic = time.monotonic()
    revision = captured(["git", "rev-parse", "HEAD"], repository)
    dirty = bool(captured(["git", "status", "--porcelain"], repository))
    run_id = generate_project_name(started_datetime, os.getpid())
    receipt_path = repository / "target" / "e2e-receipts" / f"{run_id}.json"
    log_directory = receipt_path.parent / run_id
    docker = preflight(repository, profiles, min_free_gib)

    receipt = RunReceipt(
        schema_version=1,
        started_at=started_datetime.isoformat(timespec="seconds"),
        git_revision=revision,
        git_dirty=dirty,
        host_platform=platform.platform(),
        docker_architecture=docker["architecture"],
        docker_version=docker["server_version"],
        project_prefix=run_id,
        receipt_path=str(receipt_path.relative_to(repository)),
    )
    atomic_write_json(receipt_path, receipt)

    print(f"E2E receipt: {receipt_path}")
    print(f"Compose project prefix: {run_id}")
    image_log_path = log_directory / "image-build.log"
    image_log_path.parent.mkdir(parents=True, exist_ok=True)
    image_started = time.monotonic()

    try:
        with image_log_path.open("w", encoding="utf-8") as image_log:
            streamed(
                ["nix", "build", ".#neoprism-docker-latest"], repository, image_log
            )
            streamed(["docker", "load", "-i", "result"], repository, image_log)
        receipt.image_build_duration_seconds = round(
            time.monotonic() - image_started, 3
        )
        receipt.neoprism_image_id = captured(
            [
                "docker",
                "image",
                "inspect",
                "identus-neoprism:latest",
                "--format",
                "{{.Id}}",
            ],
            repository,
        )

        for profile in profiles:
            run_profile(
                repository,
                profile,
                run_id,
                log_directory,
                receipt_path,
                receipt,
            )

        receipt.status = "passed"
        return 0
    except (KeyboardInterrupt, RuntimeError, TerminationRequested) as error:
        receipt.status = (
            "interrupted"
            if isinstance(error, (KeyboardInterrupt, TerminationRequested))
            else "failed"
        )
        print(str(error), file=sys.stderr)
        return 1
    finally:
        with defer_termination_signals():
            receipt.ended_at = utc_now()
            receipt.duration_seconds = round(time.monotonic() - started_monotonic, 3)
            atomic_write_json(receipt_path, receipt)
            print(f"E2E result: {receipt.status}; receipt: {receipt_path}")


def parse_arguments(arguments: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--profile",
        action="append",
        choices=PROFILES,
        dest="profiles",
        help="run only this profile; repeat to select multiple profiles",
    )
    parser.add_argument(
        "--min-free-gib",
        type=int,
        default=int(os.environ.get("NEOPRISM_E2E_MIN_FREE_GIB", DEFAULT_MIN_FREE_GIB)),
        help="minimum host disk space required before starting",
    )
    return parser.parse_args(arguments)


def main(arguments: Sequence[str] | None = None) -> int:
    options = parse_arguments(arguments if arguments is not None else sys.argv[1:])
    repository = Path(__file__).resolve().parent.parent
    profiles = options.profiles if options.profiles else list(PROFILES)
    try:
        with termination_signal_handlers(), exclusive_run_lock():
            return run_matrix(repository, profiles, options.min_free_gib)
    except (RuntimeError, TerminationRequested) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
