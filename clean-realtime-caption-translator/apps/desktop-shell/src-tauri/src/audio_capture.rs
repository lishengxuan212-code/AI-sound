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
    #[cfg(target_os = "windows")]
    {
        return default_wasapi_output_device_name();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let host = cpal::default_host();
        host.default_output_device()
            .and_then(|device| device.name().ok())
    }
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

pub fn list_output_devices() -> Vec<AudioDeviceSummary> {
    #[cfg(target_os = "windows")]
    {
        return list_wasapi_output_devices();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let host = cpal::default_host();
        let default = default_output_device_name();
        host.output_devices()
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
}

#[cfg(target_os = "windows")]
fn default_wasapi_output_device_name() -> Option<String> {
    let _ = wasapi::initialize_mta().ok();
    let enumerator = wasapi::DeviceEnumerator::new().ok()?;
    enumerator
        .get_default_device(&wasapi::Direction::Render)
        .ok()
        .and_then(|device| device.get_friendlyname().ok())
}

#[cfg(target_os = "windows")]
fn list_wasapi_output_devices() -> Vec<AudioDeviceSummary> {
    let _ = wasapi::initialize_mta().ok();
    let default = default_wasapi_output_device_name();
    let Ok(enumerator) = wasapi::DeviceEnumerator::new() else {
        return Vec::new();
    };
    let Ok(collection) = enumerator.get_device_collection(&wasapi::Direction::Render) else {
        return Vec::new();
    };
    let mut devices = Vec::new();
    for device in &collection {
        let Ok(device) = device else {
            continue;
        };
        let Ok(name) = device.get_friendlyname() else {
            continue;
        };
        devices.push(AudioDeviceSummary {
            is_default: default.as_ref() == Some(&name),
            name,
        });
    }
    devices
}

// MVP boundary: actual stream construction lives behind the session modules.
// System loopback capture on Windows must stay isolated from microphone capture.
