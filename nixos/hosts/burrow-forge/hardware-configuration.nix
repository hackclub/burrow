{ ... }:

{
  # Derived from Hetzner Cloud rescue-mode hardware inspection.
  boot.initrd.availableKernelModules = [
    "ahci"
    "sd_mod"
    "virtio_pci"
    "virtio_scsi"
  ];
}
