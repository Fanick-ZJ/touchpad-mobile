use leptos::prelude::*;
use thaw::*;
use thaw::{Button, ButtonAppearance};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <ConfigProvider>
            <Flex vertical=true class="text-white font-mono min-h-screen w-screen">
                <Flex justify=FlexJustify::Center align=FlexAlign::Center class="h-screen w-screen">
                    <Button appearance=ButtonAppearance::Primary>
                        "Primary"
                    </Button>
                </Flex>
            </Flex>
        </ConfigProvider>
    }
}
