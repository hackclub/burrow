# config.rs.in
global_conf = configuration_data()
global_conf.set_quoted('ID', app_id)
global_conf.set_quoted('VERSION', meson.project_version())
global_conf.set_quoted('LOCALEDIR', localedir)
global_conf.set_quoted('GETTEXT_PACKAGE', app_name)
config = configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: global_conf,
)

run_command(
    'cp',
    meson.project_build_root() / 'src' / 'config.rs',
    meson.project_source_root() / 'src',
    check: true,
)

# Cargo Build
cargo_build = custom_target(
    'cargo-build',
    build_by_default: true,
    build_always_stale: true,
    output: meson.project_name(),
    console: true,
    install: true,
    install_dir: get_option('bindir'),
    command: [
        'env', cargo_env,
        cargo_bin, 'build',
        cargo_opt, '&&', 'cp', 'target' / rust_target / meson.project_name(), '@OUTPUT@',
    ]
  )
