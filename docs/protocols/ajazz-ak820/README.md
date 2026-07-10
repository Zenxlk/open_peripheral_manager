# Ajazz AK820 — protocol notes

Status: not started. No reverse engineering has been done yet; this
directory exists so the process has a home from day one.

## Layout

- `captures/` — raw USB/HID captures (`usbmon`, Wireshark/`usbpcap`
  dumps, etc.). Ignored by git (see root `.gitignore`) because these are
  large binary artifacts; only the analysis derived from them belongs in
  version control.
- `findings.md` — running notes on what's been figured out: report
  descriptors, command byte layouts, known opcodes, open questions.

## Suggested capture workflow (to refine once started)

1. Capture known-good interactions from the vendor's official software
   (e.g. via `usbmon` on Linux or Wireshark's USBPcap).
2. Save the raw capture into `captures/` with a descriptive name and the
   date.
3. Write up what the capture shows in `findings.md`, referencing the
   capture file by name.

## Hardware identity

Captured via a throwaway `hidapi`/`hidreport` probe (see
[`docs/architecture/discovery.md`](../../architecture/discovery.md)'s
Findings section for the full analysis; raw report descriptors are not
yet in this directory's `captures/` — only the discovery-level JSON
report lives in
[`docs/inventory/captures/ajazz-ak820-2026-07-09.json`](../../inventory/captures/ajazz-ak820-2026-07-09.json)).

- USB VID:PID: `0x0c45:0x800a` (`0x0c45` is Sonix Technology — the OEM
  controller vendor, not Ajazz; `manufacturer_string`/`product_string`
  report `"SONiX"`/`"AK820"`, not the Ajazz brand).
- 4 HID interfaces on one composite USB device:
  - Interface 0 — boot-protocol keyboard only (`usage 0x01/0x06`).
  - Interface 1 — five top-level usage pairs sharing one interface:
    consumer control (`0x0c/0x01`), system control (`0x01/0x80`), a
    second keyboard usage (`0x01/0x06`), mouse (`0x01/0x02`), and a
    vendor-defined channel (`0xffff/0x01`); multiplexes report IDs
    `1, 2, 3, 5, 6`.
  - Interface 2 — dedicated vendor channel, `usage 0xff68/0x61`.
  - Interface 3 — dedicated vendor channel, `usage 0xff13/0x01`.
- No usable serial number (`Some("")`, empty string).
- `/dev/hidraw0`-`3` are root-only by default on a stock Arch install —
  actual protocol work against this device will need a udev rule, not
  just discovery-level enumeration (which needs no permissions at all).

Three distinct vendor usage pages (one shared with interface 1, two
dedicated) is the headline surprise here — likely separate command
channels for different features. Which one does what (RGB? macros?
profiles?) is exactly what the reverse-engineering pass into this
directory (Phase 6, not yet started) needs to answer; discovery
deliberately stops before that.
