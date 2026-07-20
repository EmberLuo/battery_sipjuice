#!/usr/bin/env python3
"""Render README screenshots from the current frontend with isolated demo data."""

import json
import os
import threading
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

os.environ.setdefault("WEBKIT_DISABLE_COMPOSITING_MODE", "1")
os.environ.setdefault("LIBGL_ALWAYS_SOFTWARE", "1")

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import GLib, Gtk, WebKit2  # noqa: E402


ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "docs" / "screenshots"
OUTPUT.mkdir(parents=True, exist_ok=True)

NOW = 1_784_530_800_000

BATTERY = {
    "device": "BAT0",
    "model": "SipBook Pro 14",
    "manufacturer": "Example Devices",
    "serial_number": "BSJ-DEMO-2026",
    "technology": "Li-ion",
    "present": True,
    "capacity": 76,
    "status": "Charging",
    "health_status": "Good",
    "full_capacity": {"value": 52.4, "unit": "Wh", "source_kind": "energy"},
    "design_capacity": {"value": 57.8, "unit": "Wh", "source_kind": "energy"},
    "health_percent": 90.7,
    "cycle_count": 186,
    "state_of_health": 91,
    "voltage_now": 12.18,
    "voltage_ocv": 12.45,
    "voltage_max": 13.05,
    "current_now": 1510.0,
    "power_now": 18.4,
    "temperature": 34.2,
    "internal_resistance": 82.0,
}

SOURCES = [
    {
        "name": "ucsi-source-psy-USBC000:001",
        "kind": "USB_C",
        "online": True,
        "voltage_now": 20.0,
        "current_now": 1575.0,
        "current_max": 3250.0,
        "power_now": 31.5,
        "usb_type": "PD",
    }
]

SETTINGS = {
    "language": "en-US",
    "theme": "light",
    "accent_color": "orange",
    "autostart": False,
    "silent_start": False,
    "close_action": "ask",
    "remind_charge": True,
    "remind_charge_at": 30,
    "remind_unplug": True,
    "remind_unplug_at": 80,
    "remind_temp_high": True,
    "remind_temp_high_at": 45,
    "remind_temp_low": True,
    "remind_temp_low_at": 5,
    "remind_drain": True,
    "remind_drain_at": 30,
}


def health_snapshot(days_ago, health, cycles, full):
    return {
        "battery_id": "serial:BSJ-DEMO-2026",
        "battery_device": "BAT0",
        "recorded_at_ms": NOW - days_ago * 86_400_000,
        "full_capacity": {"value": full, "unit": "Wh"},
        "design_capacity": {"value": 57.8, "unit": "Wh"},
        "health_percent": health,
        "state_of_health": round(health),
        "cycle_count": cycles,
    }


HEALTH = [
    health_snapshot(180, 93.8, 142, 54.2),
    health_snapshot(150, 93.3, 149, 53.9),
    health_snapshot(120, 92.8, 157, 53.6),
    health_snapshot(90, 92.1, 165, 53.2),
    health_snapshot(60, 91.7, 173, 53.0),
    health_snapshot(30, 91.2, 180, 52.7),
    health_snapshot(0, 90.7, 186, 52.4),
]


def session(index, start_hours, start_capacity, end_capacity, input_power, temp):
    start = NOW - start_hours * 3_600_000
    duration = 78 * 60_000
    return {
        "id": f"demo-{index}",
        "battery_id": "serial:BSJ-DEMO-2026",
        "battery_device": "BAT0",
        "battery_name": "SipBook Pro 14",
        "start_ms": start,
        "end_ms": start + duration,
        "start_capacity": start_capacity,
        "end_capacity": end_capacity,
        "charged_percent": end_capacity - start_capacity,
        "duration_ms": duration,
        "charging_ms": 72 * 60_000,
        "battery_energy_wh": 25.8 - index * 0.7,
        "input_energy_wh": 38.1 - index * 0.8,
        "charged_mah": 2180 - index * 45,
        "average_battery_power_w": 21.5 - index * 0.6,
        "average_input_power_w": input_power,
        "peak_input_power_w": input_power + 8.4,
        "average_temperature_c": temp - 2.1,
        "peak_temperature_c": temp,
        "source_names": ["ucsi-source-psy-USBC000:001"],
        "source_kinds": ["USB_C"],
        "usb_types": ["PD"],
        "sample_count": 156,
        "powered_sample_count": 154,
        "health_percent_end": 90.7,
        "cycle_count_end": 186,
        "complete": True,
        "end_reason": "unplugged",
        "active": False,
    }


SESSIONS = [
    {
        **session(0, 1, 68, 76, 31.5, 34.2),
        "end_ms": NOW,
        "duration_ms": 28 * 60_000,
        "charging_ms": 28 * 60_000,
        "complete": False,
        "end_reason": "active",
        "active": True,
    },
    session(1, 28, 24, 81, 33.2, 37.1),
    session(2, 76, 31, 80, 31.7, 36.4),
]


