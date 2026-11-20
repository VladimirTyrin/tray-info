use anyhow::{Result, bail};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_GETIDLIST_FILTER_ENUMERATOR, CM_GETIDLIST_FILTER_PRESENT, CM_Get_DevNode_PropertyW,
    CM_Get_Device_ID_List_SizeW, CM_Get_Device_ID_ListW, CM_LOCATE_DEVNODE_NORMAL,
    CM_Locate_DevNodeW, CR_BUFFER_SMALL, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::{
    DEVPROP_TYPE_BOOLEAN, DEVPROP_TYPE_BYTE, DEVPROP_TYPE_STRING, DEVPROP_TYPE_STRING_LIST,
    DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::HSTRING;

use super::multi_sz::parse_multi_sz;
use super::wide_text::{string_from_le_bytes, utf16_units};

/// The device list can grow between the size query and the read; retry a few times.
const LIST_ATTEMPTS: usize = 4;

/// Instance IDs of present devices created by the given enumerator (e.g. `BTHLE`).
pub fn present_device_ids(enumerator: &str) -> Result<Vec<String>> {
    let filter = HSTRING::from(enumerator);
    let flags = CM_GETIDLIST_FILTER_ENUMERATOR | CM_GETIDLIST_FILTER_PRESENT;
    for _ in 0..LIST_ATTEMPTS {
        let mut length = 0u32;
        let status = unsafe { CM_Get_Device_ID_List_SizeW(&mut length, &filter, flags) };
        if status != CR_SUCCESS {
            bail!(
                "CM_Get_Device_ID_List_SizeW({enumerator}) failed with CONFIGRET {}",
                status.0
            );
        }
        let mut buffer = vec![0u16; length as usize];
        let status = unsafe { CM_Get_Device_ID_ListW(&filter, &mut buffer, flags) };
        if status == CR_BUFFER_SMALL {
            continue;
        }
        if status != CR_SUCCESS {
            bail!(
                "CM_Get_Device_ID_ListW({enumerator}) failed with CONFIGRET {}",
                status.0
            );
        }
        return Ok(parse_multi_sz(&buffer));
    }
    bail!("device list for {enumerator} kept changing while being read")
}

/// Handle to a device node in the configuration manager tree.
pub struct DevNode(u32);

impl DevNode {
    pub fn locate(instance_id: &str) -> Option<Self> {
        let id = HSTRING::from(instance_id);
        let mut handle = 0u32;
        let status = unsafe { CM_Locate_DevNodeW(&mut handle, &id, CM_LOCATE_DEVNODE_NORMAL) };
        (status == CR_SUCCESS).then_some(Self(handle))
    }

    pub fn byte(&self, key: &DEVPROPKEY) -> Option<u8> {
        self.property(key, DEVPROP_TYPE_BYTE)?.first().copied()
    }

    pub fn boolean(&self, key: &DEVPROPKEY) -> Option<bool> {
        // DEVPROP_BOOLEAN is a single byte: 0 is false, 0xFF is true.
        self.property(key, DEVPROP_TYPE_BOOLEAN)?
            .first()
            .map(|&value| value != 0)
    }

    pub fn string(&self, key: &DEVPROPKEY) -> Option<String> {
        Some(string_from_le_bytes(
            &self.property(key, DEVPROP_TYPE_STRING)?,
        ))
    }

    pub fn string_list(&self, key: &DEVPROPKEY) -> Option<Vec<String>> {
        let units = utf16_units(&self.property(key, DEVPROP_TYPE_STRING_LIST)?);
        Some(parse_multi_sz(&units))
    }

    /// Returns `None` when the property is missing or has an unexpected type;
    /// both are normal for devices that simply do not report it.
    fn property(&self, key: &DEVPROPKEY, expected: DEVPROPTYPE) -> Option<Vec<u8>> {
        let mut kind = DEVPROPTYPE::default();
        let mut size = 0u32;
        let status =
            unsafe { CM_Get_DevNode_PropertyW(self.0, key, &mut kind, None, &mut size, 0) };
        if status != CR_BUFFER_SMALL || kind != expected {
            return None;
        }
        let mut buffer = vec![0u8; size as usize];
        let status = unsafe {
            CM_Get_DevNode_PropertyW(
                self.0,
                key,
                &mut kind,
                Some(buffer.as_mut_ptr()),
                &mut size,
                0,
            )
        };
        if status != CR_SUCCESS || kind != expected {
            return None;
        }
        buffer.truncate(size as usize);
        Some(buffer)
    }
}
