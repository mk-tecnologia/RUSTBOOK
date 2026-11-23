#!/usr/bin/env python3
"""Restricted host agent: inspect hbbs and apply only its advertised relay.

The web container never receives Docker access. This daemon accepts a small JSON
protocol over a UID-10001-only Unix socket inside the existing address-book bind.
"""
import argparse
import base64
import fcntl
import hashlib
import json
import logging
import os
import platform
from pathlib import Path
import re
import shlex
import signal
import socketserver
import stat
import subprocess
import tempfile
import threading
import time

SOCKET = ".rustbook-server-agent.sock"
INSTALL_LOCK = Path("/run/lock/rustbook-install.lock")


def command(*args, timeout=30):
    result = subprocess.run(args, check=True, text=True, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=timeout)
    return result.stdout.strip()


def write_private(path, content):
    path = Path(path)
    with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as target:
        os.chmod(target.name, 0o600)
        target.write(content)
        temporary = Path(target.name)
    temporary.replace(path)


def endpoint(value):
    if not isinstance(value, str) or len(value) > 255 or any(c.isspace() for c in value):
        raise ValueError("Invalid relay endpoint")
    match = re.fullmatch(r"(\[[0-9A-Fa-f:.]+\]|[A-Za-z0-9_.-]+):([0-9]{1,5})", value)
    if not match or not 1 <= int(match[2]) <= 65535 or match[1].startswith("-"):
        raise ValueError("Use relay hostname:port, without protocol or credentials")
    return value


def relay_command(args, relay):
    args = list(args)
    positions = [i for i, item in enumerate(args) if item in ("-r", "--relay-servers") or item.startswith("--relay-servers=")]
    if len(positions) > 1:
        raise ValueError("Ambiguous relay flags; configure a single -r argument first")
    if positions:
        index = positions[0]
        if args[index].startswith("--relay-servers="):
            args[index] = "--relay-servers=" + relay
        else:
            if index + 1 >= len(args):
                raise ValueError("Relay argument has no value")
            args[index + 1] = relay
    else:
        args += ["-r", relay]
    return args


def advertised(args):
    for index, item in enumerate(args):
        if item in ("-r", "--relay-servers"):
            return args[index + 1] if index + 1 < len(args) else ""
        if item.startswith("--relay-servers="):
            return item.split("=", 1)[1]
    return ""


def patch_compose(text, service, args):
    """Patch only the selected service command, retaining YAML and all other fields."""
    args = [item.replace("$", "$$") for item in args]
    if text.lstrip().startswith("{"):
        document = json.loads(text)
        if service not in document.get("services", {}):
            return None
        if "command" not in document["services"][service]:
            return None
        document["services"][service]["command"] = args
        return json.dumps(document, indent=2) + "\n"
    lines = text.splitlines(keepends=True)
    services = next((i for i, line in enumerate(lines) if re.fullmatch(r"services:\s*(?:#.*)?\n?", line)), None)
    if services is None:
        return None
    end = next((i for i in range(services + 1, len(lines)) if lines[i].strip() and not lines[i].startswith((" ", "#", "\n"))), len(lines))
    headers = [(i, len(line) - len(line.lstrip())) for i, line in enumerate(lines[services + 1:end], services + 1)
               if re.fullmatch(r"\s+[\w'\".-]+:\s*(?:#.*)?\n?", line)]
    if not headers:
        return None
    indent = min(size for _, size in headers)
    starts = [i for i, size in headers if size == indent and re.fullmatch(r"\s+[\"']?" + re.escape(service) + r"[\"']?:\s*(?:#.*)?\n?", lines[i])]
    if len(starts) != 1:
        return None
    start = starts[0]
    end = next((i for i, size in headers if i > start and size == indent), end)
    fields = [(i, len(lines[i]) - len(lines[i].lstrip())) for i in range(start + 1, end)
              if re.match(r"\s+[\w-]+:", lines[i])]
    if not fields:
        return None
    field_indent = min(size for _, size in fields)
    matches = [i for i, size in fields if size == field_indent and re.match(r"\s+command:", lines[i])]
    if len(matches) != 1:
        return None
    first = matches[0]
    last = first + 1
    while last < end:
        line = lines[last]
        if line.strip() and len(line) - len(line.lstrip()) <= field_indent:
            break
        last += 1
    replacement = " " * field_indent + "command: " + json.dumps(args, ensure_ascii=False) + "\n"
    return "".join(lines[:first]) + replacement + "".join(lines[last:])


