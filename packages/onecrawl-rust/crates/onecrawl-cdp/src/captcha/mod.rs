mod types;
mod detect;
mod solve;
mod stealth_check;
#[cfg(test)]
mod tests;

pub use types::{CaptchaDetection, CaptchaConfig, CaptchaResult};
pub use detect::{detect_captcha, wait_for_captcha, screenshot_captcha, inject_solution};
pub use solve::{solve_turnstile_native, solve_recaptcha_audio};
pub use stealth_check::{stealth_check, supported_types};
