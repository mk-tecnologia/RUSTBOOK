"""Restricted hbbs management: real files/key backups, mocked Docker operations."""
import copy
import base64
import importlib.util
import json
from pathlib import Path
import socket
import tempfile
import threading
import unittest
from unittest.mock import patch, Mock

SPEC = importlib.util.spec_from_file_location("server_agent", Path(__file__).parents[1] / "server_agent.py")
agent_module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(agent_module)


class AgentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir="/tmp")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.keys = self.root / "data"
        self.keys.mkdir()
        (self.keys / "id_ed25519").write_bytes(base64.b64encode(bytes(64)))
        (self.keys / "id_ed25519.pub").write_text("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
        self.source = self.root / "compose.yml"
        self.original = ('services:\n  hbbs:\n    image: rustdesk/test:fixed\n'
                         '    command: ["hbbs", "-r", "10.0.1.5:21117", "-k", "_", "--port", "21116"]\n'
                         '    volumes:\n      - ./data:/root\n  hbbr:\n    command: hbbr\n')
        self.source.write_text(self.original)
        self.running = {"Image": "sha256:" + "a" * 64, "State": {"Running": True},
                        "Config": {"Cmd": ["hbbs", "-r", "10.0.1.5:21117", "-k", "_", "--port", "21116"],
                                   "WorkingDir": "/root", "Env": ["KEY=public-fixture", "SAMPLE=literal$value"]},
                        "Mounts": [{"Destination": "/root", "Source": str(self.keys), "Type": "bind"}]}
        self.agent = agent_module.Agent({"root": str(self.root), "project": "test", "service": "hbbs",
                                         "files": [str(self.source)], "working_dir": str(self.root)})
        self.calls = []
        patch.object(agent_module, "INSTALL_LOCK", self.root / "install.lock").start()
        patch.object(agent_module.os, "chown").start()
        patch.object(self.agent, "container", side_effect=lambda: copy.deepcopy(self.running)).start()
        self.compose = patch.object(self.agent, "compose", side_effect=self.compose_command).start()
        self.addCleanup(patch.stopall)
        self.fail_up = False
        self.change_keys = False

    def compose_command(self, *args, override=False):
        self.calls.append((args, override))
        if args == ("config", "--format", "json"):
            return json.dumps({"services": {"hbbs": {"command": getattr(self, "effective_command", self.running["Config"]["Cmd"]), "volumes": [{"type": "bind", "source": str(self.keys), "target": "/root"}]}}})
        if args and args[0] == "up":
            document = json.loads((self.root / "server-agent-override.json").read_text())
            self.running["Config"]["Cmd"] = [item.replace("$$", "$") for item in document["services"]["hbbs"]["command"]]
            if self.fail_up:
                self.fail_up = False
                raise RuntimeError("synthetic startup failure")
            if self.change_keys:
                self.change_keys = False
                (self.keys / "id_ed25519").write_bytes(b"unexpected-generated-key")
        return ""

    def test_applies_only_relay_preserving_auth_image_mounts_and_keypair(self):
        before = {p.name: p.read_bytes() for p in self.keys.iterdir()}
        result = self.agent.apply("relay.example.com:21117")
        self.assertTrue(result["keys_preserved"])
        self.assertEqual(self.running["Config"]["Cmd"], ["hbbs", "-r", "relay.example.com:21117", "-k", "_", "--port", "21116"])
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.keys.iterdir()})
        after = self.source.read_text()
        self.assertIn('"relay.example.com:21117"', after)
        self.assertIn('    volumes:\n      - ./data:/root\n  hbbr:\n    command: hbbr\n', after)
        for args, _ in self.calls:
            self.assertNotIn("down", args)
            if args[0] == "up":
                self.assertEqual(args[-1], "hbbs")
                self.assertIn("--no-deps", args)
                self.assertIn("--no-build", args)
        override = json.loads((self.root / "server-agent-override.json").read_text())["services"]["hbbs"]
        self.assertEqual(override["image"], "sha256:" + "a" * 64)
        self.assertEqual(override["environment"]["SAMPLE"], "literal$$value")
        backup = next((self.root / "server-backups").iterdir())
        self.assertEqual((backup / "compose.original").read_text(), self.original)
        self.assertEqual((backup / "key-0").stat().st_mode & 0o777, 0o600)

    def test_same_relay_does_not_restart_or_rewrite(self):
        result = self.agent.apply("10.0.1.5:21117")
        self.assertFalse(result["changed"])
        self.assertEqual(self.source.read_text(), self.original)
        self.assertFalse(any(args[0] == "up" for args, _ in self.calls))

    def test_runtime_relay_already_correct_still_repairs_persistent_command(self):
        self.effective_command = self.running["Config"]["Cmd"][:]
        self.running["Config"]["Cmd"][2] = "relay.example.com:21117"
        result = self.agent.apply("relay.example.com:21117")
        self.assertTrue(result["changed"])
        self.assertFalse(result["restarted"])
        self.assertIn('"relay.example.com:21117"', self.source.read_text())
        self.assertFalse(any(args[0] == "up" for args, _ in self.calls))

    def test_start_failure_rolls_back_file_command_and_preserves_keys(self):
        self.fail_up = True
        original_command = self.running["Config"]["Cmd"][:]
        with self.assertRaisesRegex(ValueError, "previous command restored"):
            self.agent.apply("relay.example.com:21117")
        self.assertEqual(self.source.read_text(), self.original)
        self.assertEqual(self.running["Config"]["Cmd"], original_command)
        self.assertEqual((self.keys / "id_ed25519").read_bytes(), base64.b64encode(bytes(64)))

    def test_changed_key_is_restored_before_restarting_old_service(self):
        self.change_keys = True
        with self.assertRaisesRegex(ValueError, "previous command restored"):
            self.agent.apply("relay.example.com:21117")
        self.assertEqual((self.keys / "id_ed25519").read_bytes(), base64.b64encode(bytes(64)))
        self.assertTrue(any(args[0] == "stop" for args, _ in self.calls))
        self.assertEqual(agent_module.advertised(self.running["Config"]["Cmd"]), "10.0.1.5:21117")

    def test_mismatched_bind_or_missing_keys_refuses_before_mutation(self):
        self.running["Mounts"][0]["Source"] = str(self.root / "different")
        with self.assertRaises(ValueError):
            self.agent.apply("relay.example.com:21117")
        self.assertEqual(self.source.read_text(), self.original)
        self.assertFalse(any(args[0] == "up" for args, _ in self.calls))

    def test_invalid_relay_never_invokes_docker(self):
        for relay in ["host:0", "host:99999", "user@host:21117", "host:21117;id", "$(id):21117", "https://host:21117", "host:21117\n"]:
            with self.assertRaises(ValueError):
                self.agent.apply(relay)
        self.assertEqual(self.calls, [])

    def test_status_never_discloses_private_key_or_environment(self):
        value = self.agent.status()
        self.assertTrue(value["connected"])
        self.assertTrue(value["can_apply"])
        text = json.dumps(value)
        self.assertNotIn(base64.b64encode(bytes(64)).decode(), text)
        self.assertNotIn("SAMPLE", text)

    def test_patch_supports_block_commands_json_and_literal_dollars(self):
        source = 'services:\n  hbbs:\n    command:\n      - hbbs\n      - -k\n      - public\n    restart: always\n  hbbr:\n    command: hbbr\n'
        changed = agent_module.patch_compose(source, "hbbs", ["hbbs", "-k", "value$literal", "-r", "relay.example.com:21117"])
        self.assertIn('"value$$literal"', changed)
        self.assertIn('    restart: always\n  hbbr:\n    command: hbbr\n', changed)
        document = {"services": {"hbbs": {"command": ["hbbs"]}, "hbbr": {"command": ["hbbr"]}}}
        patched = json.loads(agent_module.patch_compose(json.dumps(document), "hbbs", ["hbbs", "-r", "relay.example.com:21117"]))
        self.assertEqual(patched["services"]["hbbr"], document["services"]["hbbr"])

    def test_unknown_socket_operations_are_rejected(self):
        path = self.root / "test.sock"
        with agent_module.Server(str(path), agent_module.Handler) as server:
            server.agent = Mock()
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            with socket.socket(socket.AF_UNIX) as client:
                client.settimeout(2); client.connect(str(path))
                client.sendall(b'{"action":"exec","command":"arbitrary"}\n')
                result = json.loads(client.recv(4096))
            self.assertFalse(result["ok"])
            server.agent.apply.assert_not_called()
            server.shutdown(); thread.join(2)

    def test_socket_accepts_only_a_single_validated_relay_operation(self):
        path = self.root / "valid.sock"
        with agent_module.Server(str(path), agent_module.Handler) as server:
            server.agent = Mock()
            server.agent.apply.return_value = {"ok": True, "keys_preserved": True}
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            with socket.socket(socket.AF_UNIX) as client:
                client.settimeout(2); client.connect(str(path))
                client.sendall(b'{"action":"apply_relay","relay":"relay.example.com:21117"}\n')
                self.assertTrue(json.loads(client.recv(4096))["ok"])
            server.agent.apply.assert_called_once_with("relay.example.com:21117")
            server.shutdown(); thread.join(2)


if __name__ == "__main__":
    unittest.main()
