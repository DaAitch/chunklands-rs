mod error;
mod vulkan;

use glfw::WindowEvent;

use error::{GameError, Result};
use log::debug;
use vulkan::{Vulkan, VulkanInit};

pub struct GameInit {
    pub debug: bool,
}

pub struct Game {
    debug: bool,
    glfw: glfw::Glfw,
    vulkan: Option<Vulkan>,
    window: glfw::Window,
    window_events: std::sync::mpsc::Receiver<(f64, WindowEvent)>,
}

impl Game {
    pub fn new(init: GameInit) -> Result<Self> {
        let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        glfw.window_hint(glfw::WindowHint::Visible(true));
        glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

        let (mut window, window_events) = glfw
            .create_window(640, 480, "Vulkan Rust", glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window.");

        assert!(glfw.vulkan_supported());
        let required_extensions = glfw.get_required_instance_extensions().unwrap();
        debug!("GLFW required vulkan extensions: {:?}", required_extensions);

        let vulkan = Vulkan::new(VulkanInit {
            debug: init.debug,
            window: &mut window,
            req_ext: &required_extensions,
            req_layers: &vec![],
        })
        .map_err(|e| GameError::VulkanError(format!("vulkan init failed: {}", e)))?;

        Ok(Self {
            debug: init.debug,
            glfw,
            vulkan: Some(vulkan),
            window,
            window_events,
        })
    }

    pub fn make_loop(&mut self) {
        let vulkan = self.vulkan.as_mut().unwrap();

        self.window.set_key_polling(true);
        self.window.set_framebuffer_size_polling(true);

        while !self.window.should_close() {
            self.glfw.poll_events();

            for (_, event) in glfw::flush_messages(&self.window_events) {
                match event {
                    glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _) => {
                        self.window.set_should_close(true);
                    }

                    glfw::WindowEvent::FramebufferSize(_, _) => {
                        vulkan.on_framebuffer_changed().unwrap();
                    }

                    _ => {}
                }
            }

            let start = self.glfw.get_time();
            vulkan.draw_frame(&self.window).unwrap();
            let end = self.glfw.get_time();

            debug!("diff: {}", end - start)
        }

        vulkan.wait_idle().unwrap();
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        self.vulkan.take().map(|vulkan| vulkan.destroy());
    }
}