class Agent:
    def __init__(self, config):
        self.config = config
        self.root = Path(config["root"])
        self.lock = threading.Lock()

    def compose(self, *args, override=False):
        call = ["docker", "compose", "--project-name", self.config["project"],
                "--project-directory", self.config["working_dir"]]
        for path in self.config["files"]:
            call += ["--file", path]
        if override:
            call += ["--file", str(self.root / "server-agent-override.json")]
        return command(*call, *args)

    def container(self):
        ids = command("docker", "ps", "-a", "--filter", "label=com.docker.compose.project=" + self.config["project"],
                      "--filter", "label=com.docker.compose.service=" + self.config["service"], "--format", "{{.ID}}").splitlines()
        if len(ids) != 1:
            raise ValueError("Expected exactly one container for the selected hbbs service")
        container = json.loads(command("docker", "inspect", ids[0]))[0]
        args = container["Config"].get("Cmd") or []
        entrypoint = container["Config"].get("Entrypoint") or []
        if not args or (Path(args[0]).name != "hbbs" and not any(Path(item).name == "hbbs" for item in entrypoint)):
            raise ValueError("Selected service does not run hbbs")
        return container

    def keys(self, container):
        workdir = container["Config"].get("WorkingDir") or "/root"
        candidates = sorted(container.get("Mounts", []), key=lambda item: len(item["Destination"]), reverse=True)
        for mount in candidates:
            destination = Path(mount["Destination"])
            try:
                relative = Path(workdir).relative_to(destination)
            except ValueError:
                continue
            source = Path(mount["Source"]) / relative
            private, public = source / "id_ed25519", source / "id_ed25519.pub"
            if private.is_file() and public.is_file() and not private.is_symlink() and not public.is_symlink():
                try:
                    secret = base64.b64decode(private.read_bytes().strip(), validate=True)
                    public_text = public.read_text().strip()
                    public_bytes = base64.b64decode(public_text, validate=True)
                except ValueError:
                    raise ValueError("Persisted hbbs keypair has an unsupported format") from None
                if len(secret) != 64 or len(public_bytes) != 32 or secret[32:] != public_bytes:
                    raise ValueError("Persisted hbbs private/public keypair does not match")
                return {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in (private, public)}, public_text
        raise ValueError("Cannot locate the existing persisted hbbs keypair; no restart is allowed")

    def status(self):
        current = self.container()
        reason = ""
        try:
            _, key = self.keys(current)
            key_safe = len(base64.b64decode(key, validate=True)) == 32
            effective = self.validate_mounts(current)
            editable = any(patch_compose(Path(name).read_text(), self.config["service"], current["Config"]["Cmd"]) is not None
                           for name in self.config["files"] if not Path(name).is_symlink())
            if not editable:
                reason = "Cannot safely locate an explicit hbbs command in Compose"
        except (ValueError, OSError, subprocess.SubprocessError):
            key, key_safe, editable, effective = "", False, False, {}
            reason = "Existing keypair or deployment mounts could not be verified"
        running = bool(current["State"].get("Running"))
        return {"connected": True, "running": running,
                "relay": advertised(current["Config"]["Cmd"]), "configured_relay": advertised(effective.get("command") or []), "public_key": key if key_safe else "",
                "can_apply": key_safe and editable and running, "service": self.config["service"],
                "compose_file": self.config["files"][0], "error": reason}

    def validate_mounts(self, current):
        resolved = json.loads(self.compose("config", "--format", "json"))
        service = resolved["services"][self.config["service"]]
        actual = {item["Destination"]: item for item in current.get("Mounts", [])}
        expected = service.get("volumes", [])
        for volume in expected:
            target = volume.get("target")
            mounted = actual.get(target)
            if not mounted:
                raise ValueError("Compose mount differs from the running container; review deployment first")
            source = volume.get("source", "")
            if volume.get("type") == "volume":
                source = resolved.get("volumes", {}).get(source, {}).get("name", source)
                if mounted.get("Name") != source:
                    raise ValueError("Named volume differs from the running container")
            elif volume.get("type") == "bind" and str(Path(source).resolve()) != str(Path(mounted["Source"]).resolve()):
                raise ValueError("Data bind differs from the running container")
        if len(expected) != len(actual):
            raise ValueError("Compose volumes differ from the running container")
        return service

    def wait_running(self, image, args):
        for _ in range(10):
            current = self.container()
            if current["State"].get("Running") and current["Image"] == image and current["Config"]["Cmd"] == args:
                return current
            time.sleep(1)
        raise ValueError("hbbs did not start with the expected image and relay arguments")

    def apply(self, relay):
        relay = endpoint(relay)
        logging.info("Applying advertised relay to configured hbbs service: %s", relay)
        if not self.lock.acquire(blocking=False):
            raise ValueError("Another hbbs change is in progress")
        try:
            with INSTALL_LOCK.open("w") as lock:
                try:
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    raise ValueError("An address-book update is in progress; retry afterwards") from None
                return self._apply(relay)
        finally:
            self.lock.release()

    def _apply(self, relay):
        current = self.container()
        if not current["State"].get("Running"):
            raise ValueError("hbbs must be running before it can be managed")
        hashes, _ = self.keys(current)
        effective = self.validate_mounts(current)
        old_args = current["Config"]["Cmd"]
        args = relay_command(old_args, relay)
        if args == old_args and effective.get("command") == args:
            return {"ok": True, "changed": False, "restarted": False, "relay": relay, "keys_preserved": True}
        selected = None
        for name in reversed(self.config["files"]):
            path = Path(name)
            if path.is_symlink() or not path.is_file():
                raise ValueError("Compose source must be a regular file")
            original = path.read_text()
            changed = patch_compose(original, self.config["service"], args)
            if changed is not None:
                selected = path, original, changed, path.stat()
                break
        if not selected:
            raise ValueError("Cannot safely locate the hbbs command in Compose; review it manually")
        path, original, changed, original_stat = selected
        mode = original_stat.st_mode & 0o777
        backup = self.root / "server-backups" / (time.strftime("%Y%m%d-%H%M%S") + "-" + str(time.time_ns()))
        backup.mkdir(parents=True, mode=0o700)
        write_private(backup / "compose.original", original)
        key_snapshots = []
        for index, name in enumerate(hashes):
            key = Path(name)
            snapshot = backup / f"key-{index}"
            snapshot.write_bytes(key.read_bytes())
            os.chmod(snapshot, 0o600)
            if hashlib.sha256(snapshot.read_bytes()).hexdigest() != hashes[str(key)]:
                raise ValueError("Keypair changed during preparation; retry after reviewing the host")
            key_snapshots.append((key, snapshot, key.stat()))
        override_path = self.root / "server-agent-override.json"
        previous_override = override_path.read_text() if override_path.exists() else None
        environment = dict(item.split("=", 1) for item in current["Config"].get("Env", []) if "=" in item)
        override = {"services": {self.config["service"]: {
            "image": current["Image"], "pull_policy": "never", "command": [item.replace("$", "$$") for item in args],
            "environment": {key: value.replace("$", "$$") for key, value in environment.items()}}}}
        write_private(backup / "metadata.json", json.dumps({"source": str(path), "image": current["Image"], "command": old_args}))
        started = False
        if path.read_text() != original:
            raise ValueError("Compose changed during preparation; no change applied")
        try:
            write_private(path, changed)
            os.chmod(path, mode)
            os.chown(path, original_stat.st_uid, original_stat.st_gid)
            write_private(override_path, json.dumps(override))
            self.compose("config", "--quiet", override=True)
            if args != old_args:
                started = True
                self.compose("up", "-d", "--no-deps", "--no-build", "--pull", "never", self.config["service"], override=True)
            fresh = self.wait_running(current["Image"], args)
            fresh_hashes, _ = self.keys(fresh)
            if fresh_hashes != hashes:
                raise ValueError("Keypair changed unexpectedly")
        except Exception:
            write_private(path, original)
            os.chmod(path, mode)
            os.chown(path, original_stat.st_uid, original_stat.st_gid)
            override["services"][self.config["service"]]["command"] = [item.replace("$", "$$") for item in old_args]
            write_private(override_path, json.dumps(override))
            if started:
                altered = [(key, snapshot, status) for key, snapshot, status in key_snapshots
                           if not key.is_file() or hashlib.sha256(key.read_bytes()).hexdigest() != hashes[str(key)]]
                if altered:
                    self.compose("stop", self.config["service"], override=True)
                    for key, snapshot, status in altered:
                        if key.is_symlink():
                            raise ValueError("Unexpected key symlink; manual recovery required") from None
                        key.write_bytes(snapshot.read_bytes())
                        os.chmod(key, status.st_mode & 0o777)
                        os.chown(key, status.st_uid, status.st_gid)
                self.compose("up", "-d", "--no-deps", "--no-build", "--pull", "never", self.config["service"], override=True)
                restored = self.wait_running(current["Image"], old_args)
                restored_hashes, _ = self.keys(restored)
                if restored_hashes != hashes:
                    raise ValueError("Rollback keypair could not be confirmed; recover from the host agent backup") from None
            if previous_override is not None:
                write_private(override_path, previous_override)
            elif not started:
                override_path.unlink(missing_ok=True)
            raise ValueError("hbbs update failed; previous command restored. Review host agent logs.") from None

        return {"ok": True, "changed": True, "restarted": started, "relay": relay, "keys_preserved": True,
                "notice": "hbbs command and unchanged keypair verified. Test a relay session between external clients."}


