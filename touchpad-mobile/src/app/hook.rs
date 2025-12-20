use leptos::{
    leptos_dom::logging::console_log,
    prelude::{RenderHtml, Signal},
};

use crate::app::main::LANGUAGE;

pub fn t(key: &str) -> Signal<String> {
    let key = key.to_string(); // 捕获 owned key

    // 派生信号：自动订阅 LANGUAGE 变化
    Signal::derive(move || {
        // 读取当前语言（会触发重新计算）
        let (_lang, _) = LANGUAGE.get().unwrap();
        let lang = _lang.to_html();
        // 执行翻译
        shared_utils::lang::translate::t(&key).to_string()
    })
}
