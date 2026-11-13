# on an unrooted router:
# dump /tmp/cert/fullchain.pem from another BHRx router and throw it into this dir
# point sonx-fp.mediasearch.verizon.com to this server via a custom dns server (e.g. adguard home)
# setting a custom host override in the dedicated "dns server" panel doesn't work
# this works because CURLOPT_SSL_VERIFYHOST is 0 in arc_cloud and the cert bundle trusts Fios BHR CA,
# which every router has a router-specific cert for that's allowed to do server auth for some reason
# (no, the non-unique myfiosgateway.com cert doesn't work for this)
# (no, emudhra also doesn't work for this)

# otherwise, on a rooted router:
# set SSL to False, then run this on the router:
# uci set arc_cloud.cloud.ARC_OWL_CLOUD_FSAM_URL='http://<ip>:18080'
# lxc-stop -k fsam
# lxc-start fsam

import hashlib
import json
import ssl
import zlib
from datetime import UTC, datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import requests

SSL = True
LISTEN = ("0.0.0.0", 55668 if SSL else 18080)

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


class LoggingHTTPServer(ThreadingHTTPServer):
    def get_request(self):
        try:
            sock, address = super().get_request()
        except OSError as exc:
            print(f"TLS/accept failed: {exc!r}", flush=True)
            raise
        if isinstance(sock, ssl.SSLSocket):
            print(f"TLS OK from {address}: {sock.version()} {sock.cipher()[0]}", flush=True)
        return sock, address


server = LoggingHTTPServer(LISTEN, FSAMHandler)
if SSL:
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain("fullchain.pem")
    server.socket = context.wrap_socket(server.socket, server_side=True)

print(f"listening on {'https' if SSL else 'http'}://{LISTEN[0]}:{LISTEN[1]}")
server.serve_forever()
