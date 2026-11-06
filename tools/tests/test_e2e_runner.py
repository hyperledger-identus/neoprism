from __future__ import annotations

import json
import signal
import subprocess
import tempfile
import unittest
from datetime import UTC, datetime
from pathlib import Path
from typing import Any
from unittest.mock import MagicMock, call, patch

from e2e_runner import (
    ProfileReceipt,
    RunReceipt,
    TerminationRequested,
    address_pool_capacity,
    atomic_write_json,
    captured,
    classify_test_networks,
    cleanup,
    compose_command,
    conformance_test_count,
    defer_termination_signals,
    embedded_wallet_funding_target,
    exclusive_run_lock,
    finalize_run_receipt,
    generate_project_name,
    missing_required_executables,
    preflight,
    pull_missing_images,
    run_profile,
    streamed,
    terminate_process_group,
    termination_signal_handlers,
    trusted_streamed_command,
    wait_for_embedded_wallet_funding,
)


class E2ERunnerTest(unittest.TestCase):
    @patch("e2e_runner.time.monotonic", side_effect=[10.0, 14.25])
    @patch("e2e_runner.streamed")
    def test_missing_image_pull_duration_is_isolated(
        self, streamed_command: MagicMock, _monotonic: object
    ) -> None:
        with tempfile.TemporaryFile(mode="w+") as log:
            duration = pull_missing_images(
                Path("."), ["postgres:16", "caddy:2.10.2"], log
            )

        self.assertEqual(duration, 4.25)
        self.assertEqual(
            streamed_command.call_args_list,
            [
                call(["docker", "pull", "--quiet", "postgres:16"], Path("."), log),
                call(["docker", "pull", "--quiet", "caddy:2.10.2"], Path("."), log),
            ],
        )

    def test_active_networks_can_exhaust_configured_address_pool(self) -> None:
        docker_info = {"DefaultAddressPools": [{"Base": "10.20.0.0/24", "Size": 25}]}
        networks = [
            {"IPAM": {"Config": [{"Subnet": "10.20.0.0/25"}]}},
            {"IPAM": {"Config": [{"Subnet": "10.20.0.128/25"}]}},
        ]

        self.assertEqual(address_pool_capacity(docker_info, networks), (2, 2, 0))

    @patch("e2e_runner.shutil.disk_usage")
    @patch("e2e_runner.inspect_network_inventory", return_value=(0, []))
    @patch("e2e_runner.address_pool_capacity", return_value=(2, 2, 0))
    @patch("e2e_runner.missing_required_executables", return_value=[])
    @patch(
        "e2e_runner.captured",
        return_value='{"Architecture":"arm64","ServerVersion":"1"}',
    )
    def test_preflight_rejects_address_pool_exhaustion_before_startup(
        self,
        _captured: object,
        _executables: object,
        _capacity: object,
        _networks: object,
        disk_usage: MagicMock,
    ) -> None:
        disk_usage.return_value.free = 100 * 1024**3
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            compose_directory = repository / "docker" / "prism-test"
            compose_directory.mkdir(parents=True)
            (compose_directory / "compose-sqlite-dev.yml").write_text(
                "services: {}\n", encoding="utf-8"
            )

            with self.assertRaisesRegex(RuntimeError, "before E2E startup"):
                preflight(repository, ["sqlite-dev"], 10)

    def test_project_name_is_unique_and_compose_safe(self) -> None:
        started_at = datetime(2026, 10, 27, 12, 34, 56, tzinfo=UTC)

        self.assertEqual(
            generate_project_name(started_at, 1234), "neoprism-e2e-20261027123456-1234"
        )

    def test_compose_command_isolates_the_project(self) -> None:
        command = compose_command(
            "neoprism-e2e-1", Path("compose.yml"), "up", "-d", "--wait"
        )

        self.assertEqual(
            command,
            [
                "docker-compose",
                "-p",
                "neoprism-e2e-1",
                "-f",
                "compose.yml",
                "up",
                "-d",
                "--wait",
            ],
        )

    def test_streamed_commands_require_an_internal_executable(self) -> None:
        self.assertEqual(
            trusted_streamed_command(["docker", "load", "-i", "result"]),
            ["docker", "load", "-i", "result"],
        )
        with self.assertRaisesRegex(ValueError, "approved executable"):
            trusted_streamed_command(["sh", "-c", "echo unsafe"])
        with self.assertRaisesRegex(ValueError, "null byte"):
            trusted_streamed_command(["docker", "bad\0argument"])

    @patch("e2e_runner.shutil.which")
    def test_missing_required_executables_are_reported_before_docker(
        self, which: MagicMock
    ) -> None:
        def executable_path(executable: str) -> str | None:
            return None if executable in {"nix", "sbt"} else f"/bin/{executable}"

        which.side_effect = executable_path

        self.assertEqual(missing_required_executables(), ["nix", "sbt"])

    @patch("e2e_runner.subprocess.Popen", side_effect=FileNotFoundError("missing"))
    def test_streamed_launch_failure_becomes_a_receiptable_error(
        self, _popen: object
    ) -> None:
        with (
            tempfile.TemporaryFile(mode="w+") as log,
            self.assertRaisesRegex(
                RuntimeError, "failed to start required executable sbt"
            ),
        ):
            streamed(["sbt", "test"], Path("."), log)

    @patch("e2e_runner.subprocess.run", side_effect=FileNotFoundError("missing"))
    def test_captured_launch_failure_becomes_a_receiptable_error(
        self, _run: object
    ) -> None:
        with self.assertRaisesRegex(
            RuntimeError, "failed to start required executable docker"
        ):
            captured(["docker", "info"], Path("."))

    @patch("e2e_runner.streamed", side_effect=RuntimeError("failed to start"))
    def test_cleanup_launch_failure_is_recorded(self, _streamed: object) -> None:
        with tempfile.TemporaryFile(mode="w+") as log, patch("e2e_runner.sys.stderr"):
            self.assertEqual(
                cleanup(Path("."), "neoprism-e2e-test", Path("compose.yml"), log),
                "failed",
            )

    @patch("e2e_runner.cleanup", return_value="passed")
    @patch("e2e_runner.streamed", return_value="")
    @patch("e2e_runner.missing_images", side_effect=RuntimeError("inventory failed"))
    def test_image_inventory_failure_has_a_terminal_profile_receipt(
        self, _images: object, _streamed: object, _cleanup: object
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            compose_directory = repository / "docker" / "prism-test"
            compose_directory.mkdir(parents=True)
            (compose_directory / "compose-sqlite-dev.yml").write_text(
                "services: {}\n", encoding="utf-8"
            )
            receipt_path = repository / "target" / "receipt.json"
            receipt = RunReceipt(
                schema_version=1,
                started_at="2026-10-29T00:00:00+00:00",
                git_revision="abc123",
                git_dirty=False,
                host_platform="test",
                docker_architecture="arm64",
                docker_version="1",
                project_prefix="neoprism-e2e-test",
                receipt_path="target/receipt.json",
            )

            with self.assertRaisesRegex(RuntimeError, "inventory failed"):
                run_profile(
                    repository,
                    "sqlite-dev",
                    "neoprism-e2e-test",
                    repository / "target" / "logs",
                    receipt_path,
                    receipt,
                )

            decoded = json.loads(receipt_path.read_text(encoding="utf-8"))
            self.assertEqual(decoded["profiles"][0]["status"], "failed")
            self.assertEqual(decoded["profiles"][0]["cleanup_status"], "passed")

    def test_classify_networks_returns_only_unused_test_projects(self) -> None:
        networks: list[dict[str, Any]] = [
            {
                "Id": "stale-id",
                "Name": "neoprism-e2e-old_default",
                "Labels": {"com.docker.compose.project": "neoprism-e2e-old"},
                "Containers": {},
            },
            {
                "Id": "legacy-id",
                "Name": "prism-test_default",
                "Labels": {"com.docker.compose.project": "prism-test"},
                "Containers": {},
            },
            {
                "Id": "active-id",
                "Name": "neoprism-e2e-active_default",
                "Labels": {"com.docker.compose.project": "neoprism-e2e-active"},
                "Containers": {"container": {}},
            },
            {
                "Id": "other-id",
                "Name": "other_default",
                "Labels": {"com.docker.compose.project": "other"},
                "Containers": {},
            },
        ]

        self.assertEqual(
            classify_test_networks(networks),
            [
                ("stale-id", "neoprism-e2e-old_default"),
                ("legacy-id", "prism-test_default"),
            ],
        )

    def test_atomic_receipt_write_is_valid_json(self) -> None:
        receipt = RunReceipt(
            schema_version=1,
            started_at="2026-10-27T12:34:56+00:00",
            git_revision="abc123",
            git_dirty=False,
            host_platform="test",
            docker_architecture="arm64",
            docker_version="1",
            project_prefix="neoprism-e2e-test",
            receipt_path="target/receipt.json",
            profiles=[
                ProfileReceipt(
                    name="sqlite-dev",
                    compose_file="compose.yml",
                    project="neoprism-e2e-test-sqlite-dev",
                    started_at="2026-10-27T12:34:56+00:00",
                )
            ],
        )

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            atomic_write_json(path, receipt)
            decoded = json.loads(path.read_text(encoding="utf-8"))

        self.assertEqual(decoded["git_revision"], "abc123")
        self.assertEqual(decoded["profiles"][0]["status"], "running")

    def test_conformance_test_count_requires_non_zero_receipt(self) -> None:
        self.assertEqual(conformance_test_count("73 tests passed. 0 tests failed."), 73)
        self.assertEqual(
            conformance_test_count("[info] 73 tests passed. 0 tests failed."), 73
        )

        with self.assertRaisesRegex(RuntimeError, "did not contain"):
            conformance_test_count("No tests to run for Test / testQuick")

        with self.assertRaisesRegex(RuntimeError, "zero conformance"):
            conformance_test_count("0 tests passed. 0 tests failed.")

    def test_embedded_wallet_target_comes_from_compose_config(self) -> None:
        config = {
            "services": {
                "bootstrap-testnet": {
                    "environment": {"GENESIS_PAYMENT_ADDR": "addr_test1fixture"}
                },
                "bf-proxy": {"ports": [{"target": 3000, "published": "19082"}]},
            }
        }

        self.assertEqual(
            embedded_wallet_funding_target(config),
            ("127.0.0.1", 19082, "addr_test1fixture"),
        )

    @patch("e2e_runner.blockfrost_utxos", return_value=[{"tx_hash": "abc"}])
    @patch(
        "e2e_runner.compose_embedded_wallet_funding_target",
        return_value=("127.0.0.1", 19082, "addr_test1fixture"),
    )
    def test_embedded_wallet_readiness_accepts_a_spendable_utxo(
        self, _target: object, _utxos: object
    ) -> None:
        duration = wait_for_embedded_wallet_funding(
            Path("."), Path("compose.yml"), timeout_seconds=0
        )

        self.assertGreaterEqual(duration, 0)

    @patch("e2e_runner.blockfrost_utxos", return_value=[])
    @patch(
        "e2e_runner.compose_embedded_wallet_funding_target",
        return_value=("127.0.0.1", 19082, "addr_test1fixture"),
    )
    def test_embedded_wallet_readiness_is_bounded(
        self, _target: object, _utxos: object
    ) -> None:
        with self.assertRaisesRegex(RuntimeError, "within 0s"):
            wait_for_embedded_wallet_funding(
                Path("."), Path("compose.yml"), timeout_seconds=0
            )

    def test_run_lock_serializes_fixed_port_profiles(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            lock_path = Path(directory) / "runner.lock"
            with (
                exclusive_run_lock(lock_path),
                self.assertRaisesRegex(RuntimeError, "serialized runs"),
                exclusive_run_lock(lock_path),
            ):
                self.fail("the second runner unexpectedly acquired the lock")

    def test_termination_signal_becomes_cleanup_safe_exception(self) -> None:
        previous = signal.getsignal(signal.SIGTERM)

        with self.assertRaises(TerminationRequested), termination_signal_handlers():
            signal.raise_signal(signal.SIGTERM)

        self.assertEqual(signal.getsignal(signal.SIGTERM), previous)

    def test_termination_is_deferred_until_atomic_cleanup_finishes(self) -> None:
        cleanup_finished = False

        with (
            self.assertRaises(TerminationRequested),
            termination_signal_handlers(),
            defer_termination_signals(),
        ):
            signal.raise_signal(signal.SIGTERM)
            cleanup_finished = True

        self.assertTrue(cleanup_finished)

    @patch("e2e_runner.os.killpg")
    def test_repeated_signal_cannot_skip_child_escalation_and_reaping(
        self, killpg: MagicMock
    ) -> None:
        process: MagicMock = MagicMock()
        process.pid = 1234
        process.poll.return_value = None

        def wait(timeout: int | None = None) -> int:
            if timeout is not None:
                signal.raise_signal(signal.SIGTERM)
                raise subprocess.TimeoutExpired("fixture", timeout)
            return 0

        process.wait.side_effect = wait

        with self.assertRaises(TerminationRequested), termination_signal_handlers():
            terminate_process_group(process)

        self.assertEqual(
            killpg.call_args_list,
            [
                call(1234, signal.SIGTERM),
                call(1234, signal.SIGKILL),
            ],
        )
        self.assertEqual(process.wait.call_count, 2)

    def test_signal_during_final_write_rewrites_pass_as_interrupted(self) -> None:
        receipt = RunReceipt(
            schema_version=1,
            started_at="2026-11-06T00:00:00+00:00",
            git_revision="abc123",
            git_dirty=False,
            host_platform="test",
            docker_architecture="arm64",
            docker_version="1",
            project_prefix="neoprism-e2e-test",
            receipt_path="target/receipt.json",
            status="passed",
        )

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            original_write = atomic_write_json
            writes = 0

            def interrupting_write(target: Path, value: RunReceipt) -> None:
                nonlocal writes
                writes += 1
                signal.raise_signal(signal.SIGINT)
                original_write(target, value)

            with (
                patch("e2e_runner.atomic_write_json", side_effect=interrupting_write),
                self.assertRaises(TerminationRequested),
                termination_signal_handlers(),
            ):
                finalize_run_receipt(path, receipt, 0.0)

            decoded = json.loads(path.read_text(encoding="utf-8"))

        self.assertGreaterEqual(writes, 2)
        self.assertEqual(decoded["status"], "interrupted")


if __name__ == "__main__":
    unittest.main()
