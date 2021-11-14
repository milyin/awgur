use std::sync::Once;

use windows::{
    runtime::{self, Handle, Interface},
    Foundation::Numerics::Vector2,
    Graphics::SizeInt32,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, RECT, WPARAM},
        System::{LibraryLoader::GetModuleHandleW, WinRT::ICompositorDesktopInterop},
        UI::WindowsAndMessaging::{
            AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, GetClientRect, LoadCursorW,
            PostQuitMessage, RegisterClassW, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT,
            GWLP_USERDATA, HMENU, IDC_ARROW, SW_SHOW, WINDOW_LONG_PTR_INDEX, WM_DESTROY,
            WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_NCCREATE, WM_RBUTTONDOWN, WM_SIZE, WM_SIZING,
            WM_TIMER, WNDCLASSW, WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
        },
    },
    UI::Composition::{Compositor, ContainerVisual, Desktop::DesktopWindowTarget},
};

use crate::{
    event::{MouseLeftPressed, MouseLeftPressedFocused, SendSlotEvent, SlotSize},
    gui::{SlotKeeper, SlotTag},
    window::wide_string::ToWide,
};

static REGISTER_WINDOW_CLASS: Once = Once::new();
static WINDOW_CLASS_NAME: &str = "awgur.Window";

pub struct Window {
    handle: HWND,
    target: Option<DesktopWindowTarget>,
    mouse_pos: Vector2,
    compositor: Compositor,
    root_visual:ContainerVisual,
    kslot: SlotKeeper,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> crate::Result<Box<Self>> {
        let class_name = WINDOW_CLASS_NAME.to_wide();
        let instance = unsafe { GetModuleHandleW(PWSTR(std::ptr::null_mut())).ok()? };
        REGISTER_WINDOW_CLASS.call_once(|| {
            let class = WNDCLASSW {
                hCursor: unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).ok().unwrap() },
                hInstance: instance,
                lpszClassName: class_name.as_pwstr(),
                lpfnWndProc: Some(Self::wnd_proc),
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });

        let width = width as i32;
        let height = height as i32;
        let window_ex_style = WS_EX_NOREDIRECTIONBITMAP;
        let window_style = WS_OVERLAPPEDWINDOW;

        let (adjusted_width, adjusted_height) = {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            unsafe {
                AdjustWindowRectEx(&mut rect, window_style, false, window_ex_style).ok()?;
            }
            (rect.right - rect.left, rect.bottom - rect.top)
        };
        let compositor = Compositor::new()?;

        let mouse_pos = Vector2::default();
        let root_visual = compositor.CreateContainerVisual()?;
        root_visual.SetSize(Vector2 { X: width as f32, Y: height as f32 })?;
        let kslot = SlotKeeper::new(root_visual.clone())?;
        let mut result = Box::new(Self {
            handle: HWND(0),
            target: None,
            mouse_pos,
            compositor,
            root_visual,
            kslot,
        });

        let title = title.to_wide();
        let window = unsafe {
            CreateWindowExW(
                window_ex_style,
                class_name.as_pwstr(),
                title.as_pwstr(),
                window_style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                adjusted_width,
                adjusted_height,
                HWND(0),
                HMENU(0),
                instance,
                result.as_mut() as *mut _ as _,
            )
            .ok()?
        };

        let compositor_desktop: ICompositorDesktopInterop = result.compositor().cast()?;
        let target = unsafe { compositor_desktop.CreateDesktopWindowTarget(result.handle(), true)? };
        target.SetRoot(result.root_visual())?;
        result.target = Some(target);

        unsafe { ShowWindow(&window, SW_SHOW) };
        Ok(result)
    }

    pub fn size(&self) -> crate::Result<SizeInt32> {
        Ok(get_window_size(self.handle)?)
    }

    pub fn handle(&self) -> HWND {
        self.handle
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match message {
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                let (x, y) = get_mouse_position(lparam);
                let point = Vector2 {
                    X: x as f32,
                    Y: y as f32,
                };
                self.mouse_pos = point;
                // self.game.on_pointer_moved(&point).unwrap();
            }
            WM_SIZE | WM_SIZING => {
                let new_size = self.size().unwrap();
                let new_size = Vector2 {
                    X: new_size.Width as f32,
                    Y: new_size.Height as f32,
                };
                self.kslot.send_size(SlotSize(new_size)).unwrap();
            }
            WM_LBUTTONDOWN => {
                self.kslot
                    .send_mouse_left_pressed(MouseLeftPressed(self.mouse_pos))
                    .unwrap();
                self.kslot
                    .send_mouse_left_pressed_focused(MouseLeftPressedFocused(self.mouse_pos))
                    .unwrap();
            }
            WM_RBUTTONDOWN => {
                // self.game.on_pointer_pressed(true, false).unwrap();
            }
            WM_TIMER => {
                // dbg!("timer");
            }
            _ => {}
        }
        // self.pool.run_until_stalled();
        unsafe { DefWindowProcW(self.handle, message, wparam, lparam) }
    }

    unsafe extern "system" fn wnd_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_NCCREATE {
            let cs = lparam.0 as *const CREATESTRUCTW;
            let this = (*cs).lpCreateParams as *mut Self;
            (*this).handle = window;

            SetWindowLong(window, GWLP_USERDATA, this as _);
        } else {
            let this = GetWindowLong(window, GWLP_USERDATA) as *mut Self;

            if let Some(this) = this.as_mut() {
                return this.message_handler(message, wparam, lparam);
            }
        }
        DefWindowProcW(window, message, wparam, lparam)
    }

    /// Get a reference to the window's compositor.
    pub fn compositor(&self) -> &Compositor {
        &self.compositor
    }

    /// Get a reference to the window's root visual.
    pub fn root_visual(&self) -> &ContainerVisual {
        &self.root_visual
    }

    pub fn slot(&self) -> SlotTag {
        self.kslot.tag()
    }
}

fn get_window_size(window_handle: HWND) -> runtime::Result<SizeInt32> {
    unsafe {
        let mut rect = RECT::default();
        let _ = GetClientRect(window_handle, &mut rect).ok()?;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        Ok(SizeInt32 {
            Width: width,
            Height: height,
        })
    }
}

fn get_mouse_position(lparam: LPARAM) -> (isize, isize) {
    let x = lparam.0 & 0xffff;
    let y = (lparam.0 >> 16) & 0xffff;
    (x, y)
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
unsafe fn SetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::SetWindowLongW;

    SetWindowLongW(window, index, value as _) as _
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
unsafe fn SetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;

    SetWindowLongPtrW(window, index, value)
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
unsafe fn GetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::SetWindowLongW;

    GetWindowLongW(window, index) as _
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
unsafe fn GetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;

    GetWindowLongPtrW(window, index)
}
