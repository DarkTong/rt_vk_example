use ash;
use ash::vk;
use ash::version::*;
#[derive(Clone, Debug)]
pub struct PipelineStateObjectDescriptor {
    pub vs_desc: ShaderProgramDescriptor,
    pub ps_desc: ShaderProgramDescriptor,
    pub attachment_desc: Vec<vk::AttachmentDescription>,
    pub input_binding_desc: Vec<vk::VertexInputBindingDescription>,
    pub input_attr_desc: Vec<vk::VertexInputAttributeDescription>,
    pub viewports: Vec<vk::Viewport>,
    pub scissors: Vec<vk::Rect2D>,
}

#[derive(Clone, Debug, Default)]
pub struct ShaderProgramDescriptor {
    pub path: String,
    pub entry: std::ffi::CString,
}

impl ::std::default::Default for PipelineStateObjectDescriptor {
    fn default() -> Self {
        PipelineStateObjectDescriptor {
            vs_desc: ShaderProgramDescriptor::default(),
            ps_desc: ShaderProgramDescriptor::default(),
            attachment_desc: vec![],
            input_attr_desc: vec![],
            input_binding_desc: vec![],
            viewports: vec![],
            scissors: vec![],
        }
    }
}

pub struct PipelineStateObject {
    pub pso_desc: PipelineStateObjectDescriptor,
    pub vs_mod: vk::ShaderModule,
    pub ps_mod: vk::ShaderModule,
    pub render_pass: vk::RenderPass,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub device: ash::Device,
}

impl Drop for PipelineStateObject {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_shader_module(self.vs_mod, None);
            self.device.destroy_shader_module(self.ps_mod, None);
            self.device.destroy_render_pass(self.render_pass, None);
        }
    }
}
