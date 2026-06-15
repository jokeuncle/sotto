use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};

type Id = *mut Object;

extern "C" {
    fn CGWindowLevelForKey(key: i32) -> i32;
}

// 让 app 不出现在 Dock 和 Cmd+Tab 切换器里
pub fn set_accessory_policy() {
    unsafe {
        let cls = class!(NSApplication);
        let app: Id = msg_send![cls, sharedApplication];
        // NSApplicationActivationPolicyAccessory = 1
        let _: () = msg_send![app, setActivationPolicy: 1_i64];
    }
}

// 把窗口压到桌面图标层级——在所有普通应用窗口之下、壁纸之上
pub fn pin_to_desktop_level(window: &tauri::WebviewWindow) {
    let handle = match window.ns_window() {
        Ok(h) => h,
        Err(_) => return,
    };
    let ns_window = handle as Id;
    if ns_window.is_null() {
        return;
    }
    unsafe {
        // kCGDesktopIconWindowLevelKey = 18
        // 这是 macOS 上能让窗口跟桌面图标同层的最低 level，仍在所有应用之下
        let level = CGWindowLevelForKey(18);
        let _: () = msg_send![ns_window, setLevel: level as i64];

        // NSWindowCollectionBehavior：
        //   canJoinAllSpaces  = 1 << 0  = 1   (在所有桌面 Spaces 都可见)
        //   stationary        = 1 << 4  = 16  (切换 Spaces 时不滑动)
        //   ignoresCycle      = 1 << 6  = 64  (不参与 Cmd+` 窗口循环)
        let behavior: u64 = 1 | 16 | 64;
        let _: () = msg_send![ns_window, setCollectionBehavior: behavior];
    }
}
