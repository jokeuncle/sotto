use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};

type Id = *mut Object;

// 让 app 不出现在 Dock 和 Cmd+Tab 切换器里
pub fn set_accessory_policy() {
    unsafe {
        let cls = class!(NSApplication);
        let app: Id = msg_send![cls, sharedApplication];
        // NSApplicationActivationPolicyAccessory = 1
        let _: () = msg_send![app, setActivationPolicy: 1_i64];
    }
}

// 把窗口压到“普通窗口之下 1 级”——所有正常应用窗口都会盖住它，
// 但仍高于 Finder 的桌面/图标层，所以鼠标事件能到 webview 这里。
// 注：之前用过 kCGDesktopIconWindowLevel(=-2147483603)，确实“最低”，
// 但代价是所有点击都会被 Finder 桌面拦截，刷新按钮没法点。
pub fn pin_below_normal(window: &tauri::WebviewWindow) {
    let handle = match window.ns_window() {
        Ok(h) => h,
        Err(_) => return,
    };
    let ns_window = handle as Id;
    if ns_window.is_null() {
        return;
    }
    unsafe {
        // NSNormalWindowLevel = 0；设为 -1 让它被所有普通窗口自然遮挡。
        let _: () = msg_send![ns_window, setLevel: -1_i64];

        // NSWindowCollectionBehavior：
        //   canJoinAllSpaces  = 1 << 0  = 1   (在所有桌面 Spaces 都可见)
        //   stationary        = 1 << 4  = 16  (切换 Spaces 时不滑动)
        //   ignoresCycle      = 1 << 6  = 64  (不参与 Cmd+` 窗口循环)
        let behavior: u64 = 1 | 16 | 64;
        let _: () = msg_send![ns_window, setCollectionBehavior: behavior];
    }
}