class Handler(socketserver.StreamRequestHandler):
    def setup(self):
        self.request.settimeout(5)
        super().setup()

    def handle(self):
        try:
            request = json.loads(self.rfile.readline(16385))
            if request == {"action": "status"}:
                result = self.server.agent.status()
            elif set(request) == {"action", "relay"} and request["action"] == "apply_relay":
                result = self.server.agent.apply(request["relay"])
            else:
                raise ValueError("Unsupported agent operation")
        except Exception as error:
            logging.exception("Restricted hbbs agent operation failed")
            result = {"ok": False, "error": str(error) if isinstance(error, ValueError) else "Host operation failed; consult systemd agent logs"}
        try:
            self.wfile.write(json.dumps(result).encode() + b"\n")
        except OSError:
            pass


class Server(socketserver.ThreadingUnixStreamServer):
    daemon_threads = False


def discover(compose_file, service):
    compose_file = str(Path(compose_file).resolve())
    if not re.fullmatch(r"[A-Za-z0-9_-]+", service):
        raise ValueError("Invalid Compose service name")
    ids = command("docker", "ps", "-a", "--filter", "label=com.docker.compose.service=" + service, "--format", "{{.ID}}").splitlines()
    matches = []
    for item in ids:
        current = json.loads(command("docker", "inspect", item))[0]
        labels = current["Config"].get("Labels", {})
        files = labels.get("com.docker.compose.project.config_files", "").split(",")
        if compose_file in files:
            matches.append({"files": [f for f in files if not f.endswith("/server-agent-override.json")],
                            "project": labels["com.docker.compose.project"],
                            "working_dir": labels.get("com.docker.compose.project.working_dir", str(Path(compose_file).parent)), "service": service})
    if len(matches) != 1:
        raise ValueError("Could not identify exactly one existing hbbs for this Compose file")
    return matches[0]


