/// Native DWM backdrops operate on this application's own window only.
pub struct NativeMaterial {
    #[cfg(windows)]
    hwnd: isize,
    pub active: bool,
    applied: Option<bool>,
}

impl NativeMaterial {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(windows)]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let hwnd = cc
                .window_handle()
                .ok()
                .and_then(|h| match h.as_raw() {
                    RawWindowHandle::Win32(w) => Some(w.hwnd.get()),
                    _ => None,
                })
                .unwrap_or(0);
            Self {
                hwnd,
                active: false,
                applied: None,
            }
        }
        #[cfg(not(windows))]
        {
            let _ = cc;
            Self {
                active: false,
                applied: None,
            }
        }
    }
    #[cfg(test)]
    pub fn disabled() -> Self {
        Self {
            #[cfg(windows)]
            hwnd: 0,
            active: false,
            applied: None,
        }
    }
    pub fn apply(&mut self, dark: bool) {
        if self.applied == Some(dark) {
            return;
        }
        self.applied = Some(dark);
        #[cfg(windows)]
        if self.hwnd != 0 {
            use windows_sys::Win32::{
                Foundation::HWND,
                Graphics::Dwm::{DwmExtendFrameIntoClientArea, DwmSetWindowAttribute},
                UI::Controls::MARGINS,
            };
            let hwnd = self.hwnd as HWND;
            let dark_flag = i32::from(dark);
            // Keep the system backdrop stable while the in-app modal animates.
            let backdrop: i32 = 2;
            let corners: i32 = 2;
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            // All pointers refer to live stack values of the documented size.
            unsafe {
                DwmSetWindowAttribute(hwnd, 20, (&dark_flag as *const i32).cast(), 4);
                DwmSetWindowAttribute(hwnd, 33, (&corners as *const i32).cast(), 4);
                let status = DwmSetWindowAttribute(hwnd, 38, (&backdrop as *const i32).cast(), 4);
                self.active = status >= 0 && DwmExtendFrameIntoClientArea(hwnd, &margins) >= 0;
                #[cfg(feature = "qa")]
                println!(
                    "DWM Mica dark={dark}: HRESULT={status:#x}, active={}",
                    self.active
                );
            }
        }
    }
}

pub fn system_dark() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{
            HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW,
        };
        let path: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
            .encode_utf16()
            .collect();
        let name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();
        let mut value = 1u32;
        let mut size = 4u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                path.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut value as *mut u32).cast(),
                &mut size,
            )
        };
        status == 0 && value == 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
