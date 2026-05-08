#![allow(dead_code)]

use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AudioDeviceSummary {
    pub name: String,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
}

pub fn default_input_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_input_device()
        .and_then(|device| device.name().ok())
}

pub fn default_output_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_output_device()
        .and_then(|device| device.name().ok())
}

pub fn list_input_devices() -> Vec<AudioDeviceSummary> {
    let host = cpal::default_host();
    let default = default_input_device_name();
    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|device| device.name().ok())
                .map(|name| AudioDeviceSummary {
                    is_default: default.as_ref() == Some(&name),
                    name,
                })
                .collect()
        })
        .unwrap_or_default()
}

// MVP boundary: actual stream construction lives behind the session modules.
// System loopback capture on Windows must stay isolated from microphone capture.
