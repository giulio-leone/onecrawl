mod antibot;
mod captcha;
mod passkey;
mod stealth;

pub use antibot::{antibot_inject, antibot_test, antibot_profiles};
pub use captcha::{captcha_detect, captcha_wait, captcha_screenshot, captcha_inject, captcha_types, captcha_solve};
pub use passkey::{passkey_enable, passkey_add, passkey_list, passkey_log, passkey_disable, passkey_remove, passkey_register, passkey_set_file, passkey_vault_list, passkey_vault_save, passkey_vault_remove, passkey_vault_clear_site, passkey_vault_export, passkey_import_bitwarden, passkey_import_1password, passkey_import_cxf};
pub use stealth::{
    stealth_inject, stealth_check,
    stealth_tls_apply, stealth_webrtc_block, stealth_battery_spoof,
    stealth_sensor_block, stealth_canvas_advanced, stealth_timezone_sync,
    stealth_font_protect, stealth_behavior_sim, stealth_behavior_stop,
    stealth_rotate, stealth_detection_audit,
};