def install(args):
    root = Path(args.root).resolve()
    data = Path(args.data).resolve()
    if not data.is_dir():
        raise ValueError("Address-book data bind must already exist")
    config = discover(args.compose, args.service)
    config.update(root=str(root), data=str(data))
    agent = Agent(config)
    current = agent.container()
    agent.keys(current)
    agent.validate_mounts(current)
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    write_private(root / "server-agent.json", json.dumps(config, indent=2))
    script = root / "server-agent.py"
    if Path(__file__).resolve() != script:
        write_private(script, Path(__file__).read_text())
    unit_args = ["/usr/bin/python3", str(script), "serve", "--config", str(root / "server-agent.json")]
    unit = ("[Unit]\nDescription=RUSTBOOK restricted hbbs configuration agent\nAfter=docker.service\nRequires=docker.service\n\n"
            "[Service]\nType=simple\nExecStart=" + shlex.join(unit_args).replace("%", "%%") + "\nRestart=on-failure\nRestartSec=3\nUMask=0077\nNoNewPrivileges=true\nTimeoutStopSec=180\n\n[Install]\nWantedBy=multi-user.target\n")
    write_private(Path("/etc/systemd/system/rustbook-server-agent.service"), unit)
    command("systemctl", "daemon-reload")
    command("systemctl", "enable", "rustbook-server-agent.service")
    command("systemctl", "restart", "rustbook-server-agent.service", timeout=180)
    print("Agente instalado. Nenhum hbbs/hbbr foi reiniciado. Use Salvar e Aplicar relay no painel.")


def serve(config_file):
    config = json.loads(Path(config_file).read_text())
    path = Path(config["data"]) / SOCKET
    if len(str(path).encode()) > 100:
        raise ValueError("Data directory path is too long for an agent socket")
    with (Path(config["root"]) / "server-agent.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if path.exists() or path.is_symlink():
            if path.is_symlink() or not stat.S_ISSOCK(path.lstat().st_mode):
                raise ValueError("Refusing to replace a non-socket runtime file")
            path.unlink()
        with Server(str(path), Handler) as server:
            os.chown(path, 10001, 10001)
            os.chmod(path, 0o600)
            server.agent = Agent(config)
            signal.signal(signal.SIGTERM, lambda *_: threading.Thread(target=server.shutdown, daemon=True).start())
            try:
                server.serve_forever()
            finally:
                server.server_close()
                path.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    setup = sub.add_parser("install")
    setup.add_argument("--root", required=True)
    setup.add_argument("--data", required=True)
    setup.add_argument("--compose", required=True)
    setup.add_argument("--service", default="hbbs")
    run = sub.add_parser("serve")
    run.add_argument("--config", required=True)
    args = parser.parse_args()
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
    if platform.system() != "Linux" or os.geteuid() != 0:
        parser.error("Run as root on the Linux Docker host")
    if args.action == "install":
        install(args)
    else:
        serve(args.config)


if __name__ == "__main__":
    main()
