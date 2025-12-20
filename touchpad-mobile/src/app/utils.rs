use leptos::prelude::{ReadSignal, Write};

use crate::app::main::LANGUAGE;
use shared_utils::lang::translate::set_current_language;

pub fn set_language(lang: &str) {
    set_current_language(lang.to_string());
    let (_, set_lang) = LANGUAGE.get().unwrap();
    *set_lang.write() = lang.to_string();
}

pub fn get_language() -> &'static ReadSignal<String> {
    let (lang, _) = LANGUAGE.get().unwrap();
    lang
}
