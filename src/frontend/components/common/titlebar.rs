use dioxus::prelude::*;

#[component]
pub fn TitleBar() -> Element {
    rsx! {
        div {
            class: "custom-titlebar",
            onmousedown: move |_event| {
                spawn(async move {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let window = dioxus_desktop::window();
                        let _ = window.drag();
                    }
                });
            }
        }

        div {
            class: "window-controls window-controls-windows",
            button {
                class: "window-control-btn minimize-btn-windows",
                title: "Minimize",
                onclick: move |_| {
                    spawn(async move {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let window = dioxus_desktop::window();
                            let _ = window.set_minimized(true);
                        }
                    });
                },
                span { class: "minimize-icon", "─" }
            }

            button {
                class: "window-control-btn close-btn-windows",
                title: "Close",
                onclick: move |_| {
                    spawn(async move {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let window = dioxus_desktop::window();
                            window.close();
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            std::process::exit(0);
                        }
                    });
                },
                span { class: "close-icon", "✕" }
            }
        }
    }
}
