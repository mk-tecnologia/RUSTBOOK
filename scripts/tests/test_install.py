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
        backup = next((self.root / "backups").iterdir())
        self.assertEqual((backup / "failed-data/db.sqlite3").read_bytes(), b"migrated database")
        self.assertTrue(self.compose_calls("down"))
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
            return "other-project"

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
        config = {"services": {"address-book": {"ports": [{"host_ip": "0.0.0.0"}]}}}
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
