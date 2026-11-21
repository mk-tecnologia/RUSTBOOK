"""Installer safety regressions; Docker/Git are mocked, data copies are real."""

import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location("installer", Path(__file__).parents[1] / "install.py")
installer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(installer)


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "managed"
        self.root.mkdir()
        self.app = installer.Installer(self.root)
        self.calls = []
        self.runner = patch.object(installer, "run", side_effect=self.run_command).start()
        self.chown = patch.object(installer.os, "chown").start()
        self.ready = patch.object(self.app, "wait_ready").start()
        self.addCleanup(patch.stopall)

    def run_command(self, *args, capture=False):
        self.calls.append(args)
        if args[:2] == ("git", "clone"):
            source = Path(args[-1])
            source.mkdir()
            (source / "Cargo.toml").write_text('[package]\nversion = "0.2.0"\n')
        if args[0] == "git" and "rev-parse" in args:
            return "a" * 40
        if args[:2] == ("docker", "ps"):
            return ""
        return "" if capture else None

    def existing(self):
        self.app.data.mkdir()
        (self.app.data / "db.sqlite3").write_bytes(b"original database")
        (self.app.data / "db.sqlite3-wal").write_bytes(b"original WAL")
        self.app.env.write_text("RUSTDESK_AB_JWT_SECRET=original-secret\n")
        self.previous = {"image": "rustbook:previous", "version": "0.1.0", "commit": "previous"}
        installer.write_json(self.app.config, installer.compose_document(self.previous["image"]))
        installer.write_json(self.app.state_file, self.previous)

    def compose_calls(self, action):
        return [c for c in self.calls if c[:2] == ("docker", "compose") and action in c]

    def legacy(self):
        self.origin = Path(self.temporary.name) / "rustdesk-address-book"
        self.legacy_data = self.origin / "data"
        self.legacy_data.mkdir(parents=True)
        (self.legacy_data / "db.sqlite3").write_bytes(b"legacy database")
        (self.legacy_data / "db.sqlite3-wal").write_bytes(b"legacy WAL")
        self.legacy_env = self.origin / ".env"
        self.legacy_env.write_text("JWT_SECRET=" + "s" * 64 + "\n")
        self.legacy_config = self.origin / "compose.deploy.yaml"
        self.legacy_config.write_text("# original Compose must remain unchanged\n")
        self.legacy_service = {
            "container_name": installer.CONTAINER,
            "volumes": [{"type": "bind", "source": str(self.legacy_data), "target": "/data"}],
            "ports": [{"target": 21114, "published": "32114", "host_ip": "192.0.2.1"}],
        }
        self.container = {
            "Image": "sha256:" + "b" * 64,
            "State": {"Running": True},
            "Config": {
                "Image": "local/rustdesk-address-book:ca39fa3-devices-compat",
                "Env": ["RUSTDESK_AB_JWT_SECRET=" + "s" * 64,
                        "RUSTDESK_AB_DB_PATH=/data/db.sqlite3",
                        "RUSTDESK_AB_ADMIN_PASSWORD=original$password'with-quote"],
                "Labels": {
                    "com.docker.compose.project": "rustdesk-address-book",
                    "com.docker.compose.service": "address-book",
                    "com.docker.compose.project.working_dir": str(self.origin),
                    "com.docker.compose.project.config_files": str(self.legacy_config),
                },
            },
            "Mounts": [{"Type": "bind", "Source": str(self.legacy_data),
                        "Destination": "/data", "RW": True}],
        }

        def run_legacy(*args, capture=False):
            if args[:2] == ("docker", "ps"):
                self.calls.append(args)
                return installer.CONTAINER
            if args[:3] == ("docker", "inspect", installer.CONTAINER):
                self.calls.append(args)
                return json.dumps([self.container])
            if args[:3] == ("docker", "inspect", "--format"):
                self.calls.append(args)
                return "true"
            if args[:2] == ("docker", "compose") and "config" in args and "--format" in args:
                self.calls.append(args)
                return json.dumps({"services": {"address-book": self.legacy_service}})
            if args[:2] == ("docker", "compose") and "up" in args:
                self.container["Config"]["Labels"]["com.docker.compose.project.config_files"] = \
                    str(self.legacy_config) + "," + str(self.app.config)
            return self.run_command(*args, capture=capture)

        self.runner.side_effect = run_legacy
        return run_legacy

    def test_legacy_install_preserves_live_volume_compose_credentials_and_project(self):
        self.legacy()
        original = self.legacy_config.read_bytes()
        self.app.install()
        self.assertEqual(self.app.data, self.legacy_data)
        self.assertEqual((self.legacy_data / "db.sqlite3").read_bytes(), b"legacy database")
        self.assertEqual(self.legacy_config.read_bytes(), original)
        self.assertFalse((self.root / "data").exists())
        override = json.loads(self.app.config.read_text())["services"]["address-book"]
        self.assertEqual(override["environment"]["RUSTDESK_AB_JWT_SECRET"], "s" * 64)
        self.assertEqual(override["environment"]["RUSTDESK_AB_ADMIN_PASSWORD"], "original$$password'with-quote")
        self.assertNotIn("ports", override)
        self.assertNotIn("volumes", override)
        state = json.loads(self.app.state_file.read_text())
        self.assertEqual(state["deployment"]["project"], "rustdesk-address-book")
        self.assertEqual(state["deployment"]["data"], str(self.legacy_data))
        for call in self.compose_calls("up") + self.compose_calls("stop"):
            self.assertEqual(call[-1], "address-book")
            self.assertNotIn("down", call)
            self.assertIn(str(self.legacy_config), call)
        backup = next((self.root / "backups").iterdir())
        self.assertEqual((backup / "data/db.sqlite3-wal").read_bytes(), b"legacy WAL")
        self.assertEqual((backup / "original-config/0").read_bytes(), original)

    def test_legacy_install_can_be_updated_again_without_manual_import(self):
        self.legacy()
        self.app.install()
        second = installer.Installer(self.root)
        with patch.object(second, "wait_ready"):
            second.install()
        self.assertEqual(second.data, self.legacy_data)
        self.assertEqual(len(self.compose_calls("up")), 2)

    def test_legacy_build_failure_leaves_old_service_and_config_untouched(self):
        legacy_run = self.legacy()

        def failed_build(*args, capture=False):
            if args[:2] == ("docker", "build"):
                raise subprocess.CalledProcessError(1, args)
            return legacy_run(*args, capture=capture)

        self.runner.side_effect = failed_build
        with self.assertRaises(subprocess.CalledProcessError):
            self.app.install()
        self.assertFalse(self.compose_calls("stop"))
        self.assertFalse(self.app.state_file.exists())
        self.assertFalse(self.app.env.exists())

    def test_legacy_health_failure_restores_old_image_data_and_file_owners(self):
        self.legacy()
        old_image = self.container["Image"]
        owner = (self.legacy_data / "db.sqlite3").stat().st_uid

        def unhealthy(version):
            (self.legacy_data / "db.sqlite3").write_bytes(b"migrated database")
            raise RuntimeError("legacy update failed")

        self.ready.side_effect = unhealthy
        with patch.object(installer.time, "sleep"), self.assertRaisesRegex(RuntimeError, "legacy update failed"):
            self.app.install()
        self.assertEqual((self.legacy_data / "db.sqlite3").read_bytes(), b"legacy database")
        state = json.loads(self.app.state_file.read_text())
        self.assertEqual(state["image"], old_image)
        self.assertIsNone(state["version"])
        self.chown.assert_any_call(self.legacy_data / "db.sqlite3", owner,
                                   (self.legacy_data / "db.sqlite3").stat().st_gid)
        self.assertEqual(len(self.compose_calls("up")), 2)
        self.assertFalse(self.compose_calls("down"))
        self.assertIn("--force-recreate", self.compose_calls("up")[-1])

    def test_legacy_backup_failure_can_be_retried(self):
        self.legacy()
        with patch.object(self.app, "backup", side_effect=OSError("disk full")):
            with self.assertRaises(OSError):
                self.app.install()
        second = installer.Installer(self.root)
        with patch.object(second, "wait_ready"):
            second.install()
        self.assertEqual(second.data, self.legacy_data)

    def test_legacy_environment_applied_after_adoption_survives_next_update(self):
        self.legacy()
        self.app.install()
        self.container["Config"]["Env"].append("RUSTDESK_AB_CORS_ORIGINS=https://updated.example.com")
        second = installer.Installer(self.root)
        with patch.object(second, "wait_ready"):
            second.install()
        override = json.loads(second.config.read_text())["services"]["address-book"]
        self.assertEqual(override["environment"]["RUSTDESK_AB_CORS_ORIGINS"], "https://updated.example.com")

    def test_legacy_mismatched_compose_volume_is_rejected_before_build(self):
        self.legacy()
        self.legacy_service["volumes"][0]["source"] = str(self.root / "wrong-data")
        with self.assertRaisesRegex(RuntimeError, "não corresponde"):
            self.app.install()
        self.assertFalse(self.compose_calls("stop"))
        self.assertFalse(any(call[:2] == ("docker", "build") for call in self.calls))

    def test_legacy_missing_persistent_jwt_is_not_replaced(self):
        self.legacy()
        self.container["Config"]["Env"] = ["RUSTDESK_AB_DB_PATH=/data/db.sqlite3"]
        with self.assertRaisesRegex(RuntimeError, "chave JWT persistente"):
            self.app.install()
        self.assertFalse(self.compose_calls("stop"))
        self.assertFalse(self.app.env.exists())

    def test_offline_import_does_not_write_into_a_detected_live_deployment(self):
        self.legacy()
        with self.assertRaisesRegex(RuntimeError, "sem --import-from"):
            self.app.import_existing(self.origin)
        self.assertEqual((self.legacy_data / "db.sqlite3").read_bytes(), b"legacy database")
        self.assertFalse(self.compose_calls("stop"))

    def test_fresh_install_generates_private_credentials_and_pins_image(self):
        self.app.install()
        content = self.app.env.read_text()
        secret = dict(line.split("=", 1) for line in content.splitlines())
        self.assertGreaterEqual(len(secret["RUSTDESK_AB_JWT_SECRET"]), 32)
        self.assertGreaterEqual(len(secret["RUSTDESK_AB_ADMIN_PASSWORD"]), 12)
        self.assertEqual(self.app.env.stat().st_mode & 0o777, 0o600)
        state = json.loads(self.app.state_file.read_text())
        self.assertEqual(state["image"], "rustbook:" + "a" * 40)
        self.assertEqual(json.loads(self.app.config.read_text())["services"]["address-book"]["image"], state["image"])
        self.assertFalse(self.compose_calls("stop"))
        self.ready.assert_called_once_with("0.2.0")

    def test_update_preserves_credentials_and_backs_up_stopped_database(self):
        self.existing()
        credentials = self.app.env.read_bytes()
        original_backup = self.app.backup

        def stopped_backup():
            self.assertTrue(self.compose_calls("stop"))
            return original_backup()

        with patch.object(self.app, "backup", side_effect=stopped_backup):
            self.app.install()
        self.assertEqual(self.app.env.read_bytes(), credentials)
        backup = next((self.root / "backups").iterdir())
        self.assertEqual((backup / "data/db.sqlite3").read_bytes(), b"original database")
        self.assertEqual((backup / "data/db.sqlite3-wal").read_bytes(), b"original WAL")
        build = next(i for i, c in enumerate(self.calls) if c[:2] == ("docker", "build"))
        stop = next(i for i, c in enumerate(self.calls) if c in self.compose_calls("stop"))
        self.assertLess(build, stop)

    def test_failed_build_never_stops_existing_service(self):
        self.existing()

        def failed_build(*args, capture=False):
            if args[:2] == ("docker", "build"):
                raise subprocess.CalledProcessError(1, args)
            return self.run_command(*args, capture=capture)

        self.runner.side_effect = failed_build
        with self.assertRaises(subprocess.CalledProcessError):
            self.app.install()
        self.assertFalse(self.compose_calls("stop"))
        self.assertEqual(json.loads(self.app.state_file.read_text()), self.previous)

    def test_failed_backup_restarts_previous_service(self):
        self.existing()
        with patch.object(self.app, "backup", side_effect=OSError("disk full")):
            with self.assertRaises(OSError):
                self.app.install()
        self.assertTrue(self.compose_calls("start"))
        self.assertFalse(self.compose_calls("up"))

    def test_failed_health_restores_database_wal_image_and_credentials(self):
        self.existing()

        def unhealthy(version):
            if version == "0.2.0":
                (self.app.data / "db.sqlite3").write_bytes(b"migrated database")
                (self.app.data / "db.sqlite3-wal").write_bytes(b"migrated WAL")
                raise RuntimeError("unhealthy")

        self.ready.side_effect = unhealthy
        with self.assertRaisesRegex(RuntimeError, "unhealthy"):
            self.app.install()
        self.assertEqual((self.app.data / "db.sqlite3").read_bytes(), b"original database")
        self.assertEqual((self.app.data / "db.sqlite3-wal").read_bytes(), b"original WAL")
        self.assertEqual(json.loads(self.app.state_file.read_text()), self.previous)
        image = json.loads(self.app.config.read_text())["services"]["address-book"]["image"]
        self.assertEqual(image, self.previous["image"])
        failed = next(self.root.glob("data.failed-*"))
        self.assertEqual((failed / "db.sqlite3").read_bytes(), b"migrated database")
        self.assertTrue(self.compose_calls("stop"))
        self.assertFalse(self.compose_calls("down"))
        self.assertEqual(len(self.compose_calls("up")), 2)
        self.assertTrue(self.chown.called)

    def test_missing_managed_environment_is_not_regenerated(self):
        self.existing()
        self.app.env.unlink()
        with self.assertRaisesRegex(RuntimeError, "ausentes"):
            self.app.install()
        self.assertFalse(self.app.env.exists())
        self.assertFalse(self.compose_calls("stop"))

    def test_unmanaged_data_is_never_overwritten(self):
        self.app.data.mkdir()
        (self.app.data / "db.sqlite3").write_bytes(b"unmanaged")
        with self.assertRaisesRegex(RuntimeError, "não gerenciados"):
            self.app.install()
        self.assertEqual((self.app.data / "db.sqlite3").read_bytes(), b"unmanaged")
        self.assertFalse(self.compose_calls("stop"))

    def test_unmanaged_container_is_never_replaced(self):
        def existing_container(*args, capture=False):
            if args[:2] == ("docker", "ps"):
                return installer.CONTAINER
            return json.dumps([{"Config": {"Labels": {}}}])

        self.runner.side_effect = existing_container
        with self.assertRaisesRegex(RuntimeError, "fora deste instalador"):
            self.app.install()
        self.assertFalse(self.compose_calls("stop"))

    def test_import_preserves_original_data_and_jwt(self):
        origin = Path(self.temporary.name) / "old"
        (origin / "data").mkdir(parents=True)
        (origin / "data/db.sqlite3").write_bytes(b"old db")
        environment = "RUSTDESK_AB_JWT_SECRET=" + "x" * 64 + "\n"
        (origin / ".env").write_text(environment)
        self.app.import_existing(origin)
        self.assertEqual((self.app.data / "db.sqlite3").read_bytes(), b"old db")
        self.assertEqual((origin / "data/db.sqlite3").read_bytes(), b"old db")
        self.assertEqual(self.app.env.read_text(), environment)
        self.assertEqual(json.loads(self.app.state_file.read_text()), {})

    def test_import_of_live_database_is_rejected(self):
        origin = Path(self.temporary.name) / "old"
        (origin / "data").mkdir(parents=True)
        (origin / "data/db.sqlite3").write_bytes(b"live db")
        (origin / ".env").write_text("RUSTDESK_AB_JWT_SECRET=existing\n")

        def live(*args, capture=False):
            if args[:3] == ("docker", "ps", "-q"):
                return "container-id"
            if args[:3] == ("docker", "inspect", "container-id"):
                return json.dumps([{"Mounts": [{"Source": str(origin / "data")}]}])
            return self.run_command(*args, capture=capture)

        self.runner.side_effect = live
        with self.assertRaisesRegex(RuntimeError, "Pare os containers"):
            self.app.import_existing(origin)
        self.assertFalse(self.app.data.exists())

    def test_health_retries_until_expected_version_and_uses_local_wildcard_address(self):
        # Exercise the real method rather than the patch used for deployment tests.
        config = {"services": {"address-book": {"ports": [
            {"host_ip": "0.0.0.0", "target": 21114, "published": "21114"}]}}}
        self.runner.side_effect = None
        self.runner.return_value = json.dumps(config)
        with patch.object(installer.urllib.request, "build_opener") as opener, \
                patch.object(installer.time, "sleep") as sleep:
            opener.return_value.open.side_effect = [
                io.StringIO('{"version":"0.1.0"}'),
                io.StringIO('{"version":"0.2.0"}'),
            ]
            installer.Installer.wait_ready(self.app, "0.2.0")
            sleep.assert_called_once_with(2)
            self.assertEqual(opener.return_value.open.call_count, 2)
            opener.return_value.open.assert_called_with("http://127.0.0.1:21114/api/version", timeout=3)


if __name__ == "__main__":
    unittest.main()
