use std::sync::OnceLock;

use crate::app::components::discover_page::DiscoverPage;
use crate::app::utils::set_language;

use super::command::{get_language, start_discover_service};
use super::components::navigation::Navigation;
use crate::app::types::DiscoverDevice;
use futures::StreamExt;
use leptos::leptos_dom::logging::console_log;
use leptos::task::spawn_local_scoped;
use leptos::{prelude::*, task::spawn_local};
use leptos_router::{
    components::{Redirect, Route, Router, Routes},
    path,
};
use reactive_stores::{Field, Patch, Store, StoreField};
use tauri_sys::event::listen;
use thaw::*;

#[derive(Clone, Debug, Default, Store)]
struct GlobalState {
    devices: Vec<DiscoverDevice>,
    current_device: Option<DiscoverDevice>,
}

pub static LANGUAGE: OnceLock<(ReadSignal<String>, WriteSignal<String>)> = OnceLock::new();

async fn listen_emit() {
    let events = listen::<DiscoverDevice>("found_device").await;
    if let Ok(mut events) = events {
        let state = expect_context::<Store<GlobalState>>();
        while let Some(event) = events.next().await {
            console_log(&format!("Got event: {:?}", event));
            state.devices().writer().unwrap().push(event.payload);
            console_log(&format!("state: {:?}", *state.devices().reader().unwrap()));
        }
    }
}

fn before_load() {
    spawn_local(async move { start_discover_service().await });
    spawn_local(async move {
        let language = get_language().await;
        set_language(&language);
    });
    // 这个函数会在异步运行环境中保存当前的Reactive环境
    spawn_local_scoped(async move {
        listen_emit().await;
    });
}

#[component]
pub fn RoutePages() -> impl IntoView {
    let lang_signal = signal("en-US".to_string());
    LANGUAGE.set(lang_signal).unwrap();
    before_load();
    view! {
        <Routes fallback= ||view! { <h1>"404"</h1> }>
            <Route path=path!("/") view=|| view! {
                <Redirect path="/discover"/>
            }/>
            <Route path=path!("/discover") view= DiscoverPage/>
            <Route path=path!("/control") view=|| view! {
                <h3>"control"</h3>
            }/>
            <Route path=path!("/settings") view=|| view! {
                <h3>"settings"</h3>
            }/>
        </Routes>
    }
}

#[component]
pub fn Main() -> impl IntoView {
    provide_context(Store::new(GlobalState::default()));
    view! {
        <ConfigProvider>
            <Suspense>
                <Flex vertical=true class="font-mono max-h-screen max-w-screen min-h-screen min-w-screen select-none">
                    <Router>
                        <RoutePages/>
                        <Flex justify=FlexJustify::Center align=FlexAlign::End class="h-screen w-screen">
                            <Navigation></Navigation>
                        </Flex>
                    </Router>
                </Flex>
            </Suspense>
        </ConfigProvider>
    }
}
