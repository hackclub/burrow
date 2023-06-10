// Check capabilities on Linux
#[cfg(target_os = "linux")]
pub fn ensure_root() {
    use caps::{has_cap, CapSet, Capability};

    let cap_net_admin = Capability::CAP_NET_ADMIN;
    if let Ok(has_cap) = has_cap(None, CapSet::Effective, cap_net_admin) {
        if !has_cap {
            eprintln!(
                "This action needs the CAP_NET_ADMIN permission. Did you mean to run it as root?"
            );
            std::process::exit(77);
        }
    } else {
        eprintln!("Failed to check capabilities. Please file a bug report!");
        std::process::exit(71);
    }
}

// Check for root user on macOS
#[cfg(target_os = "macos")]
pub fn ensure_root() {
    use nix::unistd::Uid;

    let current_uid = Uid::current();
    if !current_uid.is_root() {
        eprintln!("This action must be run as root!");
        std::process::exit(77);
    }
}

#[cfg(target_family = "windows")]
pub fn ensure_root() {
    todo!()
}
