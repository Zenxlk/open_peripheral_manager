# 0007. `GROUP=`/`MODE=` udev rule, not `uaccess`, for `LibusbTransport` devices

Date: 2026-07-23

Status: Accepted

## Context

Since ADR 0004, `pmctl rgb`/`lighting`/`sleep`/`preset apply` have all
needed `sudo` in practice, despite `docs/protocols/ajazz-ak820/
99-ak820-usb.rules` installing a `SUBSYSTEM=="usb", TAG+="uaccess"`
rule — carried as a known gap in `findings.md`/`roadmap.md` since
2026-07-20 ("the resulting `uaccess` ACL never actually appeared...
cause not diagnosed, not worth blocking on"). It became worth
diagnosing once packaging (a `PKGBUILD`) came up: a package that
auto-installs a udev rule that doesn't actually work would just move
the same broken `sudo`-every-time experience one step earlier, not fix
it.

**Root cause, found by direct comparison against the real hardware**
(`udevadm info`, `getfacl`, `loginctl seat-status` on a real, active,
properly-registered `seat0` session — ruling out "no active session"
as the cause): the rule's `TAG+="uaccess"` *does* get applied —
`udevadm info` shows `TAGS=:seat:uaccess:` on the raw USB device node
— but `systemd-logind` never turns that tag into an actual ACL entry
for the session's user on that specific node. The AK820's `hidraw`
nodes (`/dev/hidraw1`-`4`, tagged `uaccess` by the *separate*,
already-working hidraw rule from `transport.md`'s Phase 2 design) *do*
get a real `user:<name>:rw-` ACL entry from `getfacl`. The only
difference between the two: the raw USB device node
(`/dev/bus/usb/BBB/DDD`) is a `SUBSYSTEM=="usb"`, `DEVTYPE=="usb_device"`
node with a kernel driver (`usbhid`/`hid-generic`) bound to it and
recognized as an input/keyboard device at the time udev evaluates
rules — `hid-generic` stays bound to interfaces the AK820 isn't
actively being talked to over `LibusbTransport` at that moment
(detachment is transient, only for the duration of one `Driver::open()`
call). `systemd-logind` deliberately declines to hand out ACL'd raw
USB access to a device already claimed by the kernel as an input
device — a real security boundary (raw USB access to a keyboard's
device node is a much bigger foothold than a scoped `hidraw` node),
not a bug in this project's rule.

**Verified empirically, not just reasoned about**: temporarily
installing `SUBSYSTEM=="usb", ATTR{idVendor}=="0c45",
ATTR{idProduct}=="800a", MODE="0660", GROUP="wheel"` in place of the
`uaccess` rule immediately produced a real `group: wheel` ACL (no
`uaccess`/logind involvement at all — this is udev's own, simpler
`GROUP=`/`MODE=` mechanism, unconditional and independent of session
state) and `pmctl rgb set` worked for real, with no `sudo`, for a user
already in `wheel`.

## Decision

Change `99-ak820-usb.rules`' `SUBSYSTEM=="usb"` line from `TAG+="uaccess"`
to `MODE="0660", GROUP="opm"` — a new, project-wide (not AK820-specific)
group, so any future `LibusbTransport`-based driver's udev rule reuses
the same group rather than inventing a new one per device. Users join
the group once (`sudo usermod -aG opm $USER`, then log out/in — a
one-time step, same shape as `docker`/`wireshark`'s well-established
group-based device access pattern), after which every `pmctl` command
touching hardware works with no further `sudo`.

The **hidraw** rule (`70-...rules`, `transport.md`'s Phase 2 design,
`TAG+="uaccess"`) is **unchanged** — it already works correctly (`uaccess`
does get ACL'd there, confirmed above) and `transport.md` already
recorded the reasoning for preferring `uaccess` over a `plugdev`-style
group where it's available. This ADR doesn't contradict that reasoning
generally — it identifies one specific case (`LibusbTransport`'s raw
USB device node, on a device the kernel also recognizes as an input
device) where `uaccess` provably doesn't work, and uses the group
mechanism only there.

## Consequences

- `pmctl rgb`/`lighting`/`sleep`/`preset apply` work with **zero**
  `sudo` after a one-time group join — the actual goal that made
  packaging (`PKGBUILD`) worth revisiting this for. A package's
  `post_install()` can create the `opm` group and install the rule
  automatically; it still can't automatically add the *interactively
  logged-in desktop user* to a group during a `pacman` transaction
  (no reliable way to know who that is from a possibly-headless
  install), so `usermod -aG opm $USER` + logout/login stays a
  documented, one-time manual step — not eliminated, but reduced from
  "every single run" to "once, ever."
- The two rules now use genuinely different mechanisms for a
  documented reason, not by accident — worth remembering if this
  project ever adds a driver that needs *only* `hidraw`, not
  `LibusbTransport`: it can keep using plain `uaccess` and skip the
  group entirely, per `transport.md`'s original reasoning.
- `GROUP=`/`MODE=` is **not** session-scoped like `uaccess` is — any
  member of `opm`, in any session (not just the active graphical one),
  can access the device at any time. A real, accepted trade-off
  against `uaccess`'s tighter security model, made because `uaccess`
  is unavailable here at all, not chosen for convenience over a
  working `uaccess` alternative.
- Stale local rule files from earlier debugging (a `wheel`-group test
  rule, and an old `99-ak820.rules` targeting gohv's PID `0x8009`,
  never matching this project's real `0x800a` device) exist on the
  development machine outside version control — cleanup is manual,
  not this project's concern to track.
