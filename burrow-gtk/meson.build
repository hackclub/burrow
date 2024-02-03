project(
    'burrow-gtk',
    ['rust'],
    version: '0.0.1',
    meson_version: '>= 1.0',
)

# Find Cargo
cargo_bin = find_program('cargo')
cargo_env = ['CARGO_HOME=' + meson.project_build_root()]
cargo_opt = ['--manifest-path', meson.project_source_root() / 'Cargo.toml']
cargo_opt += ['--target-dir', meson.project_build_root() / 'target']

# Config
prefix = get_option('prefix')
datadir = prefix / get_option('datadir')
localedir = prefix / get_option('localedir')

app_name = 'burrow-gtk'
app_name_capitalized = 'Burrow'
base_id = 'com.hackclub.burrow'
app_idpath = '/com/hackclub/' + app_name + '/'
if get_option('buildtype') == 'release'
  cargo_opt += ['--release']
  rust_target = 'release'
  app_id = base_id
else
  rust_target = 'debug'
  app_id = base_id + '-' + 'devel'
endif

# Imports
i18n = import('i18n')
gnome = import('gnome')

# External Dependencies
dependency('gtk4', version: '>= 4.0')
dependency('libadwaita-1', version: '>= 1.2')

glib_compile_resources = find_program('glib-compile-resources', required: true)
glib_compile_schemas = find_program('glib-compile-schemas', required: true)
desktop_file_validate = find_program('desktop-file-validate', required: false)
appstream_util = find_program('appstream-util', required: false)
fc_cache = find_program('fc-cache', required: false)

# Our Sources
subdir('po')
subdir('data')
subdir('src')

# Gnome Post Install
gnome.post_install(
    glib_compile_schemas: true,
    gtk_update_icon_cache: true,
    update_desktop_database: true,
)
