pub use winit;
use crate::base::world;
use crate::base::world::{Backend, InstanceCreateInfo};

pub struct App {
    pub window: winit::Window,
    pub backend: world::Backend,
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
    pub width: f32,
    pub height: f32,
}

pub fn create_app(ci: &AppCreateInfo) -> App {
    let window = winit::WindowBuilder::new()
        .with_title(&ci.title)
        .with_dimensions(
            winit::dpi::LogicalSize::new(
                ci.width as f64,
                ci.height as f64
            )
        )
        .build(&ci.events_loop)
        .unwrap();
    let inst_base_ci = InstanceCreateInfo {
        app_name: ci.app_name.clone(),
        window_height: ci.height,
        window_width: ci.width,
    };
    let backend = Backend::new(&inst_base_ci, &window);

    App {
        window,
        backend,
    }
}

/// output method

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