MOCK_SCRIPT = f"""
(() => {{
  const battery = {json.dumps(BATTERY)};
  const sources = {json.dumps(SOURCES)};
  const settings = {json.dumps(SETTINGS)};
  const sessions = {json.dumps(SESSIONS)};
  const health = {json.dumps(HEALTH)};
  const baseNow = Date.now();

  function history(sourceKind = "battery") {{
    return Array.from({{ length: 121 }}, (_, index) => {{
      const progress = index / 120;
      const charging = index > 18;
      const wave = Math.sin(index / 7);
      return {{
        timestamp_ms: baseNow - (120 - index) * 30_000,
        capacity: Math.round(67 + progress * 9),
        temperature: 31.2 + progress * 3 + wave * 0.25,
        power_w: sourceKind === "input" ? 31.5 + wave * 2.4 : 17.8 + wave * 1.7,
        voltage: sourceKind === "input" ? 20 : 11.7 + progress * 0.48,
        current_ma: sourceKind === "input" ? 1575 + wave * 90 : 1510 + wave * 110,
        charging,
      }};
    }});
  }}

  async function invoke(command, args = {{}}) {{
    switch (command) {{
      case "get_snapshot":
        return {{ batteries: [battery], sources, timestamp_ms: baseNow }};
      case "get_history":
        return history(args.sourceKind ?? "battery");
      case "get_settings":
        return settings;
      case "get_app_version":
        return "0.4.1";
      case "get_system_accent_color":
        return null;
      case "get_battery_insights":
        return {{ sessions, health }};
      case "get_app_power_report":
        return {{
          total_energy: [
            {{ name: "Code", power_w: 4.1, energy_wh: 0.182, cpu_share: 0.224, process_count: 9 }},
            {{ name: "Firefox", power_w: 3.4, energy_wh: 0.147, cpu_share: 0.187, process_count: 12 }},
            {{ name: "GNOME Shell", power_w: 1.2, energy_wh: 0.052, cpu_share: 0.066, process_count: 1 }},
          ],
          current_power: [
            {{ name: "Code", power_w: 4.1, energy_wh: 0.182, cpu_share: 0.224, process_count: 9 }},
            {{ name: "Firefox", power_w: 3.4, energy_wh: 0.147, cpu_share: 0.187, process_count: 12 }},
            {{ name: "GNOME Shell", power_w: 1.2, energy_wh: 0.052, cpu_share: 0.066, process_count: 1 }},
          ],
        }};
      default:
        return null;
    }}
  }}

  window.__TAURI__ = {{
    core: {{ invoke }},
    event: {{ listen: async () => () => {{}} }},
  }};
}})();
"""


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, _format, *_args):
        pass


class Renderer:
    shots = ["overview", "monitor", "sessions", "health"]

    def __init__(self):
        handler = partial(QuietHandler, directory=str(ROOT / "src"))
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        manager = WebKit2.UserContentManager()
        manager.add_script(
            WebKit2.UserScript.new(
                MOCK_SCRIPT,
                WebKit2.UserContentInjectedFrames.ALL_FRAMES,
                WebKit2.UserScriptInjectionTime.START,
                None,
                None,
            )
        )
        self.webview = WebKit2.WebView.new_with_user_content_manager(manager)
        self.webview.get_settings().set_hardware_acceleration_policy(
            WebKit2.HardwareAccelerationPolicy.NEVER
        )
        self.window = Gtk.OffscreenWindow()
        self.window.set_default_size(980, 720)
        self.window.add(self.webview)
        self.window.show_all()
        self.index = 0
        self.webview.connect("load-changed", self.on_load_changed)
        port = self.server.server_address[1]
        self.webview.load_uri(f"http://127.0.0.1:{port}/index.html")

    def on_load_changed(self, _webview, event):
        if event == WebKit2.LoadEvent.FINISHED:
            GLib.timeout_add(1800, self.select_and_capture)

    def select_and_capture(self):
        tab = self.shots[self.index]
        script = f"""
          document.querySelector('[data-tab="{tab}"]').click();
          document.querySelector('.content').scrollTop = 0;
        """
        self.webview.evaluate_javascript(script, -1, None, None, None, None, None)
        GLib.timeout_add(900, self.capture)
        return GLib.SOURCE_REMOVE

    def capture(self):
        self.webview.get_snapshot(
            WebKit2.SnapshotRegion.VISIBLE,
            WebKit2.SnapshotOptions.NONE,
            None,
            self.on_snapshot,
            self.shots[self.index],
        )
        return GLib.SOURCE_REMOVE

    def on_snapshot(self, webview, result, name):
        surface = webview.get_snapshot_finish(result)
        path = OUTPUT / f"{name}.png"
        surface.write_to_png(str(path))
        print(f"Rendered {path}")
        self.index += 1
        if self.index >= len(self.shots):
            self.server.shutdown()
            Gtk.main_quit()
        else:
            GLib.timeout_add(300, self.select_and_capture)


Renderer()
Gtk.main()
