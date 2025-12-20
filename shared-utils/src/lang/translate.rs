use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};
use sys_locale::get_locale;

pub static CURRENT_LANGUAGE: OnceLock<RwLock<String>> = OnceLock::new();

// 改为存储 Owned Strings，避免生命周期问题
static LANGUAGE_MAP: OnceLock<RwLock<HashMap<&str, &str>>> = OnceLock::new();

fn load_language_map() -> HashMap<&'static str, &'static str> {
    let lang = get_current_language();
    let lang_assets = match lang.as_str() {
        "en-US" => include_str!("assets/en-US.txt"),
        "zh-CN" => include_str!("assets/zh-CN.txt"),
        _ => include_str!("assets/en-US.txt"), // 默认回退
    };

    lang_assets
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].trim(), parts[1].trim()))
            } else {
                None
            }
        })
        .collect()
}

pub fn get_current_language() -> String {
    CURRENT_LANGUAGE
        .get_or_init(|| RwLock::new(get_locale().unwrap_or_else(|| "en-US".to_string())))
        .read()
        .unwrap()
        .clone()
}

pub fn set_current_language(language: String) {
    *CURRENT_LANGUAGE
        .get_or_init(|| RwLock::new(get_locale().unwrap_or_else(|| "en-US".to_string())))
        .write()
        .unwrap() = language.clone();

    // 重要：语言改变后重新加载翻译
    reload_translations();
}

/// 重新加载当前语言的翻译映射
pub fn reload_translations() {
    let new_map = load_language_map();
    *LANGUAGE_MAP
        .get_or_init(|| RwLock::new(load_language_map()))
        .write()
        .unwrap() = new_map;
}

/// 翻译函数 - 返回 String 而非 &str
pub fn t(key: &str) -> &str {
    LANGUAGE_MAP
        .get_or_init(|| RwLock::new(load_language_map()))
        .read()
        .unwrap()
        .get(key)
        .cloned() // 返回 Owned String
        .unwrap_or_else(|| key)
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    #[test]
    fn test_translation() {
        // 确保在测试时初始化
        set_current_language("en-US".to_string());

        // 测试翻译
        assert_eq!(t("discover"), "Discover");

        // 测试未找到的 key
        assert_eq!(t("nonexistent"), "nonexistent");
    }

    #[test]
    fn test_language_switch() {
        sleep(Duration::from_secs(1));
        set_current_language("zh-CN".to_string());
        assert_eq!(get_current_language(), "zh-CN");
        assert_eq!(t("discover"), "发现");
    }
}
