pub use winit;
use crate::base::world;
use crate::base::world::{InstanceBase, InstanceCreateInfo};

pub struct App {
    pub window: winit::Window,
    pub instance_base: world::InstanceBase,
}

pub struct AppConfig {
    pub win_width: f32,
    pub win_height: f32,
}

pub struct AppCreateInfo
{
    pub app_name: String,
    pub title: String,
    pub events_loop: winit::EventsLoop,
    pub width: f64,
    pub height: f64,
}

static mut s_app: Option<Box<App>> = None;
static s_app_config: AppConfig = AppConfig {
    win_width: 800.0,
    win_height: 600.0,
};

fn create_app(ci: &AppCreateInfo) -> Box<App> {
    let window = winit::WindowBuilder::new()
        .with_title(&ci.title)
        .with_dimensions(
            winit::dpi::LogicalSize::new(
                ci.width, ci.height
            )
        )
        .build(&ci.events_loop)
        .unwrap();
    let inst_base_ci = InstanceCreateInfo {app_name: ci.app_name.clone()};
    let instance_base = InstanceBase::new(&inst_base_ci);

    Box::new(App {
        window: window,
        instance_base: instance_base,
    })
}

/// output method

pub fn get_app() -> &'static Box<App> {
    unsafe {
        &s_app.unwrap()
    }
}

pub fn reset_app(ci: &AppCreateInfo)
{
    let app = create_app(ci);
    unsafe {
        s_app.replace(app).unwrap();
    }
}


pub fn get_app_config() -> &'static AppConfig { &s_app_config }

pub fn render_loop<F: Fn()>(event_loop: &mut winit::EventsLoop, render:F) {
    pub use winit::*;
    event_loop.run_forever(|event| {
        // do render action
        render();

        // solve window event
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                        ControlFlow::Break
                    } else {
                        ControlFlow::Continue
                    }
                },
                WindowEvent::CloseRequested => ControlFlow::Break,
                _ => ControlFlow::Continue,
            },
            _ => ControlFlow::Continue,
        }
    })
}
