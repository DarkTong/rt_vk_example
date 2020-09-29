use ash::vk;
use rt_vk_example::*;

fn main()
{
    println!("current dir: {:?}", std::env::current_dir());
    let mut app_ci = app::AppCreateInfo {
        app_name: "triangle".to_string(),
        title: "triangle".to_string(),
        events_loop: winit::EventsLoop::new(),
        width: 1920.0,
        height: 1080.0,
    };
    let exp_app = app::create_app(&app_ci);
    let mut backend = exp_app.backend;

}