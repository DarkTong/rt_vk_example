pub use winit;
use ash::vk;
use ash::version::*;
use crate::base::ri;
use crate::base::surface;
use crate::base::buffer;
use std::time;
use std::boxed;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;



pub struct App {
    pub window: winit::Window,
    pub backend: RefCell<Rc<ri::Backend>>,
    pub surface: surface::Surface,
    pub buf_mgr_sys: buffer::BufferManagerSystem,
    // other
    pub cmd_pool: vk::CommandPool,
    pub present_complete: vk::Semaphore,
    pub render_complete: vk::Semaphore,
    pub render_loop_obj: boxed::Box<dyn RenderLoop>,
    // graphic queue info
    pub graphic_queue: vk::Queue,
    pub graphic_cmd_buffer: vk::CommandBuffer,
    // compute queue info
    pub compute_queue: vk::Queue,
    pub compute_cmd_buffer: vk::CommandBuffer,
    // transfer
    pub transfer_queue: vk::Queue,
    pub transfer_cmd_buffer: vk::CommandBuffer,
    pub graphic_submit_fence: vk::Fence,
    // property
    frame_count: Cell<u64>,
    frame_timer: Cell<f64>, //  一帧耗时(ms)
    timer_scale: Cell<f64>,

    events_loop: RefCell<winit::EventsLoop>,
}

#[derive(Default)]
pub struct DefaultRenderLoop{}

pub struct AppCreateInfo {
    pub app_name: String,
    pub title: String,
    pub width: f32,
    pub height: f32,
}

static VERTEX_BUFFER_SIZE: u64 = 4 * 1024 * 1024;
static INDEX_BUFFER_SIZE: u64 = 4 * 1024 * 1024;
static UNIFORM_BUFFER_SIZE: u64 = 1024 * 1024;


impl App {
    pub fn new(ci: &AppCreateInfo) -> Self
    {
        let events_loop = winit::EventsLoop::new();
        let window = {
            winit::WindowBuilder::new()
                .with_title(&ci.title)
                .with_dimensions(
                    winit::dpi::LogicalSize::new(
                        ci.width as f64,
                        ci.height as f64
                    )
                )
                .build(&events_loop)
                .unwrap()
        };
        let backend = Rc::new(ri::Backend::new(&window, 0));
        let surface = surface::Surface::new(backend.clone(), &window);
        let cmd_pool = unsafe {
            let pool_ci  = vk::CommandPoolCreateInfo {
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index: backend.queue_family_index.clone(),
                ..Default::default()
            };
            backend.device.create_command_pool(&pool_ci, None).unwrap()
        };
        let semaphore_ci = vk::SemaphoreCreateInfo::default();
        let present_complete = unsafe {
            backend.device
                .create_semaphore(&semaphore_ci, None)
                .unwrap()
        };
        let render_complete = unsafe {
            backend.device
                .create_semaphore(&semaphore_ci, None)
                .unwrap()
        };
        let render_loop_obj = boxed::Box::new(DefaultRenderLoop::default());
        let buf_mgr_sys = {
            buffer::BufferManagerSystem::new(
                &backend,
                VERTEX_BUFFER_SIZE,
                INDEX_BUFFER_SIZE,
                UNIFORM_BUFFER_SIZE,
            )
        };
        let graphic_queue = unsafe {
            backend.device.get_device_queue(backend.queue_family_index, 0)
        };
        let compute_queue = unsafe {
            backend.device.get_device_queue(backend.queue_family_index, 0)
        };
        let transfer_queue = unsafe {
            backend.device.get_device_queue(backend.queue_family_index, 0)
        };
        let (graphic_cmd_buffer,
            compute_cmd_buffer,
            transfer_cmd_buffer) = {
            let ci = vk::CommandBufferAllocateInfo {
                command_buffer_count: 3,
                command_pool: cmd_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                ..Default::default()
            };
            unsafe {
                let cmd_bufs = backend.device.allocate_command_buffers(&ci)
                    .unwrap();
                (cmd_bufs[0], cmd_bufs[1], cmd_bufs[2])
            }
        };
        let graphic_submit_fence = unsafe {
            backend.device.create_fence(
                &vk::FenceCreateInfo::default(), None)
                .unwrap()
        };

        App {
            window,
            backend: RefCell::new(backend),
            surface,
            cmd_pool,
            present_complete,
            render_complete,
            render_loop_obj,
            events_loop: RefCell::new(events_loop),
            buf_mgr_sys,
            graphic_queue,
            graphic_cmd_buffer,
            compute_queue,
            compute_cmd_buffer,
            transfer_queue,
            transfer_cmd_buffer,
            graphic_submit_fence,
            frame_count: Cell::new(0u64),
            frame_timer: Cell::new(0.0),
            timer_scale: Cell::new(1.0),
        }
    }

