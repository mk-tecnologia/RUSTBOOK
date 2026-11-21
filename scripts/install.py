#!/usr/bin/env python3
"""Install/update the standalone address book on a Linux Docker host."""

import argparse
import fcntl
import json
import os
from pathlib import Path
import platform
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.request


REPOSITORY = "https://github.com/mk-tecnologia/RUSTBOOK.git"
CONTAINER = "rustdesk-address-book"


def run(*args, capture=False):
    result = subprocess.run(args, check=True, text=True,
                            stdout=subprocess.PIPE if capture else None)
    return result.stdout.strip() if capture else None


def write_json(path, value):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    os.chmod(temporary, 0o600)
    temporary.replace(path)


def compose_document(image):
    return {"services": {"address-book": {
        "image": image,
        "container_name": CONTAINER,
        "ports": ["${BIND_IP:-127.0.0.1}:21114:21114"],
        "volumes": ["./data:/data"],
        "env_file": [".env"],
        "environment": {"RUSTDESK_AB_DB_PATH": "/data/db.sqlite3"},
        "init": True,
        "restart": "unless-stopped",
        "stop_grace_period": "15s",
        "security_opt": ["no-new-privileges:true"],
        "cap_drop": ["ALL"],
    }}}


class Installer:
    def __init__(self, directory):
        self.root = directory.resolve()
        self.config = self.root / "compose.json"
        self.state_file = self.root / "install-state.json"
        self.env = self.root / ".env"
        self.data = self.root / "data"

    def compose(self, *args):
        return run("docker", "compose", "--project-name", "rustbook",
                   "--env-file", str(self.env), "--file", str(self.config), *args)

    def check_ownership(self):
        names = run("docker", "ps", "-a", "--format", "{{.Names}}", capture=True).splitlines()
        if CONTAINER in names:
            project = run("docker", "inspect", "--format",
                          '{{index .Config.Labels "com.docker.compose.project"}}',
                          CONTAINER, capture=True)
            config = run("docker", "inspect", "--format",
                         '{{index .Config.Labels "com.docker.compose.project.config_files"}}',
                         CONTAINER, capture=True)
            if (project != "rustbook" or config != str(self.config)
                    or not self.state_file.exists()):
                raise RuntimeError("Já existe uma agenda fora deste instalador. "
                                   "Migre conforme docs/INSTALACAO.md; nenhum container foi alterado.")
        if not self.state_file.exists() and (
            self.config.exists() or self.env.exists()
            or (self.data.exists() and any(self.data.iterdir()))
        ):
            raise RuntimeError("Diretório com configuração/dados não gerenciados. "
                               "Consulte docs/INSTALACAO.md antes de migrar.")

    def set_data_owner(self):
        for directory, subdirectories, files in os.walk(self.data):
            if Path(directory).is_symlink():
                raise RuntimeError("O diretório de dados não pode conter links simbólicos.")
            os.chown(directory, 10001, 10001)
            for name in subdirectories + files:
                path = Path(directory) / name
                if path.is_symlink():
                    raise RuntimeError("O diretório de dados não pode conter links simbólicos.")
                os.chown(path, 10001, 10001)

    def credentials(self):
        if self.env.exists():
            return
        password = secrets.token_hex(16)
        # Never print credentials into terminal logs. The file is root-readable only.
        with self.env.open("x") as target:
            os.chmod(self.env, 0o600)
            target.write("BIND_IP=127.0.0.1\n"
                         "RUSTDESK_AB_ADMIN_USERNAME=admin\n"
                         f"RUSTDESK_AB_ADMIN_PASSWORD={password}\n"
                         f"RUSTDESK_AB_JWT_SECRET={secrets.token_hex(32)}\n")

    def import_existing(self, source):
        """Import an offline standalone deployment once, without touching its files."""
        self.check_ownership()
        if self.state_file.exists():
            raise RuntimeError("Este destino já é gerenciado; remova --import-from para atualizar.")
        source = source.resolve()
        if (source == self.root or (source / "data").is_symlink()
                or not (source / "data/db.sqlite3").is_file()):
            raise RuntimeError("A origem deve conter data/db.sqlite3 e .env de uma agenda existente.")
        environment = (source / ".env").read_text()
        if not any(line.startswith("RUSTDESK_AB_JWT_SECRET=") and line.split("=", 1)[1].strip()
                   for line in environment.splitlines()):
            raise RuntimeError("A origem deve ter RUSTDESK_AB_JWT_SECRET preenchido no .env.")
        containers = run("docker", "ps", "-q", capture=True).splitlines()
        if containers:
            for container in json.loads(run("docker", "inspect", *containers, capture=True)):
                for mount in container.get("Mounts", []):
                    mounted = Path(mount.get("Source", "/")).resolve()
                    data = source / "data"
                    if mounted == data or mounted in data.parents or data in mounted.parents:
                        raise RuntimeError("Pare os containers que usam os dados antes de importar.")
        for path in (source / "data").rglob("*"):
            if path.is_symlink():
                raise RuntimeError("A origem não pode conter links simbólicos.")
        shutil.copytree(source / "data", self.data)
        self.env.write_text(environment)
        os.chmod(self.env, 0o600)
        write_json(self.state_file, {})
        print("Dados e credenciais importados; a origem foi preservada.", flush=True)

    def backup(self):
        name = time.strftime("%Y%m%d-%H%M%S") + "-" + secrets.token_hex(3)
        destination = self.root / "backups" / name
        destination.mkdir(parents=True, mode=0o700)
        shutil.copytree(self.data, destination / "data")
        for path in (self.env, self.config, self.state_file):
            if path.exists():
                shutil.copy2(path, destination / path.name)
        print(f"Backup: {destination}", flush=True)
        return destination

    def wait_ready(self, version):
        config = run("docker", "compose", "--project-name", "rustbook",
                     "--env-file", str(self.env), "--file", str(self.config),
                     "config", "--format", "json", capture=True)
        service = json.loads(config)["services"]["address-book"]
        host = service["ports"][0].get("host_ip", "127.0.0.1")
        if host in ("0.0.0.0", "::"):
            host = "127.0.0.1"
        if ":" in host:
            host = f"[{host}]"
        url = f"http://{host}:21114/api/version"
        client = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            try:
                with client.open(url, timeout=3) as response:
                    if json.load(response).get("version") == version:
                        print(f"Agenda disponível: {url}", flush=True)
                        return
            except (OSError, ValueError):
                pass
            time.sleep(2)
        raise RuntimeError("A agenda não respondeu com a versão esperada em 60 segundos.")

    def restore(self, backup, previous):
        self.compose("down")
        # Keep the failed database for diagnosis; never delete either database.
        failed = backup / "failed-data"
        self.data.rename(failed)
        shutil.copytree(backup / "data", self.data)
        self.set_data_owner()
        for name in (".env", "compose.json", "install-state.json"):
            saved = backup / name
            if saved.exists():
                shutil.copy2(saved, self.root / name)
        if previous.get("image"):
            self.compose("up", "-d", "--no-deps", "--no-build", "address-book")
            self.wait_ready(previous["version"])
            print("Versão e banco anteriores restaurados.", flush=True)

    def install(self):
        self.check_ownership()
        previous = json.loads(self.state_file.read_text()) if self.state_file.exists() else {}
        if previous.get("image") and (not self.env.is_file() or not self.data.is_dir()):
            raise RuntimeError("Configuração ou dados ausentes; restaure o backup antes de atualizar.")
        with tempfile.TemporaryDirectory(prefix="source-", dir=self.root) as temporary:
            source = Path(temporary) / "repo"
            run("git", "clone", "--depth", "1", "--branch", "main", "--", REPOSITORY, str(source))
            commit = run("git", "-C", str(source), "rev-parse", "HEAD", capture=True)
            with (source / "Cargo.toml").open("rb") as manifest:
                version = tomllib.load(manifest)["package"]["version"]
            image = f"rustbook:{commit}"
            print(f"Instalando {version}, commit {commit}", flush=True)
            # Build before stopping the old service. A failed build leaves it running.
            run("docker", "build", "--tag", image, str(source))
        self.credentials()
        self.data.mkdir(mode=0o700, exist_ok=True)
        if not self.state_file.exists():
            write_json(self.state_file, {})
        try:
            if previous.get("image"):
                self.compose("stop", "address-book")
            backup = self.backup()
        except Exception:
            if previous.get("image"):
                self.compose("start", "address-book")
            raise
        try:
            self.set_data_owner()
            write_json(self.config, compose_document(image))
            self.compose("config", "--quiet")
            self.compose("up", "-d", "--no-deps", "--no-build", "address-book")
            self.wait_ready(version)
            write_json(self.state_file, {"commit": commit, "version": version, "image": image})
        except Exception:
            print("Falha na atualização; restaurando o backup...", file=sys.stderr, flush=True)
            self.restore(backup, previous)
            raise
        print(f"Concluído. Versão {version}; commit {commit}.\n"
              f"Credenciais preservadas em {self.env} (usuário inicial: admin).\n"
              "Use o mesmo comando para instalar a próxima atualização.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("install", "update"))
    parser.add_argument("version", choices=("latest",))
    parser.add_argument("--directory", type=Path, default=Path("/opt/rustbook"))
    parser.add_argument("--import-from", type=Path,
                        help="Importar uma agenda isolada parada (diretório com data/ e .env).")
    args = parser.parse_args()
    if platform.system() != "Linux" or os.geteuid() != 0:
        parser.error("Execute como root (sudo) em um servidor Linux.")
    for command in ("git", "docker"):
        if not shutil.which(command):
            parser.error(f"Instale {command} antes de continuar.")
    os.umask(0o077)
    run("docker", "compose", "version")
    run("docker", "info", "--format", "{{.ServerVersion}}")
    installer = Installer(args.directory)
    installer.root.mkdir(parents=True, exist_ok=True, mode=0o700)
    # All directories share a project/container name: serialize across directories too.
    lock_directory = Path("/run/lock")
    lock_directory.mkdir(parents=True, exist_ok=True)
    with (lock_directory / "rustbook-install.lock").open("w") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            parser.error("Outra instalação está em execução neste diretório.")
        if args.import_from:
            installer.import_existing(args.import_from)
        installer.install()


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"Instalação interrompida: {error}", file=sys.stderr)
        sys.exit(1)
