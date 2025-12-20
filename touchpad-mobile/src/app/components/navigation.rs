use crate::app::{hook::t, utils::set_language};
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_location};
use thaw::*;

#[component]
pub fn NavItem(
    label: Signal<String>,
    unreach_icon: icondata_core::Icon,
    reach_icon: icondata_core::Icon,
    to: &'static str,
) -> impl IntoView {
    let location = use_location();
    let (current_icon, set_current_icon) = signal::<icondata::Icon>(unreach_icon);
    Effect::new(move |_| {
        if location.pathname.get().starts_with(to) {
            *set_current_icon.write() = reach_icon;
        } else {
            *set_current_icon.write() = unreach_icon;
        }
    });

    let on_click = move |_| {
        set_language("zh-CN");
    };
    view! {
            <A href=to>
                <div aria-label=label class="flex-1 flex flex-col justify-center items-center">
                    <Icon icon=current_icon  on_click=on_click/>
                    <span class="font-sans text-xs">{label}</span>
                </div>
            </A>
    }
}

#[component]
pub fn Navigation() -> impl IntoView {
    view! {
        <nav class="w-screen flex justify-around py-4 border-t border-gray-200" aria-label="Main Navigation">
            <NavItem
                label=t("discover")
                unreach_icon=icondata::BsSearchHeart
                reach_icon=icondata::BsSearchHeartFill
                to="/discover"
            />
            <NavItem
                label=t("control")
                unreach_icon=icondata::BsHandIndex
                reach_icon=icondata::BsHandIndexFill
                to="/control"
            />
            <NavItem
                label=t("settings")
                unreach_icon=icondata::BsGear
                reach_icon=icondata::BsGearFill
                to="/settings"
            />
        </nav>
    }
}
