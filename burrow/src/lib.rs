// // Check capabilities on Linux
// #[cfg(target_os = "linux")]
// fn ensure_root() {
//     use caps::{has_cap, CapSet, Capability};

//     let cap_net_admin = Capability::CAP_NET_ADMIN;
//     if let Ok(has_cap) = has_cap(None, CapSet::Effective, cap_net_admin) {
//         if !has_cap {
//             eprintln!("This program must be run with CAP_NET_ADMIN!");
//             std::process::exit(1);
//         }
//     } else {
//         eprintln!("Failed to check capabilities");
//         std::process::exit(1);
//     }
// }

// Check for root user on macOS
#[cfg(target_family = "unix")]
pub fn ensure_root() {
    use nix::unistd::Uid;

    let current_uid = Uid::current();
    if !current_uid.is_root() {
        eprintln!("This program must be run as root!");
        std::process::exit(1);
    }
}

#[cfg(target_family = "windows")]
pub fn ensure_root() {
    todo!()
}