    fn window_resize(&self)
    {
    }
}


impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            let device = &self.backend.borrow().device;
            device.device_wait_idle();
            device.destroy_command_pool(self.cmd_pool, None);
            device.destroy_semaphore(self.present_complete, None);
            device.destroy_semaphore(self.render_complete, None);
            device.destroy_fence(self.graphic_submit_fence, None);
        }
    }
}


pub trait RenderLoopAction {
    fn acquire_next_image(& self) -> u32;
    fn render_loop(&self);
    fn render_frame(&self);
    fn submit_frame(&mut self);
}

pub trait RenderLoop {
    fn render(&self, app_obj: &App);
    fn update(&self, app_obj: &App, delta_time: f64);
}

impl RenderLoopAction for App {
    fn acquire_next_image(&self) -> u32
    {
        let ret = unsafe {
            self.surface.swapchain.acquire_next_image(
                self.surface.swapchain_khr,
                std::u64::MAX,
                self.present_complete,
                vk::Fence::null(),
            )
        };
        match ret {
            Err(err_code) => match err_code {
                vk::Result::ERROR_OUT_OF_DATE_KHR
                | vk::Result::SUBOPTIMAL_KHR => {
                    self.window_resize();
                    std::u32::MAX
                },
                _ => {
                    panic!("prepare_frame error {:?}", err_code);
                }
            },
            Ok((idx, _)) => idx
        }
    }

    fn render_loop(&self)
    {
        pub use winit::*;
        self.events_loop.borrow_mut().run_forever(|event| {
            // do render action
            self.render_frame();

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

    fn render_frame(&self)
    {
        let t_start = time::Instant::now().elapsed().as_micros();
        self.render_loop_obj.render(self);
        self.frame_count.set(self.frame_count.get() + 1u64);
        let t_end = time::Instant::now().elapsed().as_micros();
        let t_diff = (t_end - t_start) as f64;
        self.frame_timer.set(t_diff / 1000.0);
        self.render_loop_obj.update(self, self.frame_timer.take());
    }

    fn submit_frame(&mut self)
    {
        let present_info_khr = vk::PresentInfoKHR::builder()
            .swapchains(&[self.surface.swapchain_khr])
            .wait_semaphores(&[self.render_complete])

            .image_indices(&[0]) //  todo：应该设置有三个
            .build();
        unsafe {
            match self.surface.swapchain.queue_present(self.graphic_queue, &present_info_khr){
                Ok(_) => {
                    self.backend.borrow().device.queue_wait_idle(self.graphic_queue);
                },
                Err(err_code) => {
                    match err_code {
                        vk::Result::ERROR_OUT_OF_DATE_KHR => {
                            self.window_resize();
                        },
                        _ => panic!("submit frame error => {:?}", err_code)

                    }
                }
            }
        };
    }

}

impl RenderLoop for DefaultRenderLoop {
    fn render(&self, app_obj: &App)
    {
        // self.prepare_frame();
        // let submit_info = vk::SubmitInfo::builder()
        //     .command_buffers(&[self.graphic_cmd_buffer])
        //     .build();
        // unsafe {
        //     self.backend.device.queue_submit(
        //         self.graphic_queue, &[submit_info],
        //         self.graphic_submit_fence
        //     );
        // };
    }

    fn update(&self, app_obj: &App, delta_timer: f64)
    {
    }

}
