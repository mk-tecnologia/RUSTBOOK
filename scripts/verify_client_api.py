"""Verify client query contracts without direct SQLite access or device mutations.

Use --base-url with RUSTDESK_VERIFY_TOKEN, or --username and a prompted password.
A new login is logged by the server and its session is revoked afterwards. The
personal-book query may create the current user's book if none exists yet.
"""
import argparse
import getpass
import json
import os
import sys
from urllib.error import HTTPError
from urllib.parse import urlencode, urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener


class NoRedirects(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", required=True, help="HTTP(S) origin of the address book")
    parser.add_argument("--username", help="Log in with a prompted password instead of a preexisting token")
    args = parser.parse_args()
    origin = urlsplit(args.base_url)
    if origin.scheme not in ("http", "https") or not origin.hostname or origin.username or origin.password or origin.path not in ("", "/") or origin.query or origin.fragment:
        parser.error("base-url must be an HTTP(S) origin without credentials, path or query")
    base = args.base_url.rstrip("/")
    token = os.environ.get("RUSTDESK_VERIFY_TOKEN", "")
    if args.username and token:
        parser.error("Choose --username or RUSTDESK_VERIFY_TOKEN, not both")
    if not args.username and not token:
        parser.error("Set RUSTDESK_VERIFY_TOKEN or provide --username")
    opener = build_opener(NoRedirects())

    def request(method, path, body=None, authenticated=True, expected=200):
        headers = {}
        if token and authenticated:
            headers["Authorization"] = f"Bearer {token}"
        if body is not None:
            headers["Content-Type"] = "application/json"
        req = Request(base + path, method=method, headers=headers,
                      data=None if body is None else json.dumps(body).encode())
        try:
            response = opener.open(req, timeout=10)
        except HTTPError as error:
            response = error
        with response:
            if response.status != expected:
                raise ValueError(f"{method} {path}: expected HTTP {expected}, received {response.status}")
            data = response.read(2 * 1024 * 1024 + 1)
            if not data:
                return None
            if "application/json" not in response.headers.get("Content-Type", ""):
                raise ValueError(f"{method} {path}: expected JSON Content-Type")
            return json.loads(data)

    def envelope(value):
        if not isinstance(value, dict) or not isinstance(value.get("data"), list) or type(value.get("total")) is not int or value["total"] < len(value["data"]):
            raise ValueError("Invalid data/total envelope")
        return value

    own_session = False
    try:
        if args.username:
            password = os.environ.get("RUSTDESK_VERIFY_PASSWORD") or getpass.getpass("Password: ")
            login = request("POST", "/api/login", {"username": args.username, "password": password}, authenticated=False)
            token = login["access_token"]
            own_session = True
        me = request("GET", "/api/currentUser")
        if not isinstance(me, dict) or type(me.get("is_admin")) is not bool:
            raise ValueError("Invalid current user profile")
        personal = request("POST", "/api/ab/personal")
        if not isinstance(personal, dict) or not isinstance(personal.get("guid"), str) or not personal["guid"]:
            raise ValueError("POST personal: guid missing at root")
        if request("GET", "/api/ab/personal")["data"]["guid"] != personal["guid"]:
            raise ValueError("GET/POST personal book mismatch")
        for method in ("GET", "POST"):
            if "max_peer_one_ab" not in request(method, "/api/ab/settings"):
                raise ValueError("Missing max_peer_one_ab")
            envelope(request(method, "/api/ab/shared/profiles"))
            envelope(request(method, "/api/ab/peers?" + urlencode({"ab": personal["guid"], "current": 1, "pageSize": 1})))
        tags_path = "/api/ab/tags/" + personal["guid"]
        if not isinstance(request("POST", tags_path), list):
            raise ValueError("POST tags must return an array")
        envelope(request("GET", tags_path))
        groups = envelope(request("GET", "/api/device-group/accessible?current=1&pageSize=1"))
        devices = envelope(request("GET", "/api/peers?accessible=&status=1&current=1&pageSize=1"))
        users = envelope(request("GET", "/api/users?accessible=&status=1&current=1&pageSize=1"))
        if not me.get("is_admin") and (devices["total"] != 0 or devices["data"] or users["total"] != 1):
            raise ValueError("Ordinary account received an unrestricted inventory/profile list")
        for profile in users["data"]:
            if not isinstance(profile, dict) or not isinstance(profile.get("name"), str) or not isinstance(profile.get("display_name"), str) or profile.get("status") != 1:
                raise ValueError("Invalid accessible user profile")
        for device in devices["data"]:
            if not isinstance(device, dict) or not isinstance(device.get("id"), str) or not isinstance(device.get("info"), dict):
                raise ValueError("Invalid device payload")
            if not {"device_name", "os", "version", "username"}.issubset(device["info"]) or not {"status", "user", "user_name", "device_group_name", "note"}.issubset(device):
                raise ValueError("Missing device payload fields")
        if groups["total"] != 0:
            raise ValueError("Unexpected device groups: verify ownership/ACL implementation")
        for path in ("/api/ab/personal", "/api/ab/settings", "/api/ab/shared/profiles", "/api/ab/peers", tags_path):
            request("POST", path, authenticated=False, expected=401)
        for path in ("/api/peers", "/api/users?accessible=&status=1", "/api/device-group/accessible"):
            request("GET", path, authenticated=False, expected=401)
        if "error" not in request("GET", "/api/__compatibility_probe_not_found__", expected=404):
            raise ValueError("Unknown API route must return JSON error")
        print("Client query contracts OK (GET/POST, JSON, authentication, device visibility).")
        print("No devices or user accounts were created/modified; real-client UI remains to be checked.")
    finally:
        if own_session:
            request("POST", "/api/logout", {})


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, AttributeError, OSError) as error:
        print(f"Client API verification failed: {error}", file=sys.stderr)
        sys.exit(1)
