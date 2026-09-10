from __future__ import annotations

import csv
import io
import json
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

FIELDS = (
    "operation_id",
    "tx_hash",
    "signed_operation_data",
    "slot",
    "block_number",
    "absn",
    "osn",
    "indexed",
    "operation_kind",
    "did",
)


class ManifestError(ValueError):
    """Raised when an exported operation manifest is not canonical."""


@dataclass(frozen=True, slots=True)
class OperationRecord:
    operation_id: str
    tx_hash: str
    signed_operation_data: str
    slot: int
    block_number: int
    absn: int
    osn: int
    indexed: bool
    operation_kind: str
    did: str

    @classmethod
    def from_row(cls, row: dict[str, str]) -> OperationRecord:
        missing = set(FIELDS).difference(row)
        if missing:
            raise ManifestError(f"manifest row is missing fields: {sorted(missing)}")
        operation_id = row["operation_id"].lower()
        if not operation_id:
            raise ManifestError("manifest row has an empty operation_id")
        indexed_value = row["indexed"].lower()
        if indexed_value not in {"true", "false"}:
            raise ManifestError(f"invalid indexed value: {indexed_value}")
        try:
            return cls(
                operation_id=operation_id,
                tx_hash=row["tx_hash"].lower(),
                signed_operation_data=row["signed_operation_data"].lower(),
                slot=int(row["slot"]),
                block_number=int(row["block_number"]),
                absn=int(row["absn"]),
                osn=int(row["osn"]),
                indexed=indexed_value == "true",
                operation_kind=row["operation_kind"].lower(),
                did=row["did"].lower(),
            )
        except ValueError as error:
            raise ManifestError(
                f"manifest row contains an invalid number: {row}"
            ) from error

    def csv_row(self) -> dict[str, str | int]:
        row = asdict(self)
        row["indexed"] = "true" if self.indexed else "false"
        return row


@dataclass(frozen=True, slots=True)
class ChangedOperation:
    operation_id: str
    baseline: OperationRecord
    candidate: OperationRecord


@dataclass(frozen=True, slots=True)
class Comparison:
    baseline_count: int
    candidate_count: int
    duplicate_baseline_ids: tuple[str, ...]
    duplicate_candidate_ids: tuple[str, ...]
    missing_from_candidate: tuple[OperationRecord, ...]
    missing_from_baseline: tuple[OperationRecord, ...]
    changed: tuple[ChangedOperation, ...]

    @property
    def equal(self) -> bool:
        return not any(
            (
                self.duplicate_baseline_ids,
                self.duplicate_candidate_ids,
                self.missing_from_candidate,
                self.missing_from_baseline,
                self.changed,
            )
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "equal": self.equal,
            "baseline_count": self.baseline_count,
            "candidate_count": self.candidate_count,
            "duplicate_baseline_ids": list(self.duplicate_baseline_ids),
            "duplicate_candidate_ids": list(self.duplicate_candidate_ids),
            "missing_from_candidate": [
                asdict(item) for item in self.missing_from_candidate
            ],
            "missing_from_baseline": [
                asdict(item) for item in self.missing_from_baseline
            ],
            "changed": [asdict(item) for item in self.changed],
        }


def read_manifest(text: str) -> list[OperationRecord]:
    reader = csv.DictReader(io.StringIO(text))
    if reader.fieldnames != list(FIELDS):
        raise ManifestError(
            f"manifest fields do not match canonical fields: {reader.fieldnames}"
        )
    return sorted(
        (OperationRecord.from_row(dict(row)) for row in reader),
        key=lambda item: item.operation_id,
    )


def write_manifest(path: Path, records: list[OperationRecord]) -> None:
    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.writer(output, lineterminator="\n")
        writer.writerow(FIELDS)
        for record in records:
            row = record.csv_row()
            writer.writerow([row[field] for field in FIELDS])


def _duplicates(records: list[OperationRecord]) -> tuple[str, ...]:
    counts = Counter(record.operation_id for record in records)
    return tuple(
        sorted(operation_id for operation_id, count in counts.items() if count > 1)
    )


def compare(
    baseline: list[OperationRecord], candidate: list[OperationRecord]
) -> Comparison:
    duplicate_baseline_ids = _duplicates(baseline)
    duplicate_candidate_ids = _duplicates(candidate)
    baseline_by_id = {record.operation_id: record for record in baseline}
    candidate_by_id = {record.operation_id: record for record in candidate}
    missing_from_candidate = tuple(
        baseline_by_id[key]
        for key in sorted(baseline_by_id.keys() - candidate_by_id.keys())
    )
    missing_from_baseline = tuple(
        candidate_by_id[key]
        for key in sorted(candidate_by_id.keys() - baseline_by_id.keys())
    )
    changed = tuple(
        ChangedOperation(key, baseline_by_id[key], candidate_by_id[key])
        for key in sorted(baseline_by_id.keys() & candidate_by_id.keys())
        if baseline_by_id[key] != candidate_by_id[key]
    )
    return Comparison(
        baseline_count=len(baseline),
        candidate_count=len(candidate),
        duplicate_baseline_ids=duplicate_baseline_ids,
        duplicate_candidate_ids=duplicate_candidate_ids,
        missing_from_candidate=missing_from_candidate,
        missing_from_baseline=missing_from_baseline,
        changed=changed,
    )


def write_reports(
    output_dir: Path, comparison: Comparison, metadata: dict[str, Any]
) -> None:
    json_report = {"metadata": metadata, "comparison": comparison.to_dict()}
    (output_dir / "report.json").write_text(
        json.dumps(json_report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    status = "PASS" if comparison.equal else "FAIL"
    lines = [
        "# NeoPRISM Indexer Parity Report",
        "",
        f"Result: **{status}**",
        "",
        f"- Network: `{metadata['network']}`",
        f"- Comparison slot: `{metadata['comparison_slot']}`",
        f"- Baseline image: `{metadata['baseline_image']}`",
        f"- Baseline identity: `{metadata['baseline_identity']}`",
        f"- Candidate image: `{metadata['candidate_image']}`",
        f"- Candidate identity: `{metadata['candidate_identity']}`",
        f"- Baseline cursor: `{metadata['baseline_cursor']}`",
        f"- Candidate cursor: `{metadata['candidate_cursor']}`",
        f"- Baseline operations: `{comparison.baseline_count}`",
        f"- Candidate operations: `{comparison.candidate_count}`",
        "",
        "## Differences",
        "",
        f"- Duplicate baseline IDs: `{len(comparison.duplicate_baseline_ids)}`",
        f"- Duplicate candidate IDs: `{len(comparison.duplicate_candidate_ids)}`",
        f"- Missing from candidate: `{len(comparison.missing_from_candidate)}`",
        f"- Missing from baseline: `{len(comparison.missing_from_baseline)}`",
        f"- Changed operations: `{len(comparison.changed)}`",
        "",
        "See `report.json` for operation-level details.",
    ]
    (output_dir / "report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
