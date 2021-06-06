use modeling::{gui, state};
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use std::time::Instant;

async fn run(
    event_loop: EventLoop<gui::Event>,
    window: Window,
    swapchain_format: wgpu::TextureFormat,
) {
    let mut state = state::State::new(&window, swapchain_format, &event_loop).await;

    let start_time = Instant::now();
    let mut previous_frame_time = None;
    let mut last_update_inst = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        state.gui.handle_event(&event);
        state.handle_event(
            &event,
            control_flow,
            &window,
            start_time,
            &mut last_update_inst,
            &mut previous_frame_time,
        );
    });
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::with_user_event();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        //wgpu_subscriber::initialize_default_subscriber(None);
        // Temporarily avoid srgb formats for the swapchain on the web

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            //run(event_loop, window, wgpu::TextureFormat::Bgra8UnormSrgb).await;
            run(event_loop, window, wgpu::TextureFormat::Bgra8UnormSrgb).await;
        })
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        //use log::Level;
        //console_log::init_with_level(Level::Trace).expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window, wgpu::TextureFormat::Bgra8Unorm));
    }
}
