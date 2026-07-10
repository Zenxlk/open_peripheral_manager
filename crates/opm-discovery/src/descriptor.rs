//! Report-descriptor-level information `hidapi` doesn't surface: the set
//! of report IDs an interface declares. See `discovery.md`'s "What
//! information belongs to the HID descriptor?" — this deliberately goes
//! no deeper than counting report IDs; field-level meaning is protocol
//! reverse-engineering, out of scope here.

use std::collections::BTreeSet;
use std::path::Path;

use hidreport::{Report, ReportDescriptor};

use crate::error::Error;

/// Reads and parses the report descriptor for the `hidraw` interface at
/// `path` (e.g. `/dev/hidraw0`), returning its distinct report IDs
/// across input, output, and feature reports. Reads the world-readable
/// sysfs `report_descriptor` file — no special permissions needed, and
/// no report ever read/written to the device itself.
pub fn report_ids(path: &str) -> Result<Vec<u8>, Error> {
    let name = Path::new(path).file_name().ok_or_else(|| Error::Sysfs {
        path: path.to_owned(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a hidraw path"),
    })?;
    let descriptor_path = Path::new("/sys/class/hidraw")
        .join(name)
        .join("device/report_descriptor");

    let bytes = std::fs::read(&descriptor_path).map_err(|source| Error::Sysfs {
        path: descriptor_path.display().to_string(),
        source,
    })?;

    let descriptor = ReportDescriptor::try_from(bytes.as_slice())?;

    let mut ids = BTreeSet::new();
    for report in descriptor.input_reports() {
        collect(report, &mut ids);
    }
    for report in descriptor.output_reports() {
        collect(report, &mut ids);
    }
    for report in descriptor.feature_reports() {
        collect(report, &mut ids);
    }

    Ok(ids.into_iter().collect())
}

fn collect<R: Report>(report: &R, ids: &mut BTreeSet<u8>) {
    if let Some(id) = report.report_id() {
        ids.insert((*id).into());
    }
}
