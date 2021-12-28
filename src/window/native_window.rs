use std::sync::Once;

use futures::channel::mpsc::Sender;
use windows::{
    core::{self, Interface},
    Graphics::SizeInt32,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, PWSTR, RECT, WPARAM},
        System::{LibraryLoader::GetModuleHandleW, WinRT::Composition::ICompositorDesktopInterop},
        UI::WindowsAndMessaging::{
            AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect,
            GetMessageW, LoadCursorW, PostQuitMessage, RegisterClassW, ShowWindow,
            TranslateMessage, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MSG, SW_SHOW,
            WINDOW_LONG_PTR_INDEX, WM_DESTROY, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_NCCREATE,
            WM_RBUTTONDOWN, WM_SIZE, WM_SIZING, WM_TIMER, WNDCLASSW, WS_EX_NOREDIRECTIONBITMAP,
            WS_OVERLAPPEDWINDOW,
        },
    },
    UI::Composition::{Compositor, ContainerVisual, Desktop::DesktopWindowTarget},
};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, ModifiersState, MouseButton, WindowEvent},
};

use crate::window::wide_string::ToWide;

static REGISTER_WINDOW_CLASS: Once = Once::new();
static WINDOW_CLASS_NAME: &str = "wag.Window";

pub struct Window {
    handle: HWND,
    title: &'static str,
    target: Option<DesktopWindowTarget>,
    compositor: Compositor,
    root_visual: ContainerVisual,
    event_channel: Sender<WindowEvent<'static>>,
}

impl Window {
    pub fn new(
        compositor: Compositor,
        title: &'static str,
        root_visual: ContainerVisual,
        event_channel: Sender<WindowEvent<'static>>,
    ) -> Self {
        Self {
            handle: 0,
            title,
            target: None,
            compositor,
            root_visual,
            event_channel,
        }
    }

    pub fn open(self) -> crate::Result<Box<Self>> {
        let class_name = WINDOW_CLASS_NAME.to_wide();
        let instance = unsafe { GetModuleHandleW(PWSTR(std::ptr::null_mut())) };
        REGISTER_WINDOW_CLASS.call_once(|| {
            let class = WNDCLASSW {
                hCursor: unsafe { LoadCursorW(0, IDC_ARROW) },
                hInstance: instance,
                lpszClassName: class_name.as_pwstr(),
                lpfnWndProc: Some(Self::wnd_proc),
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });

        let size = self.root_visual.Size()?;
        let width = size.X as i32;
        let height = size.Y as i32;
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

        let title = self.title.to_wide();
        let mut result = Box::new(self); // TODO: use pin?
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
                0,
                0,
                instance,
                result.as_mut() as *mut _ as _,
            )
        };

        let compositor_desktop: ICompositorDesktopInterop = result.compositor.cast()?;
        let target =
            unsafe { compositor_desktop.CreateDesktopWindowTarget(result.handle(), true)? };
        target.SetRoot(result.root_visual.clone())?;
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
                return 0;
            }
            WM_MOUSEMOVE => {
                let (x, y) = get_mouse_position(lparam);
                let _ = self.event_channel.try_send(WindowEvent::CursorMoved {
                    device_id: unsafe { DeviceId::dummy() },
                    position: PhysicalPosition {
                        x: x as f64,
                        y: y as f64,
                    },
                    modifiers: ModifiersState::default(),
                });
            }
            WM_SIZE | WM_SIZING => {
                let size = self.size().unwrap();
                let _ = self
                    .event_channel
                    .try_send(WindowEvent::Resized((size.Width, size.Height).into()));
            }
            WM_LBUTTONDOWN => {
                let _ = self.event_channel.try_send(WindowEvent::MouseInput {
                    device_id: unsafe { DeviceId::dummy() },
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    modifiers: ModifiersState::default(),
                });
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
            let cs = lparam as *const CREATESTRUCTW;
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
}

pub fn run_message_loop() {
    let mut message = MSG::default();
    unsafe {
        // const IDT_TIMER1: usize = 1;
        // SetTimer(window.handle(), IDT_TIMER1, 10, None);
        while GetMessageW(&mut message, 0, 0, 0).into() {
            TranslateMessage(&mut message);
            DispatchMessageW(&mut message);
        }
    }
}

fn get_window_size(window_handle: HWND) -> core::Result<SizeInt32> {
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
    let x = lparam & 0xffff;
    let y = (lparam >> 16) & 0xffff;
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
