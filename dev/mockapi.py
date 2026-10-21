#!/usr/bin/env python3
"""A stand-in for the WaniKani API, for testing Tanuki end to end.

Serves the endpoints the app uses. The review count is read from a
file, so a test can change it while the app is running, and the level
is read from a second one. The next unlock is a couple of minutes away
so the app polls promptly.

    python3 dev/mockapi.py /tmp/tanuki-count 8731 /tmp/tanuki-requests.log

Then point the app at it with TANUKI_API_URL and put a token in place:

    echo a-token > ~/Library/Application\ Support/com.tanuki.notifier/token
    chmod 600 ~/Library/Application\ Support/com.tanuki.notifier/token
    TANUKI_API_URL=http://127.0.0.1:8731/v2 ./target/release/tanuki
"""
import datetime
import http.server
import json
import sys

COUNT_FILE = sys.argv[1] if len(sys.argv) > 1 else "/tmp/tanuki-count"
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 8731
LOG_FILE = sys.argv[3] if len(sys.argv) > 3 else "/tmp/tanuki-requests.log"
LEVEL_FILE = "/tmp/tanuki-level"


def read_level():
    try:
        with open(LEVEL_FILE) as handle:
            return int(handle.read().strip())
    except Exception:
        return 1


def read_count():
    try:
        with open(COUNT_FILE) as handle:
            return int(handle.read().strip())
    except Exception:
        return 0


def hour_from_now(minutes):
    """A timestamp `minutes` in the future, on the hour."""
    moment = datetime.datetime.now(datetime.timezone.utc) + datetime.timedelta(
        minutes=minutes
    )
    # Round up to the next hour so it is always in the future.
    moment = moment.replace(minute=0, second=0, microsecond=0)
    if moment <= datetime.datetime.now(datetime.timezone.utc):
        moment += datetime.timedelta(hours=1)
    return moment.isoformat().replace("+00:00", "Z")


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        with open(LOG_FILE, "a") as log:
            log.write(f"{self.path} count={read_count()}\n")

        if "user" in self.path:
            body = json.dumps(
                {
                    "object": "user",
                    "data": {
                        "id": "1",
                        "username": "tester",
                        "level": read_level(),
                        "started_at": "2024-01-01T00:00:00Z",
                    },
                }
            )
        elif "assignments" in self.path:
            body = json.dumps(
                {"object": "collection", "total_count": read_count(), "data": []}
            )
        elif "summary" in self.path:
            body = json.dumps(
                {
                    "object": "report",
                    "data_updated_at": hour_from_now(0),
                    "data": {
                        "lessons": [],
                        "next_reviews_at": hour_from_now(1),
                        "reviews": [
                            {
                                "available_at": hour_from_now(0),
                                "subject_ids": [],
                            },
                            {
                                "available_at": hour_from_now(1),
                                "subject_ids": [1, 2],
                            },
                        ],
                    },
                }
            )
        else:
            self.send_response(404)
            self.end_headers()
            return

        payload = body.encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    server = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    print(f"mock api on http://127.0.0.1:{PORT}/v2", flush=True)
    server.serve_forever()
