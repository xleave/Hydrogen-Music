fn main() {
    #[cfg(target_os = "linux")]
    {
        // 禁用 WebKitGTK 的 DMABUF 渲染器，解决 Wayland/niri 下白屏或 EGL 错误
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        // 强制 GDK 使用 Wayland 后端而不是回退到 XWayland
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            std::env::set_var("GDK_BACKEND", "wayland");
        }
    }
    hydrogen_music_lib::run();
}
