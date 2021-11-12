use modeling::{gui, state};
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use instant::Instant;

use std::path::PathBuf;
use structopt::StructOpt;

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Option<PathBuf>,
}

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
    #[cfg(not(target_arch = "wasm32"))]
    let opt = Opt::from_args();
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();
    let event_loop: EventLoop<gui::Event> = EventLoop::with_user_event();
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
        use winit::platform::web::WindowExtWebSys;
        let query_string = web_sys::window().unwrap().location().search().unwrap();
        let level: log::Level = parse_url_query_string(&query_string, "RUST_LOG")
            .map(|x| x.parse().ok())
            .flatten()
            .unwrap_or(log::Level::Error);
        console_log::init_with_level(level).expect("could not initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        use wasm_bindgen::{prelude::*, JsCast};
        wasm_bindgen_futures::spawn_local(async move {
            run(event_loop, window, wgpu::TextureFormat::Bgra8UnormSrgb).await;
        });
    }
}

#[cfg(target_arch = "wasm32")]
/// Parse the query string as returned by `web_sys::window()?.location().search()?` and get a
/// specific key out of it.
pub fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
    let query_string = query.strip_prefix('?')?;

    for pair in query_string.split('&') {
        let mut pair = pair.split('=');
        let key = pair.next()?;
        let value = pair.next()?;

        if key == search_key {
            return Some(value);
        }
    }

    None
}
