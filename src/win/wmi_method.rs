use anyhow::anyhow;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL,
    RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WBEM_FLAG_FORWARD_ONLY,
    WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE, WBEM_INFINITE, WbemLocator,
};
use windows::core::{BSTR, HSTRING, PCWSTR};

use super::{WmiArgument, WmiFailure};

/// A WMI method on the first instance of a class, with at most one fixed argument, bound once
/// and called repeatedly. Requires COM to be initialised on the calling thread.
pub struct WmiMethod {
    services: IWbemServices,
    object_path: BSTR,
    method: BSTR,
    /// Input parameters object, filled once at bind time.
    input: Option<IWbemClassObject>,
    output_property: HSTRING,
}

impl WmiMethod {
    pub fn bind(
        namespace: &str,
        class: &str,
        method: &str,
        argument: Option<WmiArgument>,
        output_property: &str,
    ) -> Result<Self, WmiFailure> {
        let services = connect(namespace)?;
        let query = BSTR::from(format!("SELECT __PATH FROM {class}"));
        let instances = unsafe {
            services.ExecQuery(
                &BSTR::from("WQL"),
                &query,
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                None,
            )
        }
        .map_err(|error| WmiFailure::from_windows(error, "WMI ExecQuery failed"))?;

        let mut found: [Option<IWbemClassObject>; 1] = [None];
        let mut returned = 0u32;
        // With RETURN_IMMEDIATELY, errors such as access denied surface here rather than in ExecQuery.
        unsafe { instances.Next(WBEM_INFINITE, &mut found, &mut returned) }
            .ok()
            .map_err(|error| WmiFailure::from_windows(error, "WMI enumeration failed"))?;
        let instance = found[0]
            .take()
            .ok_or_else(|| WmiFailure::Other(anyhow!("no instance of {namespace}:{class}")))?;
        let object_path = read_property(&instance, "__PATH").and_then(|value| {
            BSTR::try_from(&value)
                .map_err(|error| WmiFailure::from_windows(error, "__PATH is not a string"))
        })?;

        let input = argument
            .map(|argument| input_parameters(&services, class, method, argument))
            .transpose()?;

        Ok(Self {
            services,
            object_path,
            method: BSTR::from(method),
            input,
            output_property: HSTRING::from(output_property),
        })
    }

    pub fn call_i32(&self) -> Result<i32, WmiFailure> {
        let mut output: Option<IWbemClassObject> = None;
        unsafe {
            self.services.ExecMethod(
                &self.object_path,
                &self.method,
                WBEM_GENERIC_FLAG_TYPE(0),
                None,
                self.input.as_ref(),
                Some(&mut output),
                None,
            )
        }
        .map_err(|error| WmiFailure::from_windows(error, "WMI ExecMethod failed"))?;
        let output =
            output.ok_or_else(|| WmiFailure::Other(anyhow!("WMI method returned no output")))?;
        let value = read_property(&output, &self.output_property.to_string())?;
        i32::try_from(&value)
            .map_err(|error| WmiFailure::from_windows(error, "WMI output is not an integer"))
    }
}

/// An instance of the method's input signature with `argument` set.
fn input_parameters(
    services: &IWbemServices,
    class: &str,
    method: &str,
    argument: WmiArgument,
) -> Result<IWbemClassObject, WmiFailure> {
    let mut class_object = None;
    unsafe {
        services.GetObject(
            &BSTR::from(class),
            WBEM_GENERIC_FLAG_TYPE(0),
            None,
            Some(&mut class_object),
            None,
        )
    }
    .map_err(|error| WmiFailure::from_windows(error, "WMI GetObject(class) failed"))?;
    let class_object = class_object
        .ok_or_else(|| WmiFailure::Other(anyhow!("WMI returned no class object for {class}")))?;
    let mut signature = None;
    let mut output_signature = None;
    unsafe {
        class_object.GetMethod(
            &HSTRING::from(method),
            0,
            &mut signature,
            &mut output_signature,
        )
    }
    .map_err(|error| WmiFailure::from_windows(error, "WMI GetMethod failed"))?;
    let signature =
        signature.ok_or_else(|| WmiFailure::Other(anyhow!("{class}.{method} takes no input")))?;
    let input = unsafe { signature.SpawnInstance(0) }
        .map_err(|error| WmiFailure::from_windows(error, "WMI SpawnInstance failed"))?;
    // WMI expects CIM uint32 values in a VT_I4 variant; the bits are passed through unchanged.
    let value = VARIANT::from(argument.value as i32);
    unsafe { input.Put(&HSTRING::from(argument.name), 0, &value, 0) }
        .map_err(|error| WmiFailure::from_windows(error, "WMI Put(argument) failed"))?;
    Ok(input)
}

fn connect(namespace: &str) -> Result<IWbemServices, WmiFailure> {
    let locator: IWbemLocator =
        unsafe { CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER) }
            .map_err(|error| WmiFailure::from_windows(error, "cannot create WbemLocator"))?;
    let services = unsafe {
        locator.ConnectServer(
            &BSTR::from(namespace),
            &BSTR::new(),
            &BSTR::new(),
            &BSTR::new(),
            0,
            &BSTR::new(),
            None,
        )
    }
    .map_err(|error| WmiFailure::from_windows(error, "WMI ConnectServer failed"))?;
    // Without impersonation the WMI provider sees an anonymous caller and rejects method calls.
    unsafe {
        CoSetProxyBlanket(
            &services,
            RPC_C_AUTHN_WINNT,
            RPC_C_AUTHZ_NONE,
            PCWSTR::null(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
        )
    }
    .map_err(|error| WmiFailure::from_windows(error, "CoSetProxyBlanket failed"))?;
    Ok(services)
}

fn read_property(object: &IWbemClassObject, name: &str) -> Result<VARIANT, WmiFailure> {
    let name = HSTRING::from(name);
    let mut value = VARIANT::default();
    unsafe { object.Get(&name, 0, &mut value, None, None) }
        .map_err(|error| WmiFailure::from_windows(error, "WMI property read failed"))?;
    Ok(value)
}
