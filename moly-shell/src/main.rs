mod app;

/// Sets the macOS Dock icon and menubar app name.
/// Needed when running via `cargo run` since the binary isn't inside
/// the .app bundle and macOS can't read Info.plist.
#[cfg(target_os = "macos")]
fn set_dock_icon_and_name() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(200));

        let icon_path = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/AppIcon.icns");

        unsafe {
            use objc::runtime::{Class, Object};
            use objc::{msg_send, sel, sel_impl};

            let ns_string_cls = Class::get("NSString").unwrap();

            let path_bytes = std::ffi::CString::new(icon_path).unwrap();
            let ns_path: *mut Object = msg_send![
                ns_string_cls,
                stringWithUTF8String: path_bytes.as_ptr()
            ];

            let ns_image_cls = Class::get("NSImage").unwrap();
            let image: *mut Object = msg_send![ns_image_cls, alloc];
            let image: *mut Object = msg_send![image, initWithContentsOfFile: ns_path];

            let app_cls = Class::get("NSApplication").unwrap();
            let app: *mut Object = msg_send![app_cls, sharedApplication];

            if !image.is_null() {
                let _: () = msg_send![app, setApplicationIconImage: image];
            }

            let name_bytes = std::ffi::CString::new("Moxin Studio").unwrap();
            let ns_name: *mut Object = msg_send![
                ns_string_cls,
                stringWithUTF8String: name_bytes.as_ptr()
            ];
            let main_menu: *mut Object = msg_send![app, mainMenu];
            if !main_menu.is_null() {
                let first_item: *mut Object = msg_send![main_menu, itemAtIndex: 0i64];
                if !first_item.is_null() {
                    let submenu: *mut Object = msg_send![first_item, submenu];
                    if !submenu.is_null() {
                        let _: () = msg_send![submenu, setTitle: ns_name];
                    }
                    let _: () = msg_send![first_item, setTitle: ns_name];
                }
            }

            let process_info_cls = Class::get("NSProcessInfo").unwrap();
            let process_info: *mut Object = msg_send![process_info_cls, processInfo];
            let _: () = msg_send![process_info, setProcessName: ns_name];
        }
    });
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Set working directory to the executable's directory
        // This is critical for macOS app bundles to find resources in Contents/Resources/
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                let _ = std::env::set_current_dir(exe_dir);
            }
        }
    }

    // Initialize the logger
    env_logger::init();
    log::info!("Starting Moly");

    // Install panic hook that appends ALL panics to /tmp/studio_panic.log
    use std::io::Write;
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("=== PANIC ===\n{}\n\nBacktrace:\n{:?}\n\n", info, std::backtrace::Backtrace::force_capture());
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/studio_panic.log") {
            let _ = f.write_all(msg.as_bytes());
        }
        eprintln!("PANIC: {}", msg);
    }));

    // Set process name early so the Dock shows "Moxin Studio" from the start
    #[cfg(target_os = "macos")]
    unsafe {
        use objc::runtime::Class;
        use objc::{msg_send, sel, sel_impl};
        let ns_string_cls = Class::get("NSString").unwrap();
        let name_bytes = std::ffi::CString::new("Moxin Studio").unwrap();
        let ns_name: *mut objc::runtime::Object = msg_send![
            ns_string_cls, stringWithUTF8String: name_bytes.as_ptr()
        ];
        let process_info_cls = Class::get("NSProcessInfo").unwrap();
        let process_info: *mut objc::runtime::Object = msg_send![process_info_cls, processInfo];
        let _: () = msg_send![process_info, setProcessName: ns_name];
    }

    // Set Dock icon and menubar title after the run loop starts (needed for cargo run)
    #[cfg(target_os = "macos")]
    set_dock_icon_and_name();

    // macOS 26 requires setActivationPolicy to be called before the event loop
    // starts, otherwise NSAssertMainEventQueueIsCurrentEventQueue fires on the
    // first nextEventMatchingMask call.
    #[cfg(target_os = "macos")]
    unsafe {
        use objc::runtime::Class;
        use objc::{msg_send, sel, sel_impl};
        if let Some(ns_app_cls) = Class::get("NSApplication") {
            let ns_app: *mut objc::runtime::Object = msg_send![ns_app_cls, sharedApplication];
            let () = msg_send![ns_app, setActivationPolicy: 0i64]; // NSApplicationActivationPolicyRegular
        }
    }

    // Register atexit handler to kill ominix-api on exit.
    // On macOS, [NSApplication terminate:] calls exit() directly, so app_main()
    // never returns — atexit is the only reliable cleanup hook.
    #[cfg(unix)]
    unsafe {
        libc::atexit(cleanup_on_exit);
        libc::signal(libc::SIGINT, sigint_handler as libc::sighandler_t);
        libc::signal(libc::SIGTERM, sigint_handler as libc::sighandler_t);
    }

    app::app_main();

    // Belt-and-suspenders: also call here in case app_main() does return.
    moly_data::kill_server_process();
}

#[cfg(unix)]
extern "C" fn cleanup_on_exit() {
    moly_data::kill_server_process();
}

#[cfg(unix)]
extern "C" fn sigint_handler(_sig: libc::c_int) {
    moly_data::kill_server_process();
    std::process::exit(0);
}
