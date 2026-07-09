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

To be filled in once the device is in hand (USB vendor id / product id,
interface count, HID report descriptor).
