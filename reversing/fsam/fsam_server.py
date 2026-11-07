# uci set arc_cloud.cloud.ARC_OWL_CLOUD_FSAM_URL='http://<ip>:18080'
# lxc-stop -k fsam
# lxc-start fsam

import hashlib
import json
import zlib
from datetime import UTC, datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import requests

LISTEN = ("0.0.0.0", 18080)

CAPTURE_DIR = Path("captures")
CAPTURE_DIR.mkdir(exist_ok=True)

session = requests.Session()


class FSAMHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)

        now = datetime.now(UTC)
        stamp = now.strftime("%Y%m%dT%H%M%S.%fZ")
        digest = hashlib.sha256(body).hexdigest()

        safe_path = self.path.strip("/").replace("/", "_") or "root"
        stem = CAPTURE_DIR / f"{stamp}_{safe_path}_{digest[:12]}"

        # exact bytes arc_cloud was about to send
        stem.with_suffix(".deflate").write_bytes(body)

        print(f"\n{now.isoformat()}")
        print(f"{self.command} {self.path}")
        print(f"compressed: {len(body)} bytes")
        print(f"sha256: {digest}")

        try:
            raw = zlib.decompress(body)
            stem.with_suffix(".decoded").write_bytes(raw)

            # arc_cloud appears to compress the null terminator too?
            json_bytes = raw.rstrip(b"\x00")

            try:
                obj = json.loads(json_bytes)
                stem.with_suffix(".json").write_text(
                    json.dumps(obj, indent=2),
                    encoding="utf-8",
                )
                print(json.dumps(obj, indent=2))
            except Exception as e:
                print(f"decoded body isn't JSON: {e}")
        except Exception as e:
            print(f"zlib decode failed: {e}")

        msg = b'{"result":"OK"}'
        self.send_response(200)
        self.send_header("Content-Length", str(len(msg)))
        self.end_headers()
        self.wfile.write(msg)

    def log_message(self, format, *args):
        # shut up BaseHTTPRequestHandler's logging
        pass


print(f"listening on http://{LISTEN[0]}:{LISTEN[1]}")
ThreadingHTTPServer(LISTEN, FSAMHandler).serve_forever()
