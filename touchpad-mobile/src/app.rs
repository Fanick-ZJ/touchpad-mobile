use leptos::logging;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;
use thaw::{Button, ButtonAppearance};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_without_args(cmd: &str) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    let on_click = move |_| {
        logging::log!("clicked");
        spawn_local(async move {
            invoke_without_args("start_discover_service").await;
        });
    };
    view! {
        <ConfigProvider>
            <Flex vertical=true class="text-white font-mono min-h-screen w-screen">
                <Flex justify=FlexJustify::Center align=FlexAlign::Center class="h-screen w-screen">
                    <Button appearance=ButtonAppearance::Primary on_click=on_click>
                        "Primary"
                    </Button>
                </Flex>
            </Flex>
        </ConfigProvider>
    }
}
