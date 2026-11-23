#!/usr/bin/env python3
"""Install/update the standalone address book on a Linux Docker host."""

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.request


REPOSITORY = "https://github.com/mk-tecnologia/RUSTBOOK.git"
RELEASES = "https://api.github.com/repos/mk-tecnologia/RUSTBOOK/releases?per_page=30"
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


def fetch_json(url):
    request = urllib.request.Request(url, headers={"User-Agent": "RUSTBOOK-installer",
                                                  "Accept": "application/json"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def published_build(architecture):
    # GitHub may group stable/prereleases instead of returning publication order.
    releases = sorted(fetch_json(RELEASES), key=lambda release: release.get("published_at") or "", reverse=True)
    for release in releases:
        tag = release.get("tag_name", "")
        assets = {asset["name"]: asset["browser_download_url"] for asset in release.get("assets", [])}
        if (release.get("draft") or not re.fullmatch(r"build-[0-9a-f]{40}", tag)
                or "manifest.json" not in assets):
            continue
        manifest = fetch_json(assets["manifest.json"])
        if manifest.get("commit") != tag.removeprefix("build-"):
            raise RuntimeError("O manifesto publicado não corresponde ao commit da imagem.")
        image = manifest.get("images", {}).get(architecture)
        if not image or image.get("asset") not in assets:
            continue
        if (image.get("image") != f"rustbook:{manifest['commit']}"
                or not re.fullmatch(r"[0-9a-f]{64}", image.get("sha256", ""))
                or not isinstance(manifest.get("version"), str)):
            raise RuntimeError("Manifesto de imagem inválido; nenhum serviço foi alterado.")
        return manifest, image, assets[image["asset"]]
    raise RuntimeError("Nenhuma imagem pronta foi publicada ainda. Aguarde a CI no GitHub e repita "
                       "o comando, ou use --build para compilar explicitamente neste servidor.")


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


AGENT_SOCKET = ".rustbook-server-agent.sock"


def ignore_runtime_socket(directory, names):
    return [name for name in names if name == AGENT_SOCKET
            and not (Path(directory) / name).is_symlink()
            and (Path(directory) / name).is_socket()]


class Installer:
    def __init__(self, directory, build_locally=False, hbbs_server=None):
        self.root = directory.resolve()
        self.config = self.root / "compose.json"
        self.state_file = self.root / "install-state.json"
        self.env = self.root / ".env"
        self.data = self.root / "data"
        self.deployment = None
        self.adoption = None
        self.build_locally = build_locally
        self.hbbs_server = hbbs_server

    def already_current(self, previous, commit, version):
        if previous.get("commit") != commit or previous.get("version") != version:
            return False
        try:
            current = json.loads(run("docker", "inspect", CONTAINER, capture=True))[0]
            if self.hbbs_server is not None:
                environment = dict(item.split("=", 1) for item in current.get("Config", {}).get("Env", []) if "=" in item)
                if environment.get("RUSTDESK_AB_RENDEZVOUS_SERVER", "") != self.hbbs_server:
                    return False
            image_id = run("docker", "image", "inspect", previous["image"],
                           "--format", "{{.Id}}", capture=True)
            if not current["State"]["Running"] or current["Image"] != image_id:
                return False
            self.wait_ready(version, commit)
        except (subprocess.CalledProcessError, RuntimeError, ValueError, KeyError):
            return False
        print(f"Já atualizado: {version}, commit {commit}. Nenhum build ou reinício necessário.", flush=True)
        return True

    def prepare_image(self, previous):
        if not self.build_locally:
            architecture = run("docker", "info", "--format", "{{.Architecture}}", capture=True)
            architecture = {"x86_64": "amd64", "aarch64": "arm64"}.get(architecture, architecture)
            if architecture not in ("amd64", "arm64"):
                raise RuntimeError("Não há imagem pronta para esta arquitetura; use --build.")
            manifest, artifact, url = published_build(architecture)
            commit, version = manifest["commit"], manifest["version"]
            if self.already_current(previous, commit, version):
                return None
            print(f"Baixando imagem pronta {version} ({architecture}), commit {commit}...", flush=True)
            with tempfile.TemporaryDirectory(prefix="image-", dir=self.root) as temporary:
                archive = Path(temporary) / "image.tar.gz"
                digest = hashlib.sha256()
                with urllib.request.urlopen(url, timeout=60) as response, archive.open("wb") as target:
                    while chunk := response.read(1024 * 1024):
                        digest.update(chunk)
                        target.write(chunk)
                if digest.hexdigest() != artifact["sha256"]:
                    raise RuntimeError("A imagem baixada falhou na verificação SHA-256; atualização cancelada.")
                run("docker", "load", "--input", str(archive))
            # Verify the archive actually loaded the manifest's expected tag.
            run("docker", "image", "inspect", artifact["image"], capture=True)
            return commit, version, artifact["image"]
        with tempfile.TemporaryDirectory(prefix="source-", dir=self.root) as temporary:
            source = Path(temporary) / "repo"
            run("git", "clone", "--depth", "1", "--branch", "main", "--", REPOSITORY, str(source))
            commit = run("git", "-C", str(source), "rev-parse", "HEAD", capture=True)
            with (source / "Cargo.toml").open("rb") as manifest:
                version = tomllib.load(manifest)["package"]["version"]
            if self.already_current(previous, commit, version):
                return None
            image = f"rustbook:{commit}"
            print(f"Compilação local solicitada: {version}, commit {commit}.", flush=True)
            run("docker", "build", "--build-arg", f"RUSTBOOK_BUILD_COMMIT={commit}", "--tag", image, str(source))
            return commit, version, image

    def compose_args(self):
        if not self.deployment:
            return ["docker", "compose", "--project-name", "rustbook",
                    "--env-file", str(self.env), "--file", str(self.config)]
        context = self.deployment
        args = ["docker", "compose", "--project-name", context["project"],
                "--project-directory", context["directory"]]
        if context.get("env_file"):
            args += ["--env-file", context["env_file"]]
        for config in context["files"] + [str(self.config)]:
            args += ["--file", config]
        return args

    @property
    def service(self):
        return self.deployment["service"] if self.deployment else "address-book"

    def document(self, image, apply_changes=True):
        if self.deployment:
            # Compose interpolates dollar signs even inside JSON values.
            environment = {key: value.replace("$", "$$")
                           for key, value in self.deployment["environment"].items()}
            document = {"services": {self.service: {
                "image": image, "pull_policy": "never", "environment": environment}}}
        else:
            document = compose_document(image)
        if apply_changes and self.hbbs_server is not None:
            document["services"][self.service]["environment"]["RUSTDESK_AB_RENDEZVOUS_SERVER"] = self.hbbs_server.replace("$", "$$")
        return document

    def compose(self, *args):
        return run(*self.compose_args(), *args)

    def write_launcher(self):
        path = self.root / "manage.sh"
        path.write_text('#!/bin/sh\nexec ' + shlex.join(self.compose_args()) + ' "$@"\n')
        os.chmod(path, 0o700)

    def check_ownership(self):
        if self.state_file.exists():
            state = json.loads(self.state_file.read_text())
            if self.hbbs_server is None:
                self.hbbs_server = state.get("hbbs_server")
            self.deployment = state.get("deployment")
            if self.deployment:
                self.data = Path(self.deployment["data"])
        names = run("docker", "ps", "-a", "--format", "{{.Names}}", capture=True).splitlines()
        if CONTAINER in names:
            container = json.loads(run("docker", "inspect", CONTAINER, capture=True))[0]
            labels = container["Config"].get("Labels") or {}
            project = labels.get("com.docker.compose.project")
            files = labels.get("com.docker.compose.project.config_files", "").split(",")
            expected_project = self.deployment["project"] if self.deployment else "rustbook"
            expected_files = (self.deployment["files"] if self.deployment else []) + [str(self.config)]
            if not self.state_file.exists():
                self.prepare_adoption(container)
            elif (project != expected_project or (files != expected_files and not (
                    self.deployment and state.get("version") is None
                    and files == self.deployment["files"] and container["Image"] == state.get("image")))):
                raise RuntimeError("O container não corresponde ao estado deste instalador; "
                                   "nenhum container foi alterado.")
            elif self.deployment:
                self.deployment["environment"] = {
                    key: value for item in container["Config"].get("Env", []) if "=" in item
                    for key, value in [item.split("=", 1)] if key.startswith("RUSTDESK_AB_")}
        if not self.state_file.exists() and not self.adoption and (
            self.config.exists() or self.env.exists()
            or (self.data.exists() and any(self.data.iterdir()))
        ):
            raise RuntimeError("Diretório com configuração/dados não gerenciados. "
                               "Consulte docs/INSTALACAO.md antes de migrar.")

    def prepare_adoption(self, container):
        """Recognize an existing Compose agenda and retain its deployment context."""
        labels = container["Config"].get("Labels") or {}
        project = labels.get("com.docker.compose.project")
        directory = labels.get("com.docker.compose.project.working_dir")
        service = labels.get("com.docker.compose.service")
        files = labels.get("com.docker.compose.project.config_files", "").split(",")
        if (not project or not directory or service != "address-book"
                or any(not Path(path).is_absolute() or not Path(path).is_file() for path in files)
                or not Path(directory).is_dir() or str(self.config) in files):
            raise RuntimeError("Agenda fora deste instalador sem um Compose reconhecível; "
                               "nenhum container foi alterado.")
        mounts = [mount for mount in container.get("Mounts", []) if mount["Destination"] == "/data"]
        if len(mounts) != 1 or mounts[0]["Type"] != "bind" or not mounts[0].get("RW"):
            raise RuntimeError("Migração automática exige um volume bind gravável em /data.")
        data = Path(mounts[0]["Source"])
        environment = dict(item.split("=", 1) for item in container["Config"].get("Env", []) if "=" in item)
        environment = {key: value for key, value in environment.items() if key.startswith("RUSTDESK_AB_")}
        database = environment.get("RUSTDESK_AB_DB_PATH", "/data/db.sqlite3")
        if database != "/data/db.sqlite3" or not (data / "db.sqlite3").is_file():
            raise RuntimeError("Banco existente não encontrado em /data/db.sqlite3; atualização cancelada.")
        if len(environment.get("RUSTDESK_AB_JWT_SECRET", "").encode()) < 32:
            raise RuntimeError("A agenda existente precisa de uma chave JWT persistente de ao menos 32 bytes "
                               "no ambiente; nenhum segredo foi substituído.")
        if data.is_symlink() or any(path.is_symlink() for path in data.rglob("*")):
            raise RuntimeError("O diretório de dados não pode conter links simbólicos.")
        env_file = Path(directory) / ".env"
        context = {"project": project, "directory": directory, "service": service,
                   "files": files, "data": str(data), "environment": environment,
                   "env_file": str(env_file) if env_file.is_file() else None}
        # Validate the original Compose before building or interrupting the service.
        self.deployment = context
        args = self.compose_args()[:-2]  # The override does not exist yet.
        resolved = json.loads(run(*args, "config", "--format", "json", capture=True))
        original = resolved["services"][service]
        volumes = [mount for mount in original.get("volumes", []) if mount.get("target") == "/data"]
        if (original.get("container_name") != CONTAINER or len(volumes) != 1
                or volumes[0].get("type") != "bind"
                or Path(volumes[0].get("source", "")).resolve() != data.resolve()
                or volumes[0].get("read_only", False)):
            raise RuntimeError("O Compose atual não corresponde ao volume/container em execução; atualização cancelada.")
        target = int(environment.get("RUSTDESK_AB_PORT", 21114))
        if not any(int(port["target"]) == target and port.get("published")
                   and port.get("protocol", "tcp") == "tcp" for port in original.get("ports", [])):
            raise RuntimeError("A migração exige uma porta HTTP publicada no host para verificar a atualização.")
        self.data = data
        self.adoption = {"image": container["Image"], "version": None,
                         "deployment": context, "was_running": bool(container["State"]["Running"])}
        print(f"Agenda existente reconhecida: {directory}; dados preservados em {data}.", flush=True)

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

    def enable_server_management(self, compose_file=None, service="hbbs"):
        config = self.root / "server-agent.json"
        if compose_file is None:
            if not config.exists():
                return
            current = json.loads(config.read_text())
            compose_file = Path(current["files"][0])
            service = current["service"]
        script = self.root / "server-agent.py"
        temporary = self.root / ".server-agent.new.py"
        run("docker", "cp", CONTAINER + ":/app/server-agent.py", str(temporary))
        compile(temporary.read_text(), str(script), "exec")
        if config.exists() and script.exists() and temporary.read_bytes() == script.read_bytes():
            current = json.loads(config.read_text())
            if (str(compose_file.resolve()) in current["files"] and current["service"] == service
                    and current.get("data") == str(self.data) and (self.data / AGENT_SOCKET).is_socket()):
                try:
                    run("systemctl", "is-active", "--quiet", "rustbook-server-agent.service")
                    temporary.unlink()
                    print("Agente do servidor já ativo; nenhuma reinicialização necessária.", flush=True)
                    return
                except subprocess.CalledProcessError:
                    pass
        os.chmod(temporary, 0o600)
        temporary.replace(script)
        run(sys.executable, str(script), "install", "--root", str(self.root), "--data", str(self.data),
            "--compose", str(compose_file.resolve()), "--service", service)

    def credentials(self):
        if self.env.exists():
            return
        if self.deployment:
            if self.deployment.get("env_file"):
                shutil.copy2(self.deployment["env_file"], self.env)
                os.chmod(self.env, 0o600)
            else:
                self.env.write_text("# Configuração existente preservada em compose.json.\n")
                os.chmod(self.env, 0o600)
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
        if self.adoption:
            raise RuntimeError("Agenda Compose reconhecida; execute install latest sem --import-from.")
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
        shutil.copytree(source / "data", self.data, ignore=ignore_runtime_socket)
        self.env.write_text(environment)
        os.chmod(self.env, 0o600)
        write_json(self.state_file, {})
        print("Dados e credenciais importados; a origem foi preservada.", flush=True)

    def backup(self):
        name = time.strftime("%Y%m%d-%H%M%S") + "-" + secrets.token_hex(3)
        destination = self.root / "backups" / name
        destination.mkdir(parents=True, mode=0o700)
        shutil.copytree(self.data, destination / "data", ignore=ignore_runtime_socket)
        for path in (self.env, self.config, self.state_file):
            if path.exists():
                shutil.copy2(path, destination / path.name)
        ownership = {}
        for path in [self.data, *self.data.rglob("*")]:
            if path.name == AGENT_SOCKET and path.is_socket() and not path.is_symlink():
                continue
            if path.is_symlink():
                raise RuntimeError("O diretório de dados não pode conter links simbólicos.")
            status = path.stat()
            ownership[str(path.relative_to(self.data))] = [status.st_uid, status.st_gid]
        write_json(destination / "ownership.json", ownership)
        if self.deployment:
            sources = self.deployment["files"] + ([self.deployment["env_file"]]
                                                  if self.deployment.get("env_file") else [])
            snapshots = destination / "original-config"
            snapshots.mkdir(mode=0o700)
            for index, source in enumerate(sources):
                shutil.copy2(source, snapshots / str(index))
            write_json(snapshots / "paths.json", sources)
        print(f"Backup: {destination}", flush=True)
        return destination

    def wait_ready(self, version, commit=None):
        config = run(*self.compose_args(), "config", "--format", "json", capture=True)
        service = json.loads(config)["services"][self.service]
        target = int(service.get("environment", {}).get("RUSTDESK_AB_PORT", 21114))
        ports = [port for port in service.get("ports", [])
                 if int(port["target"]) == target and port.get("protocol", "tcp") == "tcp"]
        if not ports or not ports[0].get("published"):
            raise RuntimeError("A verificação exige uma porta HTTP publicada no host.")
        host = ports[0].get("host_ip") or "127.0.0.1"
        if host == "0.0.0.0":
            host = "127.0.0.1"
        elif host == "::":
            host = "::1"
        if ":" in host:
            host = f"[{host}]"
        url = f"http://{host}:{ports[0]['published']}/api/version"
        client = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            try:
                with client.open(url, timeout=3) as response:
                    metadata = json.load(response)
                    if metadata.get("version") == version and (commit is None or metadata.get("commit") == commit):
                        print(f"Agenda disponível: {url}", flush=True)
                        return
            except (OSError, ValueError):
                pass
            time.sleep(2)
        raise RuntimeError("A agenda não respondeu com a versão esperada em 60 segundos.")

    def restore(self, backup, previous):
        self.compose("stop", self.service)
        # Keep the failed database for diagnosis; never delete either database.
        failed = self.data.with_name(self.data.name + ".failed-" + backup.name)
        self.data.rename(failed)
        shutil.copytree(backup / "data", self.data)
        runtime = failed / AGENT_SOCKET
        if runtime.is_socket() and not runtime.is_symlink():
            runtime.rename(self.data / AGENT_SOCKET)
        ownership = json.loads((backup / "ownership.json").read_text())
        for relative, owner in ownership.items():
            os.chown(self.data / relative, *owner)
        print(f"Dados da tentativa preservados em {failed}.", flush=True)
        for name in (".env", "compose.json", "install-state.json"):
            saved = backup / name
            if saved.exists():
                shutil.copy2(saved, self.root / name)
        if previous.get("image") and previous.get("was_running", True):
            self.compose("up", "-d", "--no-deps", "--no-build", "--force-recreate", self.service)
            if previous.get("version"):
                self.wait_ready(previous["version"])
            else:
                # Upstream deployments may not implement /api/version.
                time.sleep(2)
                if run("docker", "inspect", "--format", "{{.State.Running}}", CONTAINER,
                       capture=True) != "true":
                    raise RuntimeError("O container anterior não permaneceu em execução; consulte os logs.")
            print("Versão e banco anteriores restaurados.", flush=True)

    def install(self):
        self.check_ownership()
        previous = json.loads(self.state_file.read_text()) if self.state_file.exists() else self.adoption or {}
        if (previous.get("image") and not self.adoption
                and (not self.env.is_file() or not self.data.is_dir())):
            raise RuntimeError("Configuração ou dados ausentes; restaure o backup antes de atualizar.")
        # Download/verify (or explicitly build) before stopping the old service.
        prepared = self.prepare_image(previous)
        if prepared is None:
            return
        commit, version, image = prepared
        self.credentials()
        self.data.mkdir(mode=0o700, exist_ok=True)
        if not self.state_file.exists():
            if self.adoption:
                write_json(self.config, self.document(previous["image"], apply_changes=False))
            write_json(self.state_file, previous)
        self.write_launcher()
        try:
            if previous.get("image"):
                self.compose("stop", self.service)
            backup = self.backup()
        except Exception:
            if previous.get("image") and previous.get("was_running", True):
                self.compose("start", self.service)
            raise
        try:
            self.set_data_owner()
            write_json(self.config, self.document(image))
            self.compose("config", "--quiet")
            self.compose("up", "-d", "--no-deps", "--no-build", self.service)
            self.wait_ready(version, commit)
            state = {"commit": commit, "version": version, "image": image}
            if self.hbbs_server is not None:
                state["hbbs_server"] = self.hbbs_server
            if self.deployment:
                state["deployment"] = self.deployment
            write_json(self.state_file, state)
        except Exception:
            print("Falha na atualização; restaurando o backup...", file=sys.stderr, flush=True)
            self.restore(backup, previous)
            raise
        print(f"Concluído. Versão {version}; commit {commit}.\n"
              f"Dados preservados em {self.data}; configuração privada em {self.root}.\n"
              "Use o mesmo comando para instalar a próxima atualização.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("install", "update"))
    parser.add_argument("version", choices=("latest",))
    parser.add_argument("--directory", type=Path, default=Path("/opt/rustbook"))
    parser.add_argument("--import-from", type=Path,
                        help="Importar uma agenda isolada parada (diretório com data/ e .env).")
    parser.add_argument("--build", action="store_true", help="Compilar main localmente em vez de baixar imagem pronta.")
    parser.add_argument("--hbbs", metavar="HOST:PORT", help="Consultar presença no hbbs (porta NAT TCP, normalmente 21115); use '' para desativar.")
    parser.add_argument("--manage-hbbs", type=Path, metavar="COMPOSE_FILE", help="Ativar aplicação de relay pelo painel para este Compose hbbs existente (sem reiniciar hbbs agora).")
    parser.add_argument("--hbbs-service", default="hbbs", help="Serviço hbbs no Compose gerenciado.")
    args = parser.parse_args()
    if args.hbbs:
        from urllib.parse import urlsplit
        try:
            endpoint = urlsplit("//" + args.hbbs)
            if (not endpoint.hostname or not endpoint.port or endpoint.username or endpoint.password
                    or endpoint.path or endpoint.query or endpoint.fragment or len(args.hbbs) > 255
                    or any(character.isspace() for character in args.hbbs)):
                raise ValueError()
        except ValueError:
            parser.error("--hbbs deve ser hostname:porta, por exemplo rustdesk.example.com:21115, sem credenciais.")
    if platform.system() != "Linux" or os.geteuid() != 0:
        parser.error("Execute como root em um servidor Linux (use su - em uma conta comum).")
    for command in (("git", "docker") if args.build else ("docker",)):
        if not shutil.which(command):
            parser.error(f"Instale {command} antes de continuar.")
    os.umask(0o077)
    run("docker", "compose", "version")
    run("docker", "info", "--format", "{{.ServerVersion}}")
    installer = Installer(args.directory, build_locally=args.build, hbbs_server=args.hbbs)
    installer.root.mkdir(parents=True, exist_ok=True, mode=0o700)
    # All directories share a project/container name: serialize across directories too.
    lock_directory = Path("/run/lock")
    lock_directory.mkdir(parents=True, exist_ok=True)
    with (lock_directory / "rustbook-install.lock").open("w") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            parser.error("Outra instalação do RUSTBOOK está em execução.")
        if args.import_from:
            installer.import_existing(args.import_from)
        installer.install()
        installer.enable_server_management(args.manage_hbbs, args.hbbs_service)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"Instalação interrompida: {error}", file=sys.stderr)
        sys.exit(1)
