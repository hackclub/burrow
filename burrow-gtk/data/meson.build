# app.desktop.in.in
desktop_conf = configuration_data()
desktop_conf.set('APP_ID', app_id)
desktop_conf.set('APP_NAME', app_name)
desktop_conf.set('APP_NAME_CAPITALIZED', app_name_capitalized)

desktop_file_in = configure_file(
    input: 'app.desktop.in.in',
    output: '@BASENAME@',
    configuration: desktop_conf,
)

desktop_file = i18n.merge_file(
    input: desktop_file_in,
    output: app_id + '.desktop',
    type: 'desktop',
    po_dir: '../po',
    install: true,
    install_dir: datadir / 'applications',
)

if desktop_file_validate.found()
    test(
        'validate-desktop',
        desktop_file_validate,
        args: [desktop_file],
    )
endif

# app.gschema.xml.in
gschema_conf = configuration_data()
gschema_conf.set('APP_ID', app_id)
gschema_conf.set('APP_NAME', app_name)
gschema_conf.set('APP_IDPATH', app_idpath)
gschema_file = configure_file(
    input: 'app.gschema.xml.in',
    output: app_id + '.gschema.xml',
    configuration: gschema_conf,
    install: true,
    install_dir: datadir / 'glib-2.0' / 'schemas',
)

if glib_compile_schemas.found()
    test(
        'validate-gschema',
        glib_compile_schemas,
        args: [
            '--dry-run',
            datadir / 'glib-2.0' / 'schemas',
        ],
    )
endif

# app.metainfo.xml.in
appdata_conf = configuration_data()
appdata_conf.set('APP_ID', app_id)
appdata_conf.set('APP_NAME', app_name)
appdata_conf.set('APP_NAME_CAPITALIZED', app_name_capitalized)
appdata_file_in = configure_file(
    input: 'app.metainfo.xml.in',
    output: '@BASENAME@',
    configuration: appdata_conf,
)
appdata_file = i18n.merge_file(
    input: appdata_file_in,
    output: app_id + '.metainfo.xml',
    po_dir: '../po',
    install: true,
    install_dir: datadir / 'metainfo',
)

if appstream_util.found()
    test(
        'validate-appdata',
        appstream_util,
        args: ['validate', '--nonet', appdata_file],
    )
endif

install_data(
    'icons/hicolor/scalable/apps/' + app_name + '.svg',
    install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps',
    rename: app_id + '.svg',
)

install_data(
    'icons/hicolor/symbolic/apps/' + app_name + '-symbolic.svg',
    install_dir: datadir / 'icons' / 'hicolor' / 'symbolic' / 'apps',
    rename: app_id + '-symbolic.svg',
)
