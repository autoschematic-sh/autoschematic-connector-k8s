#![no_main]

use autoschematic_connector_k8s::addr::K8sResourceAddress;
use autoschematic_core::connector::ResourceAddress;
use libfuzzer_sys::fuzz_target;

#[cfg(fuzzing)]
fuzz_target!(|addr: K8sResourceAddress| {
    let path = addr.to_path_buf();
    let Ok(Some(new_addr)) = K8sResourceAddress::from_path(&path) else {
        return;
    };
    println!("{:?} -> {:?} -> {:?}", addr, path, new_addr);

    assert_eq!(addr, new_addr);
});
