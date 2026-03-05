mod core;
mod keyboard;
mod mouse;
mod routing;
mod scroll_mouse;
mod settings;
mod state;
mod visual;
mod window;

pub use core::{click, dblclick, type_text, fill, focus, hover, scroll_into_view, check, uncheck, select_option, tap, drag, upload, bounding_box, press_key, key_down, key_up};
pub use keyboard::{keyboard_shortcut, keyboard_type, keyboard_insert_text};
pub use mouse::{mouse_move, mouse_down, mouse_up, mouse_wheel};
pub use routing::{route_add, route_remove, requests_list, close_page};
pub use scroll_mouse::{scroll};
pub use settings::{set_offline, set_extra_headers, set_credentials};
pub use state::{is_check};
pub use visual::{highlight, page_errors};
pub use window::{window_new};
