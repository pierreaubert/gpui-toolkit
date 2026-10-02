#!/usr/bin/env python3
"""Send one command to a host launched with GPUI_TOOLKIT_DEV_API=<directory>."""

import argparse
import json
import os
from pathlib import Path
import time
import uuid


def request(directory, command, timeout=10.0):
    """Submit one atomic request; callers must serialize access to a host."""
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    request_id = uuid.uuid4().hex
    if (directory / "request.json").exists():
        raise RuntimeError("A request is already pending in this API directory")
    (directory / "response.json").unlink(missing_ok=True)
    temporary = directory / f"request-{request_id}.tmp"
    temporary.write_text(json.dumps(dict(command, id=request_id)))
    os.replace(temporary, directory / "request.json")
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            response = json.loads((directory / "response.json").read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            response = {}
        if response.get("id") == request_id:
            if not response["ok"]:
                raise RuntimeError(response["error"])
            return response["result"]
        if response.get("ok") is False and "id" not in response:
            raise RuntimeError(response["error"])
        time.sleep(0.025)
    raise TimeoutError("Native host did not answer; check GPUI_TOOLKIT_DEV_API and its process")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory")
    parser.add_argument("command", help='JSON, e.g. {"command":"status"}')
    args = parser.parse_args()
    print(json.dumps(request(args.directory, json.loads(args.command)), indent=2))


if __name__ == "__main__":
    main()
