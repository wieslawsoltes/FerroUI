#!/usr/bin/env python3
"""Transport one authenticated release-evidence ZIP as three unchanged byte parts.

No archive is extracted, rebuilt or recompressed here. Authorization is sent
only to api.github.com with redirects disabled. Signed blob URLs are consumed
by a separate unauthenticated opener and never written to logs or manifests.
See https://docs.github.com/en/rest/actions/artifacts#download-an-artifact .
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import sys
import urllib.error
import urllib.parse
import urllib.request


PART_BYTES = 448 * 1024 * 1024
CONNECTOR_LIMIT = 512 * 1024 * 1024
BUFFER_BYTES = 1024 * 1024
DEFAULTS = {
    "repository": "wieslawsoltes/FerroUI", "repository_id": 1405576478,
    "run_id": 41815903284, "run_attempt": 1, "artifact_id": 13318422418,
    "head_sha": "5fd34166cb1f01a2d482e87030e6d9cb5d6cb1f0",
    "head_branch": "perf/compact-xaml-theme-codegen",
    "artifact_name": "winui-editable-release-comparison-41815903284-1",
    "size": 1261081055,
    "sha256": "f838b6f512a1fc5f19e22aa81d8c01b634633f0530125c89f20504ce1a4ae6a3",
}


class TransferError(RuntimeError):
    """A diagnostic containing no credentials, signed URL or network body."""


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def require(condition, message):
    if not condition:
        raise TransferError(message)


def identity(path: Path) -> dict:
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"bytes": path.stat().st_size, "sha256": digest}


def save(path: Path, value: dict):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def split_file(source: Path, directory: Path, expected: dict, part_bytes=PART_BYTES) -> list[dict]:
    """Split and reconstruct-hash bytes; does not claim CI metadata validation."""
    require(part_bytes > 0, "Part size must be positive")
    directory.mkdir(parents=True, exist_ok=False)
    parts, offset, input_hash = [], 0, hashlib.sha256()
    with source.open("rb") as stream:
        while offset < expected["bytes"]:
            number = len(parts) + 1
            part_directory = directory / f"part-{number:03d}"
            part_directory.mkdir()
            name = f"release-evidence.zip.part-{number:03d}"
            path = part_directory / name
            wanted = min(part_bytes, expected["bytes"] - offset)
            size, part_hash = 0, hashlib.sha256()
            with path.open("xb") as output:
                while size < wanted:
                    chunk = stream.read(min(BUFFER_BYTES, wanted - size))
                    require(bool(chunk), "Original byte stream ended before the expected size")
                    output.write(chunk)
                    size += len(chunk)
                    part_hash.update(chunk)
                    input_hash.update(chunk)
            actual = identity(path)
            require(actual == {"bytes": wanted, "sha256": part_hash.hexdigest()}, "Written part differs from input bytes")
            parts.append({"number": number, "directory": part_directory.name, "file": name,
                          "offset": offset, **actual})
            offset += size
        require(not stream.read(1), "Original byte stream exceeds the expected size")
    require(offset == expected["bytes"] and input_hash.hexdigest() == expected["sha256"],
            "Split input does not match the original ZIP identity")
    reconstructed, reconstructed_bytes = hashlib.sha256(), 0
    for part in parts:
        require(part["offset"] == reconstructed_bytes, "Part offsets are not contiguous")
        with (directory / part["directory"] / part["file"]).open("rb") as stream:
            while chunk := stream.read(BUFFER_BYTES):
                reconstructed.update(chunk)
                reconstructed_bytes += len(chunk)
    require(reconstructed_bytes == expected["bytes"] and reconstructed.hexdigest() == expected["sha256"],
            "Ordered parts do not reconstruct the exact original ZIP")
    return parts


def blob_url(url: str) -> str:
    parsed = urllib.parse.urlsplit(url)
    host = (parsed.hostname or "").lower()
    require(parsed.scheme == "https" and parsed.port in (None, 443) and not parsed.username and not parsed.password,
            "Artifact redirect must be a credential-free HTTPS blob URL")
    require(host.endswith((".blob.core.windows.net", ".githubusercontent.com", ".amazonaws.com")),
            "Artifact redirect has an unsupported blob origin")
    return host


class Transfer:
    def __init__(self, args):
        self.args = args
        self.output = args.output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.evidence = self.output / "evidence"
        self.evidence.mkdir()
        self.report = {
            "schema_version": 1, "scope": __doc__, "complete": False, "stage": "initialization",
            "expected": {key: getattr(args, key) for key in DEFAULTS},
            "producer": {"file": Path(__file__).name, **identity(Path(__file__))},
            "part_bytes": PART_BYTES, "connector_limit_bytes": CONNECTOR_LIMIT,
            "metadata_verified": False, "download_verified": False, "reassembly_verified": False,
            "network": [], "parts": [],
        }
        self.api = urllib.request.build_opener(NoRedirect())
        self.blob = urllib.request.build_opener(NoRedirect())
        self.persist()

    def persist(self):
        save(self.evidence / "transfer.json", self.report)

    def request(self, path):
        # The fixed API origin and disabled redirects keep the bearer token
        # away from any blob/CDN request, including redirected error paths.
        return urllib.request.Request("https://api.github.com" + path, headers={
            "Authorization": "Bearer " + self.token, "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "FerroUI-release-evidence-transfer",
        })

    def metadata(self, path, label):
        row = {"kind": "authenticated-api", "path": path}
        self.report["network"].append(row)
        self.persist()
        with self.api.open(self.request(path), timeout=60) as response:
            row["status"] = response.status
            require(response.status == 200, "Metadata request did not return HTTP200")
            raw = response.read(4 * 1024 * 1024 + 1)
            require(len(raw) <= 4 * 1024 * 1024, "Metadata response exceeds the bounded size")
        result = json.loads(raw)
        destination = self.evidence / (label + ".json")
        destination.write_bytes(raw)
        row["response"] = {"file": destination.name, **identity(destination)}
        self.persist()
        return result

    def download(self, path):
        self.report["stage"] = "download"
        self.persist()
        try:
            with self.api.open(self.request(path), timeout=60):
                raise TransferError("Download API did not return the required HTTP302 redirect")
        except urllib.error.HTTPError as error:
            if error.code != 302:
                raise
            location = error.headers.get("Location", "")
            error.close()
        # Deliberately do not retain Location or any signed query parameters.
        destination = self.output / "original-evidence.zip"
        for _ in range(4):
            host = blob_url(location)
            row = {"kind": "unauthenticated-blob", "origin": "https://" + host}
            self.report["network"].append(row)
            self.persist()
            request = urllib.request.Request(location, headers={"User-Agent": "FerroUI-release-evidence-transfer",
                                                                "Accept-Encoding": "identity"})
            try:
                response = self.blob.open(request, timeout=60)
            except urllib.error.HTTPError as error:
                row["status"] = error.code
                if error.code not in (301, 302, 303, 307, 308):
                    raise
                location = urllib.parse.urljoin(location, error.headers.get("Location", ""))
                error.close()
                continue
            with response, destination.open("xb") as output:
                row["status"] = response.status
                require(response.status == 200, "Blob request did not return HTTP200")
                require(response.headers.get("Content-Encoding", "identity") == "identity", "Blob content encoding changed the byte stream")
                length = response.headers.get("Content-Length")
                if length is not None:
                    require(int(length) == self.args.size, "Blob Content-Length differs from the original ZIP size")
                size, digest = 0, hashlib.sha256()
                while chunk := response.read(BUFFER_BYTES):
                    size += len(chunk)
                    require(size <= self.args.size, "Blob exceeds the expected ZIP size")
                    output.write(chunk)
                    digest.update(chunk)
            actual = {"bytes": size, "sha256": digest.hexdigest()}
            self.report["download"] = actual
            self.persist()
            require(actual == {"bytes": self.args.size, "sha256": self.args.sha256}, "Downloaded ZIP identity differs")
            self.report["download_verified"] = True
            self.persist()
            return destination
        raise TransferError("Blob redirect chain exceeded the supported bound")

    def execute(self):
        a = self.args
        require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", a.repository), "Invalid repository slug")
        require(re.fullmatch(r"[0-9a-f]{40}", a.head_sha) and re.fullmatch(r"[0-9a-f]{64}", a.sha256), "Invalid expected source/digest")
        require(all(getattr(a, key) > 0 for key in ("repository_id", "run_id", "run_attempt", "artifact_id", "size")), "IDs and size must be positive")
        require((a.size + PART_BYTES - 1) // PART_BYTES == 3, "This workflow transports exactly three parts")
        require(os.environ.get("GITHUB_REPOSITORY", "").lower() == a.repository.lower(), "Transfer must run in the source repository")
        self.token = os.environ.get("GITHUB_TOKEN", "")
        require(bool(self.token), "GITHUB_TOKEN with read-only Actions access is required")
        self.report["stage"] = "metadata"
        base = "/repos/" + a.repository
        repository = self.metadata(base, "repository")
        run = self.metadata(base + f"/actions/runs/{a.run_id}", "run")
        artifact = self.metadata(base + f"/actions/artifacts/{a.artifact_id}", "artifact")
        require(repository["id"] == a.repository_id and repository["full_name"].lower() == a.repository.lower(), "Repository identity differs")
        require(run["id"] == a.run_id and run["run_attempt"] == a.run_attempt and run["status"] == "completed"
                and run["conclusion"] == "success", "Source run is not the exact successful completed attempt")
        require(run["head_sha"] == a.head_sha and run["head_branch"] == a.head_branch, "Source run head differs")
        require(run["repository"]["id"] == run["head_repository"]["id"] == a.repository_id, "Source run is not from the same repository")
        require(artifact["id"] == a.artifact_id and artifact["name"] == a.artifact_name and not artifact["expired"], "Artifact identity/name/expiry differs")
        require(artifact["size_in_bytes"] == a.size and artifact["digest"] == "sha256:" + a.sha256, "Artifact size or digest differs")
        origin = artifact["workflow_run"]
        require(origin["id"] == a.run_id and origin["repository_id"] == origin["head_repository_id"] == a.repository_id
                and origin["head_sha"] == a.head_sha and origin["head_branch"] == a.head_branch, "Artifact belongs to another run/head/repository")
        archive_path = base + f"/actions/artifacts/{a.artifact_id}/zip"
        require(artifact["archive_download_url"] == "https://api.github.com" + archive_path, "Artifact download API endpoint differs")
        self.report["metadata_verified"] = True
        self.persist()
        original = self.download(archive_path)
        self.report["stage"] = "split"
        self.persist()
        parts = split_file(original, self.output / "parts", {"bytes": a.size, "sha256": a.sha256})
        require(len(parts) == 3, "Unexpected part count")
        self.report.update(parts=parts, reassembly_verified=True, stage="part-manifests")
        self.persist()
        contract = {**self.report, "stage": "complete", "complete": True}
        shared = (json.dumps(contract, indent=2, sort_keys=True) + "\n").encode()
        # Two bounded filenames per part and 64MiB headroom leave room for the
        # upload action's compression-level0 ZIP framing. Actual uploaded ZIP
        # sizes/digests must still be authenticated when downloading them.
        for part in parts:
            require(part["bytes"] + len(shared) + 1024 * 1024 < CONNECTOR_LIMIT, "Part lacks the required upload framing headroom")
            path = self.output / "parts" / part["directory"] / "manifest.json"
            path.write_bytes(shared)
            require(path.read_bytes() == shared, "Part manifest copy changed bytes")
        self.report["part_manifests"] = {"bytes": len(shared), "sha256": hashlib.sha256(shared).hexdigest()}
        # The identical per-part manifest is the reconstruction contract. This
        # outer status adds its identity without changing those copies.
        self.report.update(stage="complete", complete=True)
        self.persist()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key, value in DEFAULTS.items():
        parser.add_argument("--" + key.replace("_", "-"), type=int if isinstance(value, int) else str,
                            default=os.environ.get("TRANSFER_" + key.upper(), value))
    parser.add_argument("--output", type=Path, default=os.environ.get("TRANSFER_OUTPUT"), required="TRANSFER_OUTPUT" not in os.environ)
    args = parser.parse_args()
    transfer = Transfer(args)
    def terminated(signum, _frame):
        raise TransferError(f"Transfer interrupted by signal {signum}")
    signal.signal(signal.SIGTERM, terminated)
    try:
        transfer.execute()
    except BaseException as error:
        # Network exception strings may contain signed URLs. Retain only a
        # type/status; TransferError messages are deliberately credential-free.
        failure = {"type": type(error).__name__}
        if isinstance(error, TransferError):
            failure["message"] = str(error)
        if isinstance(error, urllib.error.HTTPError):
            failure["http_status"] = error.code
        transfer.report.update(complete=False, error=failure)
        transfer.persist()
        print(json.dumps({"complete": False, "stage": transfer.report["stage"], "error": failure}), flush=True)
        return 1
    print(json.dumps({"complete": True, "artifact_id": args.artifact_id, "original": transfer.report["download"],
                      "parts": transfer.report["parts"]}, sort_keys=True), flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
